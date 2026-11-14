//! A dense tile's samples behind a residency that can change (#1217): held
//! in RAM, packed (LZ4 in RAM), spilled (a scratch extent), backed by the
//! file the document was opened from (#1218), or unreadable.
//!
//! Readers and writers **pin** a body before they touch its samples and the
//! first pin restores them; the evictor moves only an unpinned body, under
//! its lock. The handshake is SeqCst on both sides: a pinning thread
//! increments `pins` then loads `state`, the evictor stores `EVICTING` then
//! loads `pins`, so one of them always sees the other.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::{TileDecode, TileRect, TileSample};
use crate::memory::eviction::{self, Evictable, Packing, Residency};
use crate::scratch::{self, Extent, ScratchError};

const RESIDENT: u8 = 0;
const EVICTING: u8 = 1;
const AWAY: u8 = 2;

/// Packed payload tags: the first byte of a packed or spilled payload.
const TAG_LZ4: u8 = 0;
const TAG_RAW: u8 = 1;

enum Slot {
    /// The samples are at `Body::data`.
    Resident,
    /// The LZ4 payload, tagged.
    Packed(Box<[u8]>),
    /// The payload is in `Cell::extent`.
    Spilled,
    /// The samples are decoded from `Cell::source` on the next pin.
    Backed,
}

/// Where a backed tile's samples are in the source file: the channel's
/// decoder and the tile's rectangle in that channel.
pub(crate) struct Source<T> {
    pub(crate) decode: TileDecode<T>,
    pub(crate) rect: TileRect,
}

impl<T> Clone for Source<T> {
    fn clone(&self) -> Self {
        Self {
            decode: Arc::clone(&self.decode),
            rect: self.rect,
        }
    }
}

struct Cell<T> {
    slot: Slot,
    /// The samples' copy on disk while it matches them: kept across a
    /// restore so evicting a clean tile again writes nothing (R3).
    extent: Option<Extent>,
    /// The samples' copy in the source file while it matches them (#1218):
    /// kept across a restore, as `extent` is, and cleared by the first write.
    source: Option<Source<T>>,
}

pub(crate) struct Body<T> {
    len: usize,
    state: AtomicU8,
    pins: AtomicU32,
    referenced: AtomicBool,
    hot: AtomicU64,
    history: AtomicU32,
    unreadable: AtomicBool,
    /// `cell.source` is set, readable without the lock by the evictor.
    backed: AtomicBool,
    cell: Mutex<Cell<T>>,
    /// The resident samples (a leaked `Box<[T]>` of `len`), or null while
    /// away. Read without the lock only by a pinned thread that saw
    /// `RESIDENT`.
    data: AtomicPtr<T>,
}

// SAFETY: `data` is owned by the body and only touched under the pin/lock
// protocol above; `T` is `Send + Sync`.
unsafe impl<T: Send + Sync> Send for Body<T> {}
// SAFETY: as above.
unsafe impl<T: Send + Sync> Sync for Body<T> {}

impl<T: TileSample> Body<T> {
    /// A resident body owning `samples`, registered with the evictor.
    pub(crate) fn new(samples: Box<[T]>) -> Arc<Self> {
        let bytes = std::mem::size_of_val(&*samples) as u64;
        eviction::on_allocate(bytes);
        let len = samples.len();
        let body = Arc::new(Self {
            len,
            state: AtomicU8::new(RESIDENT),
            pins: AtomicU32::new(0),
            referenced: AtomicBool::new(false),
            hot: AtomicU64::new(0),
            history: AtomicU32::new(0),
            unreadable: AtomicBool::new(false),
            backed: AtomicBool::new(false),
            cell: Mutex::new(Cell {
                slot: Slot::Resident,
                extent: None,
                source: None,
            }),
            data: AtomicPtr::new(Box::into_raw(samples).cast::<T>()),
        });
        eviction::register(Arc::downgrade(&body) as std::sync::Weak<dyn Evictable>);
        body
    }

    /// An away body of `len` samples that `source` decodes on its first pin.
    pub(crate) fn backed(len: usize, source: Source<T>) -> Arc<Self> {
        let body = Arc::new(Self {
            len,
            state: AtomicU8::new(AWAY),
            pins: AtomicU32::new(0),
            referenced: AtomicBool::new(false),
            hot: AtomicU64::new(0),
            history: AtomicU32::new(0),
            unreadable: AtomicBool::new(false),
            backed: AtomicBool::new(true),
            cell: Mutex::new(Cell {
                slot: Slot::Backed,
                extent: None,
                source: Some(source),
            }),
            data: AtomicPtr::new(std::ptr::null_mut()),
        });
        eviction::count_backed(1);
        eviction::register(Arc::downgrade(&body) as std::sync::Weak<dyn Evictable>);
        body
    }

    /// `n` copies of `v`.
    pub(crate) fn filled(n: usize, v: T) -> Arc<Self> {
        Self::new(vec![v; n].into_boxed_slice())
    }

    /// Hold the samples resident until the pin drops, restoring them first.
    /// Under [`eviction::transient`], an away backed body is decoded into
    /// the pin alone and stays away.
    pub(crate) fn pin(&self) -> Pin<'_, T> {
        if eviction::is_transient() {
            if let Some(owned) = self.peek_backed() {
                return Pin {
                    body: self,
                    owned: Some(owned),
                };
            }
        }
        self.pin_raw();
        Pin {
            body: self,
            owned: None,
        }
    }

    /// The sample at tile-local `(dx, dy)` of an away backed body, decoded
    /// alone without restoring the body: a sparse reader (a thumbnail) pays
    /// for the rows it samples, not for whole tiles. `None` when the body is
    /// not away in its source or the decode fails.
    pub(crate) fn peek_sample(&self, dx: usize, dy: usize) -> Option<T> {
        if self.state.load(Ordering::SeqCst) == RESIDENT {
            return None;
        }
        let source = {
            let cell = self.lock();
            if !matches!(cell.slot, Slot::Backed) {
                return None;
            }
            cell.source.clone()?
        };
        let rect = TileRect {
            x: source.rect.x + dx,
            y: source.rect.y + dy,
            w: 1,
            h: 1,
        };
        let mut sample = [T::default()];
        (source.decode)(rect, &mut sample).ok()?;
        Some(sample[0])
    }

    /// The samples of an away backed body, decoded without installing them;
    /// `None` when the body is not away in its source or the decode fails
    /// (the caller's ordinary pin then records the loss).
    fn peek_backed(&self) -> Option<Box<[T]>> {
        if self.state.load(Ordering::SeqCst) == RESIDENT {
            return None;
        }
        let source = {
            let cell = self.lock();
            if !matches!(cell.slot, Slot::Backed) {
                return None;
            }
            cell.source.clone()?
        };
        let mut samples = vec![T::default(); self.len].into_boxed_slice();
        (source.decode)(source.rect, &mut samples).ok()?;
        Some(samples)
    }

    /// Increment the pin count and make the samples resident; the caller
    /// must [`Self::unpin`] once.
    pub(crate) fn pin_raw(&self) {
        self.pins.fetch_add(1, Ordering::SeqCst);
        if self.state.load(Ordering::SeqCst) != RESIDENT {
            self.restore();
        }
        if !self.referenced.load(Ordering::Relaxed) {
            self.referenced.store(true, Ordering::Relaxed);
        }
        if let Some(epoch) = eviction::hot_epoch() {
            self.hot.store(epoch, Ordering::Relaxed);
        }
        if self.unreadable.load(Ordering::Relaxed) {
            eviction::count_unreadable_read();
        }
    }

    /// Bring the samples back. Called by a pinning thread that saw them away.
    fn restore(&self) {
        if self.backed.load(Ordering::Relaxed) {
            // A decode from the source is a fresh allocation: let a burst of
            // them evict first, as any new body does. Before the lock, so a
            // pass this starts can claim other bodies freely.
            eviction::on_allocate(self.bytes());
        }
        let mut cell = self.lock();
        if self.state.load(Ordering::SeqCst) == RESIDENT {
            return;
        }
        let _io = scratch::io_timer();
        let samples = match &cell.slot {
            Slot::Resident => unreachable!("away state with a resident slot"),
            Slot::Backed => {
                let source = cell.source.as_ref().expect("a backed body has a source");
                let mut samples = vec![T::default(); self.len].into_boxed_slice();
                (source.decode)(source.rect, &mut samples)
                    .map(|()| samples)
                    .map_err(|()| ScratchError::Unreadable)
            }
            Slot::Packed(payload) => unpack::<T>(payload, self.len),
            Slot::Spilled => {
                let extent = cell.extent.expect("a spilled body has an extent");
                let mut payload = Vec::new();
                scratch::read(&extent, &mut payload)
                    .map_err(|_| ScratchError::Unreadable)
                    .and_then(|()| unpack::<T>(&payload, self.len))
            }
        };
        let samples = samples.unwrap_or_else(|_| {
            // A lost tile reads as the default; every read of it counts so
            // the edit that needed it is refused (design §5).
            if let Some(extent) = cell.extent.take() {
                scratch::free(extent);
            }
            cell.source = None;
            self.backed.store(false, Ordering::Relaxed);
            self.unreadable.store(true, Ordering::Relaxed);
            eviction::count_unreadable();
            vec![T::default(); self.len].into_boxed_slice()
        });
        match cell.slot {
            Slot::Packed(_) => eviction::count_packed(-1, -(packed_len(&cell.slot) as i64)),
            Slot::Spilled => eviction::count_spilled(-1),
            Slot::Backed => eviction::count_backed(-1),
            Slot::Resident => {}
        }
        cell.slot = Slot::Resident;
        self.data
            .store(Box::into_raw(samples).cast::<T>(), Ordering::Release);
        self.state.store(RESIDENT, Ordering::SeqCst);
    }

    /// Claim the body for eviction: locked, resident, readable and unpinned.
    /// On `Some`, the state is `EVICTING` and the caller must leave it
    /// resident or away before unlocking.
    fn claim(&self) -> Option<MutexGuard<'_, Cell<T>>> {
        if self.pins.load(Ordering::Relaxed) > 0 || self.unreadable.load(Ordering::Relaxed) {
            return None;
        }
        let cell = self.cell.try_lock().ok()?;
        if self.state.load(Ordering::SeqCst) != RESIDENT {
            return None;
        }
        self.state.store(EVICTING, Ordering::SeqCst);
        if self.pins.load(Ordering::SeqCst) > 0 {
            self.state.store(RESIDENT, Ordering::SeqCst);
            return None;
        }
        Some(cell)
    }

    /// Free the resident samples. The body is claimed.
    fn drop_samples(&self) {
        let data = self.data.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !data.is_null() {
            // SAFETY: `data` was a `Box<[T]>` of `len` leaked by this body,
            // and the claim excludes every other user.
            drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, self.len)) });
        }
    }

    fn bytes(&self) -> u64 {
        (self.len * std::mem::size_of::<T>()) as u64
    }
}

impl<T> Body<T> {
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// The body's identity for the census: stable while it lives.
    pub(crate) fn address(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    fn lock(&self) -> MutexGuard<'_, Cell<T>> {
        self.cell.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn unpin(&self) {
        self.pins.fetch_sub(1, Ordering::Release);
    }

    /// The samples. The caller holds a pin.
    fn samples(&self) -> &[T] {
        let data = self.data.load(Ordering::Acquire);
        // SAFETY: pinned and resident, so `data` is the live allocation of
        // `len` samples and the evictor will not move it.
        unsafe { std::slice::from_raw_parts(data, self.len) }
    }

    /// The samples, writable. The caller holds a pin and the body's only
    /// strong reference from a plane (`&mut` to that plane).
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn samples_mut(&self) -> &mut [T] {
        let data = self.data.load(Ordering::Acquire);
        // SAFETY: the caller's contract: pinned, resident, and unshared.
        unsafe { std::slice::from_raw_parts_mut(data, self.len) }
    }

    /// The samples will change: neither the copy on disk nor the source
    /// matches them any more.
    pub(crate) fn dirty(&self) {
        let mut cell = self.lock();
        if let Some(extent) = cell.extent.take() {
            scratch::free(extent);
        }
        cell.source = None;
        self.backed.store(false, Ordering::Relaxed);
    }

    pub(crate) fn is_unreadable(&self) -> bool {
        self.unreadable.load(Ordering::Relaxed)
    }

    pub(crate) fn hold_history(&self) {
        self.history.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn release_history(&self) {
        self.history.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A pin on a body, released on drop; or, for a transient read of a
/// backed body, the samples it decoded.
pub(crate) struct Pin<'a, T> {
    body: &'a Body<T>,
    owned: Option<Box<[T]>>,
}

impl<T> std::ops::Deref for Pin<'_, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match &self.owned {
            Some(owned) => owned,
            None => self.body.samples(),
        }
    }
}

impl<T> Drop for Pin<'_, T> {
    fn drop(&mut self) {
        if self.owned.is_none() {
            self.body.unpin();
        }
    }
}

impl<T> Drop for Body<T> {
    fn drop(&mut self) {
        let data = *self.data.get_mut();
        if !data.is_null() {
            // SAFETY: the last reference: nobody else can hold the samples.
            drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, self.len)) });
        }
        let cell = self.cell.get_mut().unwrap_or_else(|e| e.into_inner());
        match &cell.slot {
            Slot::Resident => {}
            Slot::Packed(p) => eviction::count_packed(-1, -(p.len() as i64)),
            Slot::Spilled => eviction::count_spilled(-1),
            Slot::Backed => eviction::count_backed(-1),
        }
        if *self.unreadable.get_mut() {
            eviction::count_unreadable_dropped();
        }
        if let Some(extent) = cell.extent.take() {
            scratch::free(extent);
        }
    }
}

fn packed_len(slot: &Slot) -> usize {
    match slot {
        Slot::Packed(p) => p.len(),
        _ => 0,
    }
}

/// The samples as a tagged payload: LZ4 when that saves an eighth, else raw.
fn pack<T: TileSample>(samples: &[T]) -> (Vec<u8>, bool) {
    let raw = T::bytes(samples);
    let mut out = Vec::with_capacity(raw.len() / 2 + 16);
    out.push(TAG_LZ4);
    let packed = lz4_flex::block::compress(raw);
    if packed.len() < raw.len() - raw.len() / 8 {
        out.extend_from_slice(&packed);
        (out, true)
    } else {
        out[0] = TAG_RAW;
        out.extend_from_slice(raw);
        (out, false)
    }
}

fn unpack<T: TileSample>(payload: &[u8], len: usize) -> Result<Box<[T]>, ScratchError> {
    let mut samples = vec![T::default(); len].into_boxed_slice();
    let out = T::bytes_mut(&mut samples);
    match payload.split_first() {
        Some((&TAG_LZ4, body)) => {
            let n = lz4_flex::block::decompress_into(body, out)
                .map_err(|_| ScratchError::Unreadable)?;
            if n != out.len() {
                return Err(ScratchError::Unreadable);
            }
        }
        Some((&TAG_RAW, body)) if body.len() == out.len() => out.copy_from_slice(body),
        _ => return Err(ScratchError::Unreadable),
    }
    Ok(samples)
}

impl<T: TileSample> Evictable for Body<T> {
    fn pinned(&self) -> bool {
        self.pins.load(Ordering::Relaxed) > 0
    }

    fn take_referenced(&self) -> bool {
        self.referenced.swap(false, Ordering::Relaxed)
    }

    fn hot_stamp(&self) -> u64 {
        self.hot.load(Ordering::Relaxed)
    }

    fn history_held(&self) -> bool {
        self.history.load(Ordering::Relaxed) > 0
    }

    fn backed(&self) -> bool {
        self.backed.load(Ordering::Relaxed)
    }

    fn residency(&self) -> Residency {
        if self.is_unreadable() {
            return Residency::Unreadable;
        }
        if self.state.load(Ordering::SeqCst) == RESIDENT {
            return Residency::Resident;
        }
        match self.lock().slot {
            Slot::Resident => Residency::Resident,
            Slot::Packed(_) => Residency::Packed,
            Slot::Spilled => Residency::Spilled,
            Slot::Backed => Residency::Backed,
        }
    }

    fn pack(&self, may_spill: bool) -> Packing {
        let Some(mut cell) = self.claim() else {
            return Packing::Skipped;
        };
        if cell.source.is_some() {
            // Unedited since the open: the source decodes it again (#1218).
            self.drop_samples();
            cell.slot = Slot::Backed;
            self.state.store(AWAY, Ordering::SeqCst);
            eviction::count_backed(1);
            return Packing::Freed {
                bytes: self.bytes(),
                written: 0,
            };
        }
        if cell.extent.is_some() {
            // Clean: the copy on disk still matches (R3).
            self.drop_samples();
            cell.slot = Slot::Spilled;
            self.state.store(AWAY, Ordering::SeqCst);
            eviction::count_spilled(1);
            return Packing::Freed {
                bytes: self.bytes(),
                written: 0,
            };
        }
        let (payload, compressed) = pack(self.samples());
        if compressed {
            let len = payload.len();
            self.drop_samples();
            cell.slot = Slot::Packed(payload.into_boxed_slice());
            self.state.store(AWAY, Ordering::SeqCst);
            eviction::count_packed(1, len as i64);
            return Packing::Freed {
                bytes: self.bytes().saturating_sub(len as u64),
                written: 0,
            };
        }
        if !may_spill {
            self.state.store(RESIDENT, Ordering::SeqCst);
            return Packing::Skipped;
        }
        match scratch::write(&payload) {
            Ok(extent) => {
                self.drop_samples();
                cell.slot = Slot::Spilled;
                cell.extent = Some(extent);
                self.state.store(AWAY, Ordering::SeqCst);
                eviction::count_spilled(1);
                Packing::Freed {
                    bytes: self.bytes(),
                    written: payload.len() as u64,
                }
            }
            Err(e) => {
                self.state.store(RESIDENT, Ordering::SeqCst);
                Packing::Failed(e)
            }
        }
    }

    fn spill(&self) -> Packing {
        let Ok(mut cell) = self.cell.try_lock() else {
            return Packing::Skipped;
        };
        let Slot::Packed(payload) = &cell.slot else {
            return Packing::Skipped;
        };
        match scratch::write(payload) {
            Ok(extent) => {
                let len = payload.len();
                cell.slot = Slot::Spilled;
                cell.extent = Some(extent);
                eviction::count_packed(-1, -(len as i64));
                eviction::count_spilled(1);
                Packing::Freed {
                    bytes: len as u64,
                    written: len as u64,
                }
            }
            Err(e) => Packing::Failed(e),
        }
    }
}
