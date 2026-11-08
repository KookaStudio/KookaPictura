//! Refcounted copy-on-write pixel planes.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::tiled_plane::{TileSample, TiledPlane};

mod access;
pub use access::assert_no_materialization;

enum Store<T> {
    /// One contiguous allocation: a plane that was never tiled.
    Flat(Arc<[T]>),
    /// Square tiles per channel (#1213).
    Tiled(TiledPlane<T>),
}

impl<T: Clone> Clone for Store<T> {
    fn clone(&self) -> Self {
        match self {
            Store::Flat(data) => Store::Flat(Arc::clone(data)),
            Store::Tiled(tiles) => Store::Tiled(tiles.clone()),
        }
    }
}

/// One image plane behind a refcount.
///
/// Cloning the owning document is a refcount bump, and a write forks only what
/// it touches, so a stroke or a history snapshot stops paying for pixels that
/// have not changed.
///
/// A plane starts **flat**: one contiguous allocation that a uniquely held
/// plane mutates in place exactly as a `Vec` does, and a shared one copies on
/// first write. A document plane is **tiled** ([`Plane::tile`],
/// [`Plane::filled`]): each channel is cut into 256² tiles that are `Empty`,
/// `Uniform` or a shared dense allocation, a write forks only the tiles it
/// touches, and a settle folds tiles that hold one value back to `Uniform`.
/// A contiguous view of a tiled plane ([`Plane::as_slice`]) is built on demand
/// and counted ([`materializations`]); hot paths use the streaming access in
/// `access.rs` instead.
///
/// A plane also carries a **stamp**: two planes with the same nonzero stamp
/// hold the same bytes. Every mutable access clears it before a byte can
/// change and a clone keeps it, so the history can tell an untouched plane from
/// a written one without comparing them. Only [`Plane::set_stamp`] sets one.
///
/// An **ident** ([`Plane::ident`]) is the same promise made lazily through a
/// shared reference, for caches derived from the bytes: it is assigned on the
/// first query, kept by a clone, and cleared by every mutable access.
pub struct Plane<T = u8> {
    store: Store<T>,
    mirror: OnceLock<Arc<[T]>>,
    stamp: u64,
    ident: AtomicU64,
}

impl<T> Plane<T> {
    fn wrap(data: Arc<[T]>) -> Self {
        Self::with_store(Store::Flat(data))
    }

    fn with_store(store: Store<T>) -> Self {
        Self {
            store,
            mirror: OnceLock::new(),
            stamp: 0,
            ident: AtomicU64::new(0),
        }
    }

    /// Forget the stamp, the ident and any contiguous view: called by every
    /// mutable access before a byte can change.
    fn touch(&mut self) {
        self.stamp = 0;
        *self.ident.get_mut() = 0;
        self.mirror = OnceLock::new();
    }

    pub(crate) fn tiled(&self) -> Option<&TiledPlane<T>> {
        match &self.store {
            Store::Tiled(t) => Some(t),
            Store::Flat(_) => None,
        }
    }

    pub(crate) fn flat(&self) -> Option<&[T]> {
        match &self.store {
            Store::Flat(d) => Some(d),
            Store::Tiled(_) => None,
        }
    }
}

impl<T: Clone> Clone for Plane<T> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            mirror: OnceLock::new(),
            stamp: self.stamp,
            ident: AtomicU64::new(self.ident.load(Ordering::Relaxed)),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Plane<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.store {
            Store::Flat(data) => f.debug_tuple("Plane").field(data).finish(),
            Store::Tiled(t) => f
                .debug_struct("Plane")
                .field("width", &t.width)
                .field("height", &t.height)
                .field("channels", &t.channels)
                .finish(),
        }
    }
}

/// Samples compared per chunk when a tiled plane is compared by value.
const COMPARE_CHUNK: usize = 1 << 16;

/// Logical equality read in chunks, so neither side is materialized.
fn eq_slice<T: TileSample>(plane: &Plane<T>, other: &[T]) -> bool {
    if plane.len() != other.len() {
        return false;
    }
    if let Some(flat) = plane.flat() {
        return flat == other;
    }
    let mut buf = vec![T::default(); COMPARE_CHUNK.min(other.len())];
    other.chunks(COMPARE_CHUNK).enumerate().all(|(k, chunk)| {
        let out = &mut buf[..chunk.len()];
        plane.read_into(k * COMPARE_CHUNK, out);
        out == chunk
    })
}

impl<T: TileSample> PartialEq for Plane<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        if self.shares(other) {
            return true;
        }
        if let Some(flat) = other.flat() {
            return eq_slice(self, flat);
        }
        if let Some(flat) = self.flat() {
            return eq_slice(other, flat);
        }
        let n = COMPARE_CHUNK.min(self.len());
        let (mut a, mut b) = (vec![T::default(); n], vec![T::default(); n]);
        (0..self.len()).step_by(COMPARE_CHUNK.max(1)).all(|start| {
            let m = n.min(self.len() - start);
            self.read_into(start, &mut a[..m]);
            other.read_into(start, &mut b[..m]);
            a[..m] == b[..m]
        })
    }
}

impl<T: TileSample + Eq> Eq for Plane<T> {}

impl<T: TileSample> Deref for Plane<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: TileSample> DerefMut for Plane<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Default for Plane<T> {
    fn default() -> Self {
        Self::wrap(Arc::from(Vec::new()))
    }
}

impl<T> From<Vec<T>> for Plane<T> {
    fn from(data: Vec<T>) -> Self {
        Self::wrap(Arc::from(data))
    }
}

/// A stamp no plane has carried before.
pub fn fresh_stamp() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

static MATERIALIZATIONS: AtomicU64 = AtomicU64::new(0);

/// How many times a tiled plane has been materialized into one contiguous
/// allocation.
pub fn materializations() -> u64 {
    MATERIALIZATIONS.load(Ordering::Relaxed)
}

/// Reset the materialization counter (tests and the profile).
pub fn reset_materializations() {
    MATERIALIZATIONS.store(0, Ordering::Relaxed);
}

fn materialize<T: TileSample>(tiles: &TiledPlane<T>) -> Arc<[T]> {
    MATERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
    Arc::from(tiles.materialize())
}

impl<T> Plane<T> {
    /// The logical length; never materializes a tiled plane.
    pub fn len(&self) -> usize {
        match &self.store {
            Store::Flat(d) => d.len(),
            Store::Tiled(t) => t.width * t.height * t.channels,
        }
    }

    /// Whether the plane holds no samples.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether a contiguous copy of a tiled plane is cached: the cost
    /// [`Self::as_slice`] leaves behind until the next write.
    pub fn is_mirrored(&self) -> bool {
        self.mirror.get().is_some()
    }

    /// The plane's stamp; `0` when it has none.
    pub fn stamp(&self) -> u64 {
        self.stamp
    }

    /// A nonzero id no plane with different bytes has carried, assigned on
    /// first query. Two planes with the same ident hold the same bytes.
    pub fn ident(&self) -> u64 {
        let id = self.ident.load(Ordering::Relaxed);
        if id != 0 {
            return id;
        }
        let fresh = fresh_stamp();
        match self
            .ident
            .compare_exchange(0, fresh, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => fresh,
            Err(won) => won,
        }
    }

    /// Mark the plane's bytes with `stamp`. The caller vouches that every plane
    /// carrying the same stamp holds the same bytes.
    pub fn set_stamp(&mut self, stamp: u64) {
        self.stamp = stamp;
    }

    /// Whether the plane is tiled.
    pub fn is_tiled(&self) -> bool {
        matches!(self.store, Store::Tiled(_))
    }

    /// A tiled plane's `(width, height, channels)`; `None` while flat.
    pub fn geometry(&self) -> Option<(usize, usize, usize)> {
        self.tiled().map(|t| (t.width, t.height, t.channels))
    }
}

impl Plane<u8> {
    /// A uniquely held flat plane of `len` bytes that `fill` writes in place.
    ///
    /// `Plane::from(Vec)` copies the vector into the refcounted allocation, and
    /// zero-filling a fresh one is a single-threaded pass; this takes zeroed
    /// pages straight from the allocator, so a caller filling a large plane in
    /// parallel pays for its own pass only.
    pub fn build(len: usize, fill: impl FnOnce(&mut [u8])) -> Self {
        let zeroed: Arc<[std::mem::MaybeUninit<u8>]> = Arc::new_zeroed_slice(len);
        // SAFETY: the allocation is zeroed, and every bit pattern, zero
        // included, is a valid `u8`.
        let mut data: Arc<[u8]> = unsafe { zeroed.assume_init() };
        fill(Arc::get_mut(&mut data).expect("a fresh allocation is unique"));
        Self::wrap(data)
    }
}

impl<T: TileSample> Plane<T> {
    /// A tiled plane of `channels` stacked `width × height` channels whose
    /// every sample is `value`. It holds no tile allocation.
    pub fn filled(width: usize, height: usize, channels: usize, value: T) -> Self {
        Self::with_store(Store::Tiled(TiledPlane::filled(
            width, height, channels, value,
        )))
    }

    /// A tiled plane whose global rows `rows(y, row)` fills, one strip of
    /// 256 rows at a time, so the whole plane is never held contiguously.
    /// Tiles that come out uniform hold no allocation.
    pub fn build_tiled<E>(
        width: usize,
        height: usize,
        channels: usize,
        rows: impl FnMut(usize, &mut [T]) -> Result<(), E>,
    ) -> Result<Self, E> {
        if width == 0 || height == 0 || channels == 0 {
            return Ok(Self::default());
        }
        TiledPlane::build(width, height, channels, rows).map(|t| Self::with_store(Store::Tiled(t)))
    }

    /// Tile the plane as stacked `width × height` channels.
    ///
    /// A flat plane is copied into tiles once (uniform tiles hold nothing). A
    /// plane already tiled at this geometry is left as it is; one tiled at
    /// another geometry is materialized and re-tiled. Returns `false`, leaving
    /// the plane flat, when its length is not a whole number of channels.
    pub fn tile(&mut self, width: usize, height: usize) -> bool {
        let len = self.len();
        if width == 0 || height == 0 || len == 0 || !len.is_multiple_of(width * height) {
            return false;
        }
        if self
            .geometry()
            .is_some_and(|(w, h, _)| (w, h) == (width, height))
        {
            return true;
        }
        let tiles = match &self.store {
            Store::Flat(data) => TiledPlane::from_flat(data, width, height),
            Store::Tiled(t) => TiledPlane::from_flat(&materialize(t), width, height),
        };
        self.store = Store::Tiled(tiles);
        self.mirror = OnceLock::new();
        true
    }

    /// The plane's logical samples as one contiguous slice.
    ///
    /// Free on a flat plane. A tiled plane is materialized once, counted, and
    /// cached until the next write: only `ponytail:` paths may ask for it.
    pub fn as_slice(&self) -> &[T] {
        match &self.store {
            Store::Flat(data) => data,
            Store::Tiled(t) => self.mirror.get_or_init(|| materialize(t)),
        }
    }

    /// The plane's samples, mutable. A shared flat plane forks first; a tiled
    /// plane is materialized (counted) and becomes flat.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let mirror = self.mirror.take();
        self.touch();
        if let Store::Tiled(t) = &self.store {
            let data = mirror.unwrap_or_else(|| materialize(t));
            self.store = Store::Flat(data);
        }
        match &mut self.store {
            Store::Flat(data) => Arc::make_mut(data),
            Store::Tiled(_) => unreachable!("the plane was just flattened"),
        }
    }

    /// The sample at logical index `i`, read in place; `None` past the end.
    pub fn sample(&self, i: usize) -> Option<T> {
        match &self.store {
            Store::Flat(d) => d.get(i).copied(),
            Store::Tiled(t) => (i < self.len()).then(|| t.sample(i % t.width, i / t.width)),
        }
    }

    /// Whether `self` and `other` are the same bytes by construction: one
    /// flat allocation, or the same tiles at every index.
    pub fn shares(&self, other: &Self) -> bool {
        match (&self.store, &other.store) {
            (Store::Flat(a), Store::Flat(b)) => Arc::ptr_eq(a, b),
            (Store::Tiled(a), Store::Tiled(b)) => {
                a.same_geometry(b) && (0..a.tile_count()).all(|i| a.shares_tile(i, b))
            }
            _ => false,
        }
    }

    /// Each allocation the plane holds as `(address, bytes)`: the flat
    /// allocation, or every dense tile.
    pub fn allocations(&self) -> Vec<(usize, usize)> {
        let size = std::mem::size_of::<T>();
        match &self.store {
            Store::Flat(d) => vec![(d.as_ptr() as usize, d.len() * size)],
            Store::Tiled(t) => t.allocations().map(|(p, n)| (p, n * size)).collect(),
        }
    }

    /// Bytes held by this plane's own allocations, without the cached
    /// contiguous view.
    pub fn allocated_bytes(&self) -> usize {
        self.allocations().iter().map(|(_, n)| n).sum()
    }

    /// Append `more`, mirroring `Vec::extend_from_slice`; the plane becomes
    /// flat.
    ///
    /// ponytail: a slice behind a refcount cannot grow in place, so this
    /// rebuilds; the callers build small buffers.
    pub fn extend_from_slice(&mut self, more: &[T]) {
        let mut data = self.as_slice().to_vec();
        data.extend_from_slice(more);
        *self = Self::wrap(Arc::from(data));
    }

    /// Extend from an iterator, mirroring `Vec::extend`.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut data = self.as_slice().to_vec();
        data.extend(iter);
        *self = Self::wrap(Arc::from(data));
    }

    /// Drop every sample, mirroring `Vec::clear`.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Shorten the plane to `len`, mirroring `Vec::truncate`.
    ///
    /// ponytail: a slice behind a refcount cannot shrink in place, so this
    /// rebuilds; the callers are the tests that build malformed buffers.
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            let mut data = self.as_slice().to_vec();
            data.truncate(len);
            *self = Self::wrap(Arc::from(data));
        }
    }
}

impl<T: TileSample> AsRef<[T]> for Plane<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<'a, T: TileSample> IntoIterator for &'a Plane<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<T: TileSample> PartialEq<Vec<T>> for Plane<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        eq_slice(self, other.as_slice())
    }
}

impl<T: TileSample> PartialEq<[T]> for Plane<T> {
    fn eq(&self, other: &[T]) -> bool {
        eq_slice(self, other)
    }
}

impl<T: TileSample, const N: usize> PartialEq<[T; N]> for Plane<T> {
    fn eq(&self, other: &[T; N]) -> bool {
        eq_slice(self, other)
    }
}

impl<T: TileSample> PartialEq<&[T]> for Plane<T> {
    fn eq(&self, other: &&[T]) -> bool {
        eq_slice(self, other)
    }
}

impl<T> std::iter::FromIterator<T> for Plane<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::wrap(Arc::from(iter.into_iter().collect::<Vec<T>>()))
    }
}

#[cfg(test)]
pub(crate) fn shares(a: &Plane<u8>, b: &Plane<u8>) -> bool {
    a.shares(b)
}

#[cfg(test)]
mod tests;
