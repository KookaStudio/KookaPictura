//! Square planar tiles: the storage behind a tiled [`Plane`](crate::Plane).
//!
//! Each channel of a tiled plane is cut into `TILE × TILE` tiles, clipped at
//! the right and bottom edges. A plane that stacks several channels restarts
//! the grid at each channel, so no tile mixes two channels. Rows are global:
//! channel `c` is rows `c * height .. (c + 1) * height`.

use std::ops::Range;
use std::sync::Arc;

mod rows;

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
}

impl TileSample for u16 {
    #[inline]
    fn same(self, other: Self) -> bool {
        self == other
    }

    fn all_same(s: &[Self], v: Self) -> bool {
        blocks_eq(s, v)
    }
}

impl TileSample for f32 {
    #[inline]
    fn same(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

#[derive(Clone)]
pub(crate) enum Tile<T> {
    /// Reads as `T::default()` and holds no allocation.
    Empty,
    /// Every sample reads as the value; no allocation.
    Uniform(T),
    /// A shared allocation of the clipped tile, copied on first write.
    Dense(Arc<[T]>),
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
#[derive(Clone, Copy)]
pub enum TileData<'a, T> {
    /// Every sample is this value (an `Empty` tile reads as the default).
    Uniform(T),
    /// The samples, `stride` per row.
    Dense(&'a [T]),
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
        match self.data {
            TileData::Uniform(v) => v,
            TileData::Dense(d) => d[dy * self.stride + dx],
        }
    }

    /// The value every sample holds, when the tile is uniform.
    pub fn uniform(&self) -> Option<T> {
        match self.data {
            TileData::Uniform(v) => Some(v),
            TileData::Dense(_) => None,
        }
    }

    /// Row `dy`'s samples, when the tile is dense.
    pub fn row(&self, dy: usize) -> Option<&[T]> {
        match self.data {
            TileData::Uniform(_) => None,
            TileData::Dense(d) => Some(&d[dy * self.stride..dy * self.stride + self.w]),
        }
    }

    /// Copy tile-local columns `xs` of row `dy` into `out`.
    pub fn copy_row(&self, dy: usize, xs: Range<usize>, out: &mut [T]) {
        match self.data {
            TileData::Uniform(v) => out[..xs.len()].fill(v),
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

#[derive(Clone)]
pub(crate) struct TiledPlane<T> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) channels: usize,
    cols: usize,
    rows: usize,
    tiles: Vec<Tile<T>>,
    gens: Vec<u64>,
    dirty: Vec<bool>,
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
            cols,
            rows,
            tiles: vec![tile; n],
            gens: vec![crate::fresh_stamp(); n],
            dirty: vec![false; n],
        }
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
                let mut dense = filled_tile(w * h, first);
                let out = Arc::get_mut(&mut dense).expect("a fresh tile is unique");
                for (r, dst) in out.chunks_mut(w).enumerate() {
                    dst.copy_from_slice(row(r));
                }
                Tile::Dense(dense)
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
        let tx = i % self.cols;
        let ty = (i / self.cols) % self.rows;
        let c = i / (self.cols * self.rows);
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
        let i = (c * self.rows + ty) * self.cols + tx;
        let w = TILE.min(self.width - tx * TILE);
        (i, (ly % TILE) * w + x % TILE)
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> T {
        let (i, off) = self.locate(x, y);
        match &self.tiles[i] {
            Tile::Empty => T::default(),
            Tile::Uniform(v) => *v,
            Tile::Dense(d) => d[off],
        }
    }

    /// Columns `xs` of global row `y` borrowed in place, when they lie in one
    /// dense tile.
    pub(crate) fn span_in_tile(&self, y: usize, xs: Range<usize>) -> Option<&[T]> {
        if xs.is_empty() || xs.end > self.width || (xs.end - 1) / TILE != xs.start / TILE {
            return None;
        }
        let (i, off) = self.locate(xs.start, y);
        match &self.tiles[i] {
            Tile::Dense(d) => Some(&d[off..off + xs.len()]),
            _ => None,
        }
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
            Tile::Dense(d) => TileData::Dense(d),
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
                f((c * self.rows + ty) * self.cols + tx);
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
        self.tiles[i] = other.tiles[i].clone();
        self.gens[i] = other.gens[i];
        self.dirty[i] = other.dirty[i];
        true
    }

    /// Make tile `i` a uniquely held dense tile with a new generation, marked
    /// for the next settle.
    fn fork(&mut self, i: usize, gen: u64) {
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
        let gen = crate::fresh_stamp();
        let rects: Vec<_> = wanted.iter().map(|&i| self.tile_rect(i)).collect();
        let mut fresh = Vec::new();
        for (&i, &(_, _, w, h)) in wanted.iter().zip(&rects) {
            self.gens[i] = gen;
            self.dirty[i] = true;
            let unique = match &mut self.tiles[i] {
                Tile::Dense(d) => Arc::get_mut(d).is_some(),
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
        let mut next = wanted.iter().zip(rects).peekable();
        for (i, tile) in self.tiles.iter_mut().enumerate() {
            let Some(&(&want, (x, y, w, h))) = next.peek() else {
                break;
            };
            if i != want {
                continue;
            }
            next.next();
            if let Tile::Dense(d) = tile {
                let data = Arc::get_mut(d).expect("a forked tile is unique");
                out.push(TileMut {
                    x,
                    y,
                    w,
                    h,
                    stride: w,
                    data,
                });
            }
        }
        out
    }

    /// Make tile `i` hold `value` everywhere, without an allocation, under a
    /// new generation.
    pub(crate) fn set_uniform(&mut self, i: usize, value: T) {
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
                Tile::Dense(d) => Tile::Dense(Arc::from(&d[..])),
                other => other.clone(),
            })
            .collect();
        Self {
            tiles,
            gens: vec![crate::fresh_stamp(); self.gens.len()],
            dirty: self.dirty.clone(),
            ..self.clone_shape()
        }
    }

    fn clone_shape(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            channels: self.channels,
            cols: self.cols,
            rows: self.rows,
            tiles: Vec::new(),
            gens: Vec::new(),
            dirty: Vec::new(),
        }
    }

    /// Collapse every written tile whose samples are all one value into
    /// `Uniform` (or `Empty` at the default). No sample, and no generation,
    /// changes.
    pub(crate) fn settle(&mut self) {
        use rayon::prelude::*;
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
                let first = d[0];
                if all_same(d, first) {
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
            Tile::Dense(d) => d.iter().all(|s| *s == value),
        })
    }

    /// Each dense allocation as `(address, samples)`.
    pub(crate) fn allocations(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.tiles.iter().filter_map(|t| match t {
            Tile::Dense(d) => Some((d.as_ptr() as usize, d.len())),
            _ => None,
        })
    }

    /// The plane's samples, row after row, in one new allocation.
    pub(crate) fn materialize(&self) -> Vec<T> {
        let mut out = vec![T::default(); self.len()];
        self.read_span(0, &mut out);
        out
    }

    fn dense_mut(&mut self, i: usize) -> &mut [T] {
        match &mut self.tiles[i] {
            Tile::Dense(d) => Arc::get_mut(d).expect("a forked tile is unique"),
            _ => unreachable!("the tile was forked"),
        }
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

/// A new tile allocation of `n` copies of `v`, built in place.
fn filled_tile<T: TileSample>(n: usize, v: T) -> Arc<[T]> {
    std::iter::repeat_n(v, n).collect()
}

/// Make `tile` (of `n` samples) a uniquely held dense tile.
fn fork_tile<T: TileSample>(tile: &mut Tile<T>, n: usize) {
    match tile {
        Tile::Empty => *tile = Tile::Dense(filled_tile(n, T::default())),
        Tile::Uniform(v) => *tile = Tile::Dense(filled_tile(n, *v)),
        Tile::Dense(d) => {
            if Arc::get_mut(d).is_none() {
                *d = Arc::from(&d[..]);
            }
        }
    }
}

#[cfg(test)]
mod tests;
