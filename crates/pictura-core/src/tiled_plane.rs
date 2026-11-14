//! Square planar tiles: the storage behind a tiled [`Plane`](crate::Plane).
//!
//! Each channel of a tiled plane is cut into `TILE × TILE` tiles, clipped at
//! the right and bottom edges. A plane that stacks several channels restarts
//! the grid at each channel, so no tile mixes two channels. Rows are global:
//! channel `c` is rows `c * height .. (c + 1) * height`.

use std::ops::Range;
use std::sync::Arc;

mod body;
mod rows;

pub use crate::memory::eviction::Residency;
use body::{Body, Pin};

/// Tile side in samples, for every sample depth (64 KiB per tile at 8-bit,
/// 128 KiB at 16-bit, 256 KiB at 32-bit).
///
/// ponytail: fixed at 256²; CS6's "Cache Tile Size" preference
/// (`scratch-disks-and-memory.md`) does not drive it.
pub const TILE: usize = 256;

/// A sample a tiled plane can hold.
pub trait TileSample: Copy + Default + PartialEq + Send + Sync + 'static {
    /// Bitwise equality: the samples uniform collapse may fold into one value
    /// without changing a stored bit (`-0.0` and `0.0` stay apart).
    fn same(self, other: Self) -> bool;

    /// Whether every sample of `s` is `v`, bit for bit. Uniform checks run on
    /// every tile write and settle, so the integer samples compare in blocks.
    fn all_same(s: &[Self], v: Self) -> bool {
        s.iter().all(|x| x.same(v))
    }

    /// The samples' bytes in memory order, for packing (#1217).
    fn bytes(s: &[Self]) -> &[u8];

    /// [`TileSample::bytes`], writable, for unpacking.
    fn bytes_mut(s: &mut [Self]) -> &mut [u8];
}

/// The bytes of a slice of plain numbers.
fn pod_bytes<T: Copy>(s: &[T]) -> &[u8] {
    // SAFETY: called only for u16 and f32, which have no padding and no
    // invalid bit patterns; the length covers exactly the slice.
    unsafe { std::slice::from_raw_parts(s.as_ptr().cast::<u8>(), std::mem::size_of_val(s)) }
}

fn pod_bytes_mut<T: Copy>(s: &mut [T]) -> &mut [u8] {
    // SAFETY: as `pod_bytes`; any byte pattern is a valid u16 or f32.
    unsafe { std::slice::from_raw_parts_mut(s.as_mut_ptr().cast::<u8>(), std::mem::size_of_val(s)) }
}

/// Integer samples compare bitwise with `==`, so a block of them against a
/// block of `v` is one slice comparison.
fn blocks_eq<T: Copy + PartialEq>(s: &[T], v: T) -> bool {
    let pattern = [v; 64];
    s.chunks(64).all(|chunk| chunk == &pattern[..chunk.len()])
}

impl TileSample for u8 {
    #[inline]
    fn same(self, other: Self) -> bool {
        self == other
    }

    fn all_same(s: &[Self], v: Self) -> bool {
        blocks_eq(s, v)
    }

    fn bytes(s: &[Self]) -> &[u8] {
        s
    }

    fn bytes_mut(s: &mut [Self]) -> &mut [u8] {
        s
    }
}

impl TileSample for u16 {
    #[inline]
    fn same(self, other: Self) -> bool {
        self == other
    }

    fn all_same(s: &[Self], v: Self) -> bool {
        blocks_eq(s, v)
    }

    fn bytes(s: &[Self]) -> &[u8] {
        pod_bytes(s)
    }

    fn bytes_mut(s: &mut [Self]) -> &mut [u8] {
        pod_bytes_mut(s)
    }
}

impl TileSample for f32 {
    #[inline]
    fn same(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }

    fn bytes(s: &[Self]) -> &[u8] {
        pod_bytes(s)
    }

    fn bytes_mut(s: &mut [Self]) -> &mut [u8] {
        pod_bytes_mut(s)
    }
}

/// A tile's rectangle in its channel: first column, first row, and clipped
/// size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileRect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// Decodes a backed tile's samples (#1218): fills `out` with the `w × h`
/// samples of `rect`, row after row. `rect` is a whole tile, or a part of one
/// for a sparse transient read. An error leaves the tile unreadable. One
/// closure serves every tile of a channel; it is the codec's, so core never
/// names a file format.
pub type TileDecode<T> = Arc<dyn Fn(TileRect, &mut [T]) -> Result<(), ()> + Send + Sync>;

pub(crate) enum Tile<T> {
    /// Reads as `T::default()` and holds no allocation.
    Empty,
    /// Every sample reads as the value; no allocation.
    Uniform(T),
    /// A shared body of the clipped tile, copied on first write; its samples
    /// may be packed or spilled (#1217).
    Dense(Arc<Body<T>>),
}

impl<T: Clone> Clone for Tile<T> {
    fn clone(&self) -> Self {
        match self {
            Tile::Empty => Tile::Empty,
            Tile::Uniform(v) => Tile::Uniform(v.clone()),
            Tile::Dense(d) => Tile::Dense(Arc::clone(d)),
        }
    }
}

/// One tile held outside its plane, with its generation: a refcount on the
/// tile's body, never a copy of its samples (#1215). History keeps the tiles
/// an edit replaced this way and swaps them back on undo; a tile taken with
/// [`crate::Plane::history_tile`] marks its body as history's, which the
/// evictor frees before live tiles (#1217).
pub struct SharedTile<T> {
    tile: Tile<T>,
    gen: u64,
    history: bool,
}

impl<T> SharedTile<T> {
    /// The dense body as `(address, samples)`; `None` for an empty or
    /// uniform tile, which holds none.
    pub fn allocation(&self) -> Option<(usize, usize)> {
        match &self.tile {
            Tile::Dense(d) => Some((d.address(), d.len())),
            _ => None,
        }
    }

    fn hold(&self) {
        if let (true, Tile::Dense(d)) = (self.history, &self.tile) {
            d.hold_history();
        }
    }

    fn release(&self) {
        if let (true, Tile::Dense(d)) = (self.history, &self.tile) {
            d.release_history();
        }
    }
}

impl<T: Clone> Clone for SharedTile<T> {
    fn clone(&self) -> Self {
        let copy = Self {
            tile: self.tile.clone(),
            gen: self.gen,
            history: self.history,
        };
        copy.hold();
        copy
    }
}

impl<T> Drop for SharedTile<T> {
    fn drop(&mut self) {
        self.release();
    }
}

/// What a tile holds, for diagnostics and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TileState<T> {
    Empty,
    Uniform(T),
    /// A dense allocation of this many samples.
    Dense(usize),
}

/// A tile's samples as a reader sees them.
pub enum TileData<'a, T> {
    /// Every sample is this value (an `Empty` tile reads as the default).
    Uniform(T),
    /// The samples, `stride` per row.
    Dense(DenseRef<'a, T>),
}

/// A dense tile's samples, held resident while this lives (#1217).
pub struct DenseRef<'a, T> {
    samples: &'a [T],
    _pin: Option<Pin<'a, T>>,
}

impl<'a, T> DenseRef<'a, T> {
    /// Samples that cannot be evicted (a flat plane's).
    pub(crate) fn fixed(samples: &'a [T]) -> Self {
        Self {
            samples,
            _pin: None,
        }
    }

    fn pinned(pin: Pin<'a, T>) -> Self {
        // SAFETY: the samples stay resident and in place while the pin
        // lives, and the pin lives exactly as long as this reference, which
        // hands them out only for its own borrow.
        let samples = unsafe { std::slice::from_raw_parts(pin.as_ptr(), pin.len()) };
        Self {
            samples,
            _pin: Some(pin),
        }
    }
}

impl<T> std::ops::Deref for DenseRef<'_, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.samples
    }
}

/// One tile, read in place. `x` and `y` are the tile's first column and
/// global row; `w × h` its clipped size.
pub struct TileRef<'a, T> {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub stride: usize,
    pub data: TileData<'a, T>,
}

impl<T: Copy> TileRef<'_, T> {
    /// The sample at tile-local `(dx, dy)`.
    pub fn get(&self, dx: usize, dy: usize) -> T {
        match &self.data {
            TileData::Uniform(v) => *v,
            TileData::Dense(d) => d[dy * self.stride + dx],
        }
    }

    /// The value every sample holds, when the tile is uniform.
    pub fn uniform(&self) -> Option<T> {
        match &self.data {
            TileData::Uniform(v) => Some(*v),
            TileData::Dense(_) => None,
        }
    }

    /// Row `dy`'s samples, when the tile is dense.
    pub fn row(&self, dy: usize) -> Option<&[T]> {
        match &self.data {
            TileData::Uniform(_) => None,
            TileData::Dense(d) => Some(&d[dy * self.stride..dy * self.stride + self.w]),
        }
    }

    /// Copy tile-local columns `xs` of row `dy` into `out`.
    pub fn copy_row(&self, dy: usize, xs: Range<usize>, out: &mut [T]) {
        match &self.data {
            TileData::Uniform(v) => out[..xs.len()].fill(*v),
            TileData::Dense(d) => {
                let at = dy * self.stride;
                out[..xs.len()].copy_from_slice(&d[at + xs.start..at + xs.end]);
            }
        }
    }
}

/// One tile, writable. `data` holds `h` rows of `stride` samples, of which the
/// first `w` are the tile's.
pub struct TileMut<'a, T> {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub stride: usize,
    pub data: &'a mut [T],
}

impl<T> TileMut<'_, T> {
    pub fn row(&self, dy: usize) -> &[T] {
        &self.data[dy * self.stride..dy * self.stride + self.w]
    }

    pub fn row_mut(&mut self, dy: usize) -> &mut [T] {
        &mut self.data[dy * self.stride..dy * self.stride + self.w]
    }
}

pub(crate) struct TiledPlane<T> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) channels: usize,
    // u32 with the pin list boxed keeps a plane (and every `PixelBuffer`)
    // the size it was before #1217.
    cols: u32,
    rows: u32,
    tiles: Vec<Tile<T>>,
    gens: Vec<u64>,
    dirty: Vec<bool>,
    /// Tiles whose samples a `&mut` borrow handed out, pinned until the next
    /// mutable call (when no such slice can be alive) or the plane drops.
    pinned: Option<Box<[usize]>>,
}

impl<T: Clone> Clone for TiledPlane<T> {
    fn clone(&self) -> Self {
        self.with_tiles(self.tiles.clone(), self.gens.clone(), self.dirty.clone())
    }
}

impl<T> Drop for TiledPlane<T> {
    fn drop(&mut self) {
        self.release_pins();
    }
}

impl<T> TiledPlane<T> {
    /// Unpin what the last mutable borrow pinned. Every `&mut self` method
    /// that can replace or hand out a tile calls this first.
    fn release_pins(&mut self) {
        for &i in self.pinned.take().iter().flatten() {
            if let Tile::Dense(d) = &self.tiles[i] {
                d.unpin();
            }
        }
    }

    /// A plane of this geometry holding `tiles`, with no pins.
    fn with_tiles(&self, tiles: Vec<Tile<T>>, gens: Vec<u64>, dirty: Vec<bool>) -> Self {
        Self {
            width: self.width,
            height: self.height,
            channels: self.channels,
            cols: self.cols,
            rows: self.rows,
            tiles,
            gens,
            dirty,
            pinned: None,
        }
    }
}

impl<T: TileSample> TiledPlane<T> {
    /// A plane of `width × height × channels` samples whose tiles are all
    /// `Uniform(value)` (or `Empty` at the default).
    pub(crate) fn filled(width: usize, height: usize, channels: usize, value: T) -> Self {
        let cols = width.div_ceil(TILE);
        let rows = height.div_ceil(TILE);
        let n = cols * rows * channels;
        let tile = if value.same(T::default()) {
            Tile::Empty
        } else {
            Tile::Uniform(value)
        };
        Self {
            width,
            height,
            channels,
            cols: cols as u32,
            rows: rows as u32,
            tiles: vec![tile; n],
            gens: vec![crate::fresh_stamp(); n],
            dirty: vec![false; n],
            pinned: None,
        }
    }

    /// A one-channel plane whose every tile is away, backed by `decode`
    /// (#1218). Nothing is decoded until a tile is read, so no tile is known
    /// to be uniform.
    pub(crate) fn backed(width: usize, height: usize, decode: TileDecode<T>) -> Self {
        let mut plane = Self::filled(width, height, 1, T::default());
        for i in 0..plane.tile_count() {
            let (x, y, w, h) = plane.tile_rect(i);
            let source = body::Source {
                decode: Arc::clone(&decode),
                rect: TileRect { x, y, w, h },
            };
            plane.tiles[i] = Tile::Dense(Body::backed(w * h, source));
        }
        plane
    }

    /// Copy a contiguous plane into tiles, one tile per core; a tile that holds
    /// one value becomes `Uniform` (or `Empty`) without an allocation.
    pub(crate) fn from_flat(data: &[T], width: usize, height: usize) -> Self {
        use rayon::prelude::*;
        let channels = data.len() / (width * height);
        let mut plane = Self::filled(width, height, channels, T::default());
        let shape = &plane;
        let tiles: Vec<Tile<T>> = (0..shape.tile_count())
            .into_par_iter()
            .map(|i| {
                let (x, y, w, h) = shape.tile_rect(i);
                let row = |r: usize| &data[(y + r) * width + x..(y + r) * width + x + w];
                let first = data[y * width + x];
                if (0..h).all(|r| all_same(row(r), first)) {
                    return if first.same(T::default()) {
                        Tile::Empty
                    } else {
                        Tile::Uniform(first)
                    };
                }
                let mut samples = vec![first; w * h].into_boxed_slice();
                for (r, dst) in samples.chunks_mut(w).enumerate() {
                    dst.copy_from_slice(row(r));
                }
                Tile::Dense(Body::new(samples))
            })
            .collect();
        plane.tiles = tiles;
        plane
    }

    pub(crate) fn len(&self) -> usize {
        self.width * self.height * self.channels
    }

    pub(crate) fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub(crate) fn same_geometry(&self, other: &Self) -> bool {
        (self.width, self.height, self.channels) == (other.width, other.height, other.channels)
    }

    /// Tile `i`'s first column, first global row, width and height.
    pub(crate) fn tile_rect(&self, i: usize) -> (usize, usize, usize, usize) {
        let (cols, rows) = (self.cols as usize, self.rows as usize);
        let tx = i % cols;
        let ty = (i / cols) % rows;
        let c = i / (cols * rows);
        let x = tx * TILE;
        let local = ty * TILE;
        (
            x,
            c * self.height + local,
            TILE.min(self.width - x),
            TILE.min(self.height - local),
        )
    }

    /// The tile holding column `x` of global row `y`, and the sample's offset
    /// in it.
    fn locate(&self, x: usize, y: usize) -> (usize, usize) {
        let (c, ly) = (y / self.height, y % self.height);
        let (tx, ty) = (x / TILE, ly / TILE);
        let i = (c * self.rows as usize + ty) * self.cols as usize + tx;
        let w = TILE.min(self.width - tx * TILE);
        (i, (ly % TILE) * w + x % TILE)
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> T {
        let (i, off) = self.locate(x, y);
        match &self.tiles[i] {
            Tile::Empty => T::default(),
            Tile::Uniform(v) => *v,
            Tile::Dense(d) => {
                if crate::memory::eviction::is_transient() {
                    let (dx, dy) = (x % TILE, (y % self.height) % TILE);
                    if let Some(v) = d.peek_sample(dx, dy) {
                        return v;
                    }
                }
                d.pin()[off]
            }
        }
    }

    /// Where tile `i`'s samples are; `None` past the last tile.
    pub(crate) fn residency(&self, i: usize) -> Option<Residency> {
        self.tiles.get(i).map(|t| match t {
            Tile::Dense(d) => crate::memory::eviction::residency_of(&**d),
            _ => Residency::Resident,
        })
    }

    /// Dense tiles whose samples were lost from scratch.
    pub(crate) fn unreadable_tiles(&self) -> usize {
        if crate::memory::eviction::unreadable_count() == 0 {
            return 0;
        }
        self.tiles
            .iter()
            .filter(|t| matches!(t, Tile::Dense(d) if d.is_unreadable()))
            .count()
    }

    pub(crate) fn state(&self, i: usize) -> Option<TileState<T>> {
        self.tiles.get(i).map(|t| match t {
            Tile::Empty => TileState::Empty,
            Tile::Uniform(v) => TileState::Uniform(*v),
            Tile::Dense(d) => TileState::Dense(d.len()),
        })
    }

    pub(crate) fn tile_ref(&self, i: usize) -> TileRef<'_, T> {
        let (x, y, w, h) = self.tile_rect(i);
        let data = match &self.tiles[i] {
            Tile::Empty => TileData::Uniform(T::default()),
            Tile::Uniform(v) => TileData::Uniform(*v),
            Tile::Dense(d) => TileData::Dense(DenseRef::pinned(d.pin())),
        };
        TileRef {
            x,
            y,
            w,
            h,
            stride: w,
            data,
        }
    }

    /// The indices of the tiles meeting columns `xs` × global rows `ys`, in
    /// row-major order.
    pub(crate) fn tiles_in(&self, xs: Range<usize>, ys: Range<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        self.each_tile_in(xs, ys, |i| out.push(i));
        out
    }

    /// [`Self::tiles_in`] without collecting: `f` gets each index in order.
    pub(crate) fn each_tile_in(
        &self,
        xs: Range<usize>,
        ys: Range<usize>,
        mut f: impl FnMut(usize),
    ) {
        let (x0, x1) = (xs.start, xs.end.min(self.width));
        let total = self.height * self.channels;
        let (y0, y1) = (ys.start, ys.end.min(total));
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let (tx0, tx1) = (x0 / TILE, x1.div_ceil(TILE));
        let mut y = y0;
        while y < y1 {
            let (c, ly) = (y / self.height, y % self.height);
            let ty = ly / TILE;
            for tx in tx0..tx1 {
                f((c * self.rows as usize + ty) * self.cols as usize + tx);
            }
            let next_local = ((ty + 1) * TILE).min(self.height);
            y += next_local - ly;
        }
    }

    pub(crate) fn generation(&self, i: usize) -> u64 {
        self.gens.get(i).copied().unwrap_or(0)
    }

    pub(crate) fn shares_tile(&self, i: usize, other: &Self) -> bool {
        self.same_geometry(other) && i < self.gens.len() && self.gens[i] == other.gens[i]
    }

    /// Take `other`'s tile `i` (same allocation, same generation).
    pub(crate) fn adopt_tile(&mut self, i: usize, other: &Self) -> bool {
        if !self.same_geometry(other) || i >= self.tiles.len() {
            return false;
        }
        self.release_pins();
        self.tiles[i] = other.tiles[i].clone();
        self.gens[i] = other.gens[i];
        self.dirty[i] = other.dirty[i];
        true
    }

    /// Tile `i` and its generation, sharing the allocation; `history` marks
    /// the body as held by history while the tile lives.
    pub(crate) fn shared_tile(&self, i: usize, history: bool) -> Option<SharedTile<T>> {
        let tile = SharedTile {
            tile: self.tiles.get(i)?.clone(),
            gen: self.gens[i],
            history,
        };
        tile.hold();
        Some(tile)
    }

    /// Exchange tile `i` and its generation with `held`. The tile came out of a
    /// plane of this geometry at the same index, so its size matches.
    pub(crate) fn swap_tile(&mut self, i: usize, held: &mut SharedTile<T>) -> bool {
        if i >= self.tiles.len() {
            return false;
        }
        self.release_pins();
        held.release();
        std::mem::swap(&mut self.tiles[i], &mut held.tile);
        held.hold();
        std::mem::swap(&mut self.gens[i], &mut held.gen);
        self.dirty[i] = false;
        true
    }

    /// Make tile `i` a uniquely held dense tile with a new generation, marked
    /// for the next settle.
    fn fork(&mut self, i: usize, gen: u64) {
        self.release_pins();
        let (_, _, w, h) = self.tile_rect(i);
        fork_tile(&mut self.tiles[i], w * h);
        self.gens[i] = gen;
        self.dirty[i] = true;
    }

    /// The tiles under columns `xs` × global rows `ys`, forked and writable,
    /// in row-major order. They are disjoint, so a caller may split them
    /// across threads.
    pub(crate) fn tiles_mut(&mut self, xs: Range<usize>, ys: Range<usize>) -> Vec<TileMut<'_, T>> {
        let wanted = self.tiles_in(xs, ys);
        self.tiles_mut_at(&wanted)
    }

    /// The tiles at `wanted` (strictly ascending, in range), forked and
    /// writable, in that order.
    pub(crate) fn tiles_mut_at(&mut self, wanted: &[usize]) -> Vec<TileMut<'_, T>> {
        use rayon::prelude::*;
        self.release_pins();
        let gen = crate::fresh_stamp();
        let rects: Vec<_> = wanted.iter().map(|&i| self.tile_rect(i)).collect();
        let mut fresh = Vec::new();
        for (&i, &(_, _, w, h)) in wanted.iter().zip(&rects) {
            self.gens[i] = gen;
            self.dirty[i] = true;
            // ponytail: this check-then-act is sound only while tile bodies
            // are cloned on the GUI thread (save, tile jobs and history
            // capture clone before they spawn; the evictor holds only `Weak`).
            // A worker-side clone landing between the check and the write
            // would mutate a body another holder shares, breaking the
            // copy-on-write history relies on.
            let unique = match &self.tiles[i] {
                Tile::Dense(d) => Arc::strong_count(d) == 1,
                _ => false,
            };
            if !unique {
                fresh.push((i, w * h));
            }
        }
        // A first write fills whole tiles. Many at once are spread over the
        // cores, as the zero pages of a contiguous plane were faulted in by
        // the raster; a few are not worth a parallel dispatch.
        if fresh.len() > 4 {
            let mut size = vec![0; self.tiles.len()];
            for &(i, n) in &fresh {
                size[i] = n;
            }
            self.tiles
                .par_iter_mut()
                .zip(&size)
                .filter(|(_, n)| **n > 0)
                .for_each(|(tile, &n)| fork_tile(tile, n));
        } else {
            for (i, n) in fresh {
                fork_tile(&mut self.tiles[i], n);
            }
        }
        let mut out = Vec::with_capacity(wanted.len());
        let mut pinned = Vec::with_capacity(wanted.len());
        for (&i, (x, y, w, h)) in wanted.iter().zip(rects) {
            let Tile::Dense(d) = &self.tiles[i] else {
                continue;
            };
            d.pin_raw();
            d.dirty();
            pinned.push(i);
            // SAFETY: pinned until the next `&mut self` call, which needs this
            // borrow to have ended; forked above, so this plane holds the
            // body's only strong reference; `wanted` is strictly ascending,
            // so no two slices alias.
            let data = unsafe { d.samples_mut() };
            let data = unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
            out.push(TileMut {
                x,
                y,
                w,
                h,
                stride: w,
                data,
            });
        }
        self.pinned = (!pinned.is_empty()).then(|| pinned.into_boxed_slice());
        out
    }

    /// Make tile `i` hold `value` everywhere, without an allocation, under a
    /// new generation.
    pub(crate) fn set_uniform(&mut self, i: usize, value: T) {
        self.release_pins();
        self.tiles[i] = if value.same(T::default()) {
            Tile::Empty
        } else {
            Tile::Uniform(value)
        };
        self.gens[i] = crate::fresh_stamp();
        self.dirty[i] = false;
    }

    /// The one value every sample of columns `xs` × global rows `ys` holds,
    /// when every tile meeting them is uniform at that value; `None` when one
    /// is dense or two disagree. Samples are not scanned.
    pub(crate) fn uniform_over(&self, xs: Range<usize>, ys: Range<usize>) -> Option<T> {
        let mut found: Option<T> = None;
        let mut mixed = false;
        self.each_tile_in(xs, ys, |i| {
            if mixed {
                return;
            }
            let v = match &self.tiles[i] {
                Tile::Empty => T::default(),
                Tile::Uniform(v) => *v,
                Tile::Dense(_) => {
                    mixed = true;
                    return;
                }
            };
            match found {
                Some(f) if !f.same(v) => mixed = true,
                _ => found = Some(v),
            }
        });
        if mixed {
            None
        } else {
            found
        }
    }

    /// A copy holding the same samples in allocations of its own (dense
    /// tiles copied in parallel), with fresh generations, so neither side's
    /// next write copies anything the other holds.
    pub(crate) fn detached(&self) -> Self {
        use rayon::prelude::*;
        let tiles = self
            .tiles
            .par_iter()
            .map(|t| match t {
                Tile::Dense(d) => Tile::Dense(Body::new(Box::from(&*d.pin()))),
                other => other.clone(),
            })
            .collect();
        self.with_tiles(
            tiles,
            vec![crate::fresh_stamp(); self.gens.len()],
            self.dirty.clone(),
        )
    }

    /// Collapse every written tile whose samples are all one value into
    /// `Uniform` (or `Empty` at the default). No sample, and no generation,
    /// changes.
    pub(crate) fn settle(&mut self) {
        use rayon::prelude::*;
        self.release_pins();
        if !self.dirty.contains(&true) {
            return;
        }
        self.tiles
            .par_iter_mut()
            .zip(self.dirty.par_iter_mut())
            .for_each(|(tile, dirty)| {
                if !std::mem::take(dirty) {
                    return;
                }
                let Tile::Dense(d) = tile else {
                    return;
                };
                let samples = d.pin();
                let first = samples[0];
                let uniform = all_same(&samples, first);
                drop(samples);
                if uniform {
                    *tile = if first.same(T::default()) {
                        Tile::Empty
                    } else {
                        Tile::Uniform(first)
                    };
                }
            });
    }

    /// Whether every sample equals `value`; uniform tiles are not scanned.
    pub(crate) fn all_eq(&self, value: T) -> bool {
        self.tiles.iter().all(|t| match t {
            Tile::Empty => T::default() == value,
            Tile::Uniform(v) => *v == value,
            Tile::Dense(d) => d.pin().iter().all(|s| *s == value),
        })
    }

    /// Each dense allocation as `(address, samples)`.
    pub(crate) fn allocations(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.tiles.iter().filter_map(|t| match t {
            Tile::Dense(d) => Some((d.address(), d.len())),
            _ => None,
        })
    }

    /// The plane's samples, row after row, in one new allocation.
    pub(crate) fn materialize(&self) -> Vec<T> {
        let mut out = vec![T::default(); self.len()];
        self.read_span(0, &mut out);
        out
    }

    /// Write `f` into forked tile `i`'s samples, pinned for the call.
    fn with_dense_mut(&mut self, i: usize, f: impl FnOnce(&mut [T])) {
        let Tile::Dense(d) = &self.tiles[i] else {
            unreachable!("the tile was forked");
        };
        let pin = d.pin();
        d.dirty();
        // SAFETY: pinned for the call; forked, so this plane (borrowed
        // mutably) holds the only strong reference.
        f(unsafe { d.samples_mut() });
        drop(pin);
    }

    /// Whether every sample of `src` already equals tile `i`'s uniform value,
    /// so writing it would change nothing.
    fn writes_nothing<'s>(&self, i: usize, mut rows: impl Iterator<Item = &'s [T]>) -> bool {
        let v = match &self.tiles[i] {
            Tile::Empty => T::default(),
            Tile::Uniform(v) => *v,
            Tile::Dense(_) => return false,
        };
        rows.all(|row| all_same(row, v))
    }
}

/// Whether every sample of `s` is `v`, bit for bit.
pub(crate) fn all_same<T: TileSample>(s: &[T], v: T) -> bool {
    T::all_same(s, v)
}

/// Make `tile` (of `n` samples) a uniquely held dense tile.
fn fork_tile<T: TileSample>(tile: &mut Tile<T>, n: usize) {
    match tile {
        Tile::Empty => *tile = Tile::Dense(Body::filled(n, T::default())),
        Tile::Uniform(v) => *tile = Tile::Dense(Body::filled(n, *v)),
        Tile::Dense(d) => {
            // Sound under the GUI-thread clone rule of `tiles_mut_at`.
            if Arc::strong_count(d) > 1 {
                let copy = Body::new(Box::from(&*d.pin()));
                *d = copy;
            }
        }
    }
}

#[cfg(test)]
mod tests;
