//! Lazy, file-backed PSD open (#1218). An 8-bit raw or RLE layer channel is
//! indexed at open (where each row's bytes are) and its tiles are backed by
//! the file: a tile decodes its 256-row band when first read, and an
//! unedited tile is dropped and decoded again instead of packed or spilled.
//!
//! The file stays open behind a [`SourcePin`] and is read with `pread`, not a
//! map, so a file cut short under the app fails a decode instead of raising
//! `SIGBUS`. A rename over the path or an unlink leaves the descriptor reading
//! the data it was opened with, which is how Pictura's own saves land
//! (`write_atomically`). A file rewritten in place is detected by its size or
//! modification time, and every later decode fails: the tiles not yet read
//! become unreadable, which edits and saves refuse (#1217).

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use pictura_core::{PixelBuffer, Plane, PsdRect, TileDecode, TileRect, TILE};

use crate::common::{Reader, COMPRESSION_RAW, COMPRESSION_RLE};
use crate::error::PsdError;
use crate::probe::ImageBudget;
use crate::read::{decode_packbits, read_rle_count};
use crate::MappedFile;
use crate::OpenOptions;
use pictura_core::Document;

/// An open source file, its identity at open, and what has been read from it.
pub struct SourcePin {
    file: File,
    path: PathBuf,
    len: u64,
    mtime: Option<SystemTime>,
    /// Set once the file is seen rewritten in place; never cleared.
    rewritten: AtomicBool,
    bytes_read: AtomicU64,
    bands_decoded: AtomicU64,
}

impl std::fmt::Debug for SourcePin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourcePin")
            .field("path", &self.path)
            .field("len", &self.len)
            .finish()
    }
}

impl SourcePin {
    /// Open `path` and record its size and modification time. Fails where
    /// positioned reads are unavailable (off Unix): the caller opens eagerly.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Arc<Self>> {
        if !cfg!(unix) {
            return Err(std::io::Error::other("lazy opens need positioned reads"));
        }
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let meta = file.metadata()?;
        Ok(Arc::new(Self {
            file,
            path,
            len: meta.len(),
            mtime: meta.modified().ok(),
            rewritten: AtomicBool::new(false),
            bytes_read: AtomicU64::new(0),
            bands_decoded: AtomicU64::new(0),
        }))
    }

    /// The file mapped read-only, for the open's parse.
    pub fn map(&self) -> std::io::Result<MappedFile> {
        #[cfg(unix)]
        return MappedFile::of_file(&self.file);
        #[cfg(not(unix))]
        return MappedFile::open(&self.path);
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the file behind the descriptor changed size or modification
    /// time since the open: it was rewritten in place.
    pub fn rewritten(&self) -> bool {
        if self.rewritten.load(Ordering::Relaxed) {
            return true;
        }
        let changed = match self.file.metadata() {
            Ok(meta) => meta.len() != self.len || meta.modified().ok() != self.mtime,
            Err(_) => true,
        };
        if changed {
            self.rewritten.store(true, Ordering::Relaxed);
        }
        changed
    }

    /// Bytes read from the file by decodes since the open.
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    /// Bands decoded from the file since the open (cache misses).
    pub fn bands_decoded(&self) -> u64 {
        self.bands_decoded.load(Ordering::Relaxed)
    }

    /// Fill `buf` from `offset`; fails on a rewritten file or a short read.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ()> {
        if self.rewritten() {
            return Err(());
        }
        let end = offset.checked_add(buf.len() as u64).ok_or(())?;
        if end > self.len {
            return Err(());
        }
        read_exact_at(&self.file, buf, offset).map_err(|_| ())?;
        self.bytes_read
            .fetch_add(buf.len() as u64, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

// Never reached off Unix: `SourcePin::open` refuses there, so no tile is
// backed by a file.
#[cfg(not(unix))]
fn read_exact_at(_: &File, _: &mut [u8], _: u64) -> std::io::Result<()> {
    Err(std::io::Error::other("positioned reads need Unix"))
}

/// Where a channel's rows are.
enum Rows {
    /// `width` bytes per row from this file offset.
    Raw(u64),
    /// Row `y`'s PackBits bytes are `starts[y]..starts[y + 1]` in the file.
    Rle(Box<[u64]>),
}

/// One 8-bit channel's rows in a pinned file.
pub(crate) struct ChannelIndex {
    pin: Arc<SourcePin>,
    /// The band cache's key for this channel.
    key: u64,
    width: usize,
    height: usize,
    rows: Rows,
    /// The last row decoded alone, and its number.
    last_row: Mutex<Option<(usize, Arc<[u8]>)>>,
}

/// A decode of at most this many rows of a band not cached reads those rows
/// alone instead of the band.
const SPARSE_ROWS: usize = 4;

static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

impl ChannelIndex {
    fn new(pin: &Arc<SourcePin>, width: usize, height: usize, rows: Rows) -> Self {
        Self {
            pin: Arc::clone(pin),
            key: NEXT_KEY.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rows,
            last_row: Mutex::new(None),
        }
    }

    /// A raw channel of `width × height` bytes at `at` in `file`, of which
    /// `available` bytes belong to it.
    pub(crate) fn raw(
        pin: &Arc<SourcePin>,
        at: usize,
        available: usize,
        width: usize,
        height: usize,
    ) -> Result<Self, PsdError> {
        let need = width
            .checked_mul(height)
            .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
        if available < need {
            return Err(PsdError::Invalid("raw channel too short".into()));
        }
        Ok(Self::new(pin, width, height, Rows::Raw(at as u64)))
    }

    /// An RLE channel whose `height` row counts are read by `counts` and
    /// whose rows start at `data_at`, all within `end` (exclusive, in the
    /// file): the rows' total must fit, as an eager decode's `take` checks.
    fn rle(
        pin: &Arc<SourcePin>,
        counts: &mut Reader,
        is_psb: bool,
        data_at: usize,
        end: usize,
        width: usize,
        height: usize,
    ) -> Result<Self, PsdError> {
        let mut starts = Vec::with_capacity(height + 1);
        let mut at = data_at;
        starts.push(at as u64);
        for _ in 0..height {
            at = at
                .checked_add(read_rle_count(counts, is_psb)?)
                .ok_or(PsdError::Truncated)?;
            starts.push(at as u64);
        }
        if at > end {
            return Err(PsdError::Truncated);
        }
        Ok(Self::new(
            pin,
            width,
            height,
            Rows::Rle(starts.into_boxed_slice()),
        ))
    }

    /// A layer channel's payload (after its compression word) at
    /// `file[at..at + len]`, compressed raw or RLE.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn layer_channel(
        pin: &Arc<SourcePin>,
        file: &[u8],
        at: usize,
        len: usize,
        compression: u16,
        width: usize,
        height: usize,
        is_psb: bool,
    ) -> Result<Self, PsdError> {
        let end = at.checked_add(len).ok_or(PsdError::Truncated)?;
        let payload = file.get(at..end).ok_or(PsdError::Truncated)?;
        match compression {
            COMPRESSION_RAW => Self::raw(pin, at, len, width, height),
            COMPRESSION_RLE => {
                let mut counts = Reader::new(payload);
                let table = height
                    .checked_mul(crate::read::count_bytes(is_psb))
                    .ok_or(PsdError::Truncated)?;
                if table > payload.len() {
                    // The eager decode fails reading the counts.
                    return Err(PsdError::Truncated);
                }
                Self::rle(pin, &mut counts, is_psb, at + table, end, width, height)
            }
            c => Err(PsdError::Unsupported(format!(
                "lazy channel compression {c}"
            ))),
        }
    }

    /// The decoded rows of band `band` (256 rows, fewer at the bottom).
    fn band(&self, band: usize) -> Result<Arc<[u8]>, ()> {
        if let Some(hit) = cache().get(self.key, band) {
            return Ok(hit);
        }
        let r0 = band * TILE;
        let r1 = (r0 + TILE).min(self.height);
        if r0 >= r1 {
            return Err(());
        }
        let mut out = vec![0u8; (r1 - r0) * self.width];
        match &self.rows {
            Rows::Raw(start) => {
                let at = start + (r0 * self.width) as u64;
                self.pin.read_at(at, &mut out)?;
            }
            Rows::Rle(starts) => {
                let (first, last) = (starts[r0], starts[r1]);
                let mut packed = vec![0u8; usize::try_from(last - first).map_err(|_| ())?];
                self.pin.read_at(first, &mut packed)?;
                for (y, row) in (r0..r1).zip(out.chunks_exact_mut(self.width)) {
                    let s = (starts[y] - first) as usize;
                    let e = (starts[y + 1] - first) as usize;
                    decode_packbits(&packed[s..e], row).map_err(|_| ())?;
                }
            }
        }
        self.pin.bands_decoded.fetch_add(1, Ordering::Relaxed);
        let out: Arc<[u8]> = out.into();
        cache().put(self.key, band, Arc::clone(&out));
        Ok(out)
    }

    /// Decode row `y` alone, keeping the last one: a sparse reader samples a
    /// row many times before it moves on.
    fn row(&self, y: usize) -> Result<Arc<[u8]>, ()> {
        let mut last = self.last_row.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, row)) = &*last {
            if *at == y {
                return Ok(Arc::clone(row));
            }
        }
        let mut out = vec![0u8; self.width];
        match &self.rows {
            Rows::Raw(start) => self
                .pin
                .read_at(start + (y * self.width) as u64, &mut out)?,
            Rows::Rle(starts) => {
                let mut packed =
                    vec![0u8; usize::try_from(starts[y + 1] - starts[y]).map_err(|_| ())?];
                self.pin.read_at(starts[y], &mut packed)?;
                decode_packbits(&packed, &mut out).map_err(|_| ())?;
            }
        }
        let out: Arc<[u8]> = out.into();
        *last = Some((y, Arc::clone(&out)));
        Ok(out)
    }

    /// Fill `out` with `rect`, whose rows lie in one band: from the band,
    /// or, for a few rows of a band not cached, from those rows alone.
    fn tile(&self, rect: TileRect, out: &mut [u8]) -> Result<(), ()> {
        let band = rect.y / TILE;
        if rect.h <= SPARSE_ROWS && !cache().contains(self.key, band) {
            for dy in 0..rect.h {
                let row = self.row(rect.y + dy)?;
                let src = row.get(rect.x..rect.x + rect.w).ok_or(())?;
                out[dy * rect.w..(dy + 1) * rect.w].copy_from_slice(src);
            }
            return Ok(());
        }
        let data = self.band(band)?;
        for dy in 0..rect.h {
            let at = (rect.y - band * TILE + dy) * self.width + rect.x;
            let src = data.get(at..at + rect.w).ok_or(())?;
            out[dy * rect.w..(dy + 1) * rect.w].copy_from_slice(src);
        }
        Ok(())
    }

    /// The channel as a tiled plane whose every tile is backed by the file.
    pub(crate) fn into_plane(self) -> Plane<u8> {
        let (width, height) = (self.width, self.height);
        let index = Arc::new(self);
        let decode: TileDecode<u8> = Arc::new(move |rect, out| index.tile(rect, out));
        Plane::backed(width, height, decode)
    }
}

// ---- The band cache ----

/// The most decoded band bytes kept, process-wide: a 17000-wide band is about
/// 4 MiB, so this holds one band of each channel of a layer and of the merged
/// image, which a row-major sweep of tiles reads together.
///
/// ponytail: the governor does not see this heap; it is a fixed 32 MiB.
const CACHE_BYTES: usize = 32 << 20;

/// A band: its channel's key and its number.
type BandKey = (u64, usize);

#[derive(Default)]
struct BandCache {
    /// Each band's rows and use stamp.
    bands: HashMap<BandKey, (Arc<[u8]>, u64)>,
    /// Use stamp → band, oldest first.
    order: BTreeMap<u64, BandKey>,
    bytes: usize,
    clock: u64,
}

static CACHE: LazyLock<Mutex<BandCache>> = LazyLock::new(Mutex::default);

fn cache() -> std::sync::MutexGuard<'static, BandCache> {
    CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

impl BandCache {
    fn contains(&self, key: u64, band: usize) -> bool {
        self.bands.contains_key(&(key, band))
    }

    fn get(&mut self, key: u64, band: usize) -> Option<Arc<[u8]>> {
        self.clock += 1;
        let clock = self.clock;
        let (data, stamp) = self.bands.get_mut(&(key, band))?;
        let data = Arc::clone(data);
        let old = std::mem::replace(stamp, clock);
        self.order.remove(&old);
        self.order.insert(clock, (key, band));
        Some(data)
    }

    fn put(&mut self, key: u64, band: usize, data: Arc<[u8]>) {
        self.clock += 1;
        let clock = self.clock;
        self.bytes += data.len();
        if let Some((old, stamp)) = self.bands.insert((key, band), (data, clock)) {
            self.bytes -= old.len();
            self.order.remove(&stamp);
        }
        self.order.insert(clock, (key, band));
        while self.bytes > CACHE_BYTES {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some((data, _)) = self.bands.remove(&oldest) {
                self.bytes -= data.len();
            }
        }
    }
}

// ---- The lazy read ----

/// [`crate::read_psd_within_options`] for an `options.source` open (#1218), with the
/// file's merged image as a preview when it can stand for the layers on the
/// canvas: the bottom layer is a visible Background with no transparency, so
/// the picture is opaque, and no profile was converted on read.
///
/// ponytail: a transparent document gets no preview. Its merged image holds
/// transparency only when the layer count is negative, which the reader does
/// not keep.
pub fn read_psd_lazy(
    bytes: &[u8],
    options: &OpenOptions,
    budget: ImageBudget,
) -> Result<(Document, Option<MergedPreview>), PsdError> {
    let mut preview = None;
    let mut doc = crate::read::read_container(
        bytes,
        options,
        &mut crate::budget::DecodeBudget::new(budget),
        &mut preview,
    )?;
    crate::path_resources::resolve_paths(&mut doc);
    crate::guide_resources::resolve_guides(&mut doc);
    // The Background flag is inferred from the name (the reader's `derive_background`):
    // a bottom layer of that name with transparency is not opaque.
    let usable = doc.source_icc.is_none()
        && doc.layers.get(0).is_some_and(|bottom| {
            bottom.background && bottom.visible && !bottom.channels.iter().any(|c| c.id == -1)
        });
    Ok((doc, preview.filter(|_| usable)))
}

// ---- The merged preview ----

/// The file's merged image, decoded a rectangle at a time (#1218): the first
/// picture of a lazily opened document until its first edit.
#[derive(Clone)]
pub struct MergedPreview {
    width: u32,
    height: u32,
    channels: Arc<[ChannelIndex; 3]>,
}

impl std::fmt::Debug for MergedPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MergedPreview")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl MergedPreview {
    /// Index the colour planes of a merged image whose image-data section's
    /// payload (after its compression word) starts at `file[at..]`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn index(
        pin: &Arc<SourcePin>,
        file: &[u8],
        at: usize,
        compression: u16,
        header_channels: usize,
        width: usize,
        height: usize,
        is_psb: bool,
    ) -> Result<Self, PsdError> {
        let plane = width
            .checked_mul(height)
            .ok_or_else(|| PsdError::Invalid("image dimensions overflow".into()))?;
        let channels: [ChannelIndex; 3] = match compression {
            COMPRESSION_RAW => {
                let available = file.len().saturating_sub(at);
                let mut out = Vec::with_capacity(3);
                for c in 0..3 {
                    let start = at.checked_add(c * plane).ok_or(PsdError::Truncated)?;
                    out.push(ChannelIndex::raw(
                        pin,
                        start,
                        available.saturating_sub(c * plane),
                        width,
                        height,
                    )?);
                }
                out.try_into().ok().expect("three channels")
            }
            COMPRESSION_RLE => {
                let cb = crate::read::count_bytes(is_psb);
                let rows = header_channels
                    .checked_mul(height)
                    .ok_or(PsdError::Truncated)?;
                let table = rows.checked_mul(cb).ok_or(PsdError::Truncated)?;
                let data_at = at.checked_add(table).ok_or(PsdError::Truncated)?;
                if data_at > file.len() {
                    return Err(PsdError::Truncated);
                }
                let mut counts = Reader::new(&file[at..data_at]);
                let mut start = data_at;
                let mut out = Vec::with_capacity(3);
                for _ in 0..3 {
                    let index = ChannelIndex::rle(
                        pin,
                        &mut counts,
                        is_psb,
                        start,
                        file.len(),
                        width,
                        height,
                    )?;
                    if let Rows::Rle(starts) = &index.rows {
                        start = starts[height] as usize;
                    }
                    out.push(index);
                }
                out.try_into().ok().expect("three channels")
            }
            c => {
                return Err(PsdError::Unsupported(format!(
                    "lazy merged compression {c}"
                )))
            }
        };
        Ok(Self {
            width: width as u32,
            height: height as u32,
            channels: Arc::new(channels),
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// `rect` as straight planar RGBA, opaque inside the image and
    /// transparent outside it. A band that fails to decode reads as
    /// transparent black.
    pub fn region(&self, rect: PsdRect) -> PixelBuffer {
        let (rw, rh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
        let plane = rw * rh;
        let x0 = rect.left.max(0) as usize;
        let x1 = (rect.right.max(0) as usize).min(self.width as usize);
        let y0 = rect.top.max(0) as usize;
        let y1 = (rect.bottom.max(0) as usize).min(self.height as usize);
        let width = self.width as usize;
        let bands = if x0 < x1 && y0 < y1 {
            y0 / TILE..y1.div_ceil(TILE)
        } else {
            0..0
        };
        let n = x1.saturating_sub(x0);
        // Zeroed pages straight from the allocator, written once. A band at a
        // time, so at most one band per channel is held beside the cache.
        let data = Plane::build(plane * 4, |data| {
            for band in bands {
                let rows: Vec<Option<Arc<[u8]>>> =
                    self.channels.iter().map(|c| c.band(band).ok()).collect();
                if rows.iter().any(Option::is_none) {
                    continue;
                }
                let start = (band * TILE).max(y0);
                let end = ((band + 1) * TILE).min(y1);
                for yy in start..end {
                    let src = (yy - band * TILE) * width;
                    let dst = (yy as i64 - i64::from(rect.top)) as usize * rw
                        + (x0 as i64 - i64::from(rect.left)) as usize;
                    for (c, band_rows) in rows.iter().enumerate() {
                        let band_rows = band_rows.as_ref().expect("checked");
                        data[c * plane + dst..c * plane + dst + n]
                            .copy_from_slice(&band_rows[src + x0..src + x1]);
                    }
                    data[3 * plane + dst..3 * plane + dst + n].fill(255);
                }
            }
        });
        PixelBuffer {
            width: rw as u32,
            height: rh as u32,
            channels: 4,
            data,
        }
    }
}

#[cfg(test)]
#[path = "lazy_tests.rs"]
mod tests;
