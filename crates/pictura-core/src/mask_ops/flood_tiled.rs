//! A four-connected scanline flood over the 256² tiles of the document grid
//! (#1214): the source is asked per tile as the flood reaches it, the visited
//! state is one bit per pixel for the reached tiles only, and a tile reached
//! whole is never visited pixel by pixel.

use rayon::prelude::*;

use super::TiledMask;
use crate::tiled_plane::TILE;
use crate::Plane;

/// `u64` words in one tile's bitmap: 256 × 256 bits, stride 256.
const WORDS: usize = TILE * TILE / 64;

/// One tile's bits, one per pixel at tile-local `dy * 256 + dx`.
pub type TileBits = Box<[u64]>;

fn new_bits() -> TileBits {
    vec![0u64; WORDS].into_boxed_slice()
}

#[inline]
fn bit_index(dx: usize, dy: usize) -> usize {
    dy * TILE + dx
}

#[inline]
fn get_bit(bits: &[u64], i: usize) -> bool {
    (bits[i >> 6] >> (i & 63)) & 1 != 0
}

#[inline]
fn set_bit(bits: &mut [u64], i: usize) {
    bits[i >> 6] |= 1 << (i & 63);
}

/// What a tiled flood asks of its source.
pub trait FloodSource {
    /// Whether pixel `to` joins the flood from its reached four-neighbour
    /// `from`; both are `(x, y)`.
    fn matches(&mut self, from: (usize, usize), to: (usize, usize)) -> bool;

    /// Whether reaching any pixel of tile `tile` (row-major on the grid)
    /// reaches every pixel of it: true only when the tile's pixels are all
    /// connected under [`Self::matches`] and none is a wall.
    fn whole(&mut self, tile: usize) -> bool;
}

/// Where a tiled flood starts.
pub enum Seeds<'a> {
    /// One pixel, `(x, y)`.
    Pixel(usize, usize),
    /// Every non-zero pixel of a mask of the flood's size.
    Mask(&'a TiledMask),
}

/// The tile grid of a `w × h` plane.
#[derive(Clone, Copy)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub cols: usize,
    pub rows: usize,
}

impl Grid {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            cols: w.div_ceil(TILE),
            rows: h.div_ceil(TILE),
        }
    }

    pub fn count(&self) -> usize {
        self.cols * self.rows
    }

    #[inline]
    pub fn tile_of(&self, x: usize, y: usize) -> usize {
        (y / TILE) * self.cols + x / TILE
    }

    /// Tile `t`'s first column, first row, width and height.
    pub fn rect(&self, t: usize) -> (usize, usize, usize, usize) {
        let (x, y) = ((t % self.cols) * TILE, (t / self.cols) * TILE);
        (x, y, TILE.min(self.w - x), TILE.min(self.h - y))
    }
}

/// A tile's answers to a predicate of the target pixel alone.
pub enum TileMatch {
    /// No pixel matches.
    None,
    /// Every pixel matches.
    All,
    /// The matching pixels.
    Bits(TileBits),
}

impl TileMatch {
    /// The answers for a `tw × th` tile, asked one row at a time: `row(dy,
    /// out)` sets `out[dx]` for the row's `tw` pixels.
    pub fn from_rows(tw: usize, th: usize, mut row: impl FnMut(usize, &mut [bool])) -> Self {
        let mut bits = new_bits();
        let mut out = vec![false; tw];
        let (mut any, mut all) = (false, true);
        for dy in 0..th {
            out.fill(false);
            row(dy, &mut out);
            for (dx, &hit) in out.iter().enumerate() {
                if hit {
                    any = true;
                    set_bit(&mut bits, bit_index(dx, dy));
                } else {
                    all = false;
                }
            }
        }
        match (any, all) {
            (false, _) => TileMatch::None,
            (true, true) => TileMatch::All,
            _ => TileMatch::Bits(bits),
        }
    }

    #[inline]
    fn get(&self, dx: usize, dy: usize) -> bool {
        match self {
            TileMatch::None => false,
            TileMatch::All => true,
            TileMatch::Bits(b) => get_bit(b, bit_index(dx, dy)),
        }
    }
}

/// A [`FloodSource`] for a predicate of the target pixel alone (the Paint
/// Bucket, the Magic Wand): each tile's answers are computed by `eval(tile,
/// rect)` the first time the flood asks about it, and a tile that matches
/// whole is [`FloodSource::whole`].
pub struct MatchTiles<F> {
    grid: Grid,
    tiles: Vec<Option<TileMatch>>,
    eval: F,
}

impl<F> MatchTiles<F>
where
    F: FnMut(usize, (usize, usize, usize, usize)) -> TileMatch,
{
    pub fn new(w: usize, h: usize, eval: F) -> Self {
        let grid = Grid::new(w, h);
        Self {
            tiles: (0..grid.count()).map(|_| None).collect(),
            grid,
            eval,
        }
    }

    fn tile(&mut self, t: usize) -> &TileMatch {
        if self.tiles[t].is_none() {
            let rect = self.grid.rect(t);
            self.tiles[t] = Some((self.eval)(t, rect));
        }
        self.tiles[t].as_ref().expect("evaluated above")
    }

    /// The number of tiles evaluated so far (tests).
    pub fn evaluated(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }
}

impl<F> FloodSource for MatchTiles<F>
where
    F: FnMut(usize, (usize, usize, usize, usize)) -> TileMatch,
{
    #[inline]
    fn matches(&mut self, _from: (usize, usize), (x, y): (usize, usize)) -> bool {
        let t = self.grid.tile_of(x, y);
        self.tile(t).get(x % TILE, y % TILE)
    }

    fn whole(&mut self, tile: usize) -> bool {
        matches!(self.tile(tile), TileMatch::All)
    }
}

/// Every pixel of a `w × h` grid that `eval` matches, without connectivity:
/// the global (non-contiguous) Paint Bucket and Magic Wand. One row of tiles
/// is evaluated at a time, on every core, so the answers held at once are one
/// band's.
pub fn match_mask(
    w: usize,
    h: usize,
    eval: impl Fn(usize, (usize, usize, usize, usize)) -> TileMatch + Sync,
) -> TiledMask {
    let grid = Grid::new(w, h);
    let mut mask = Plane::filled(w, h, 1, 0u8);
    for ty in 0..grid.rows {
        let band: Vec<(usize, TileMatch)> = (0..grid.cols)
            .into_par_iter()
            .map(|tx| {
                let t = ty * grid.cols + tx;
                (t, eval(t, grid.rect(t)))
            })
            .collect();
        let mut dense = Vec::new();
        for (t, m) in band {
            match m {
                TileMatch::None => {}
                TileMatch::All => {
                    mask.set_tile_uniform(t, 255);
                }
                TileMatch::Bits(bits) => dense.push((t, bits)),
            }
        }
        write_bits(&mut mask, dense);
    }
    mask
}

/// Write each `(tile, bits)` into `mask` as 0 / 255 bytes, then settle.
fn write_bits(mask: &mut TiledMask, dense: Vec<(usize, TileBits)>) {
    if dense.is_empty() {
        return;
    }
    let indices: Vec<usize> = dense.iter().map(|(t, _)| *t).collect();
    let tiles = mask.tiles_mut_at(&indices);
    tiles
        .into_par_iter()
        .zip(dense.par_iter())
        .for_each(|(mut tile, (_, bits))| {
            for dy in 0..tile.h {
                let row = tile.row_mut(dy);
                for (dx, px) in row.iter_mut().enumerate() {
                    *px = if get_bit(bits, bit_index(dx, dy)) {
                        255
                    } else {
                        0
                    };
                }
            }
        });
    mask.settle();
}

/// The flood's visited state: a bitmap for each tile it marked a pixel in,
/// and a flag for each tile it reached whole.
struct Visited {
    grid: Grid,
    bits: Vec<Option<TileBits>>,
    full: Vec<bool>,
    /// `whole` answers: 0 unknown, 1 yes, 2 no.
    known: Vec<u8>,
    /// Tiles reached whole whose edges are still to spread from.
    pending: Vec<usize>,
}

impl Visited {
    fn new(grid: Grid) -> Self {
        let n = grid.count();
        Self {
            grid,
            bits: (0..n).map(|_| None).collect(),
            full: vec![false; n],
            known: vec![0; n],
            pending: Vec::new(),
        }
    }

    #[inline]
    fn marked(&self, x: usize, y: usize) -> bool {
        let t = self.grid.tile_of(x, y);
        self.full[t]
            || self.bits[t]
                .as_ref()
                .is_some_and(|b| get_bit(b, bit_index(x % TILE, y % TILE)))
    }

    /// Reach the whole of tile `t`.
    fn fill(&mut self, t: usize) {
        if !self.full[t] {
            self.full[t] = true;
            self.bits[t] = None;
            self.pending.push(t);
        }
    }

    #[inline]
    fn mark(&mut self, x: usize, y: usize, source: &mut impl FloodSource) {
        let t = self.grid.tile_of(x, y);
        if self.full[t] {
            return;
        }
        if self.known[t] == 0 {
            self.known[t] = if source.whole(t) { 1 } else { 2 };
        }
        if self.known[t] == 1 {
            self.fill(t);
            return;
        }
        let bits = self.bits[t].get_or_insert_with(new_bits);
        set_bit(bits, bit_index(x % TILE, y % TILE));
    }

    /// The tiles holding visited state (tests).
    #[cfg(test)]
    fn touched(&self) -> usize {
        (0..self.full.len())
            .filter(|&t| self.full[t] || self.bits[t].is_some())
            .count()
    }
}

/// Grow the marked run `start..end` of row `y` left and right over the pixels
/// `source` accepts; the run it reached.
fn widen(
    v: &mut Visited,
    source: &mut impl FloodSource,
    y: usize,
    start: usize,
    end: usize,
) -> (usize, usize) {
    let w = v.grid.w;
    let mut left = start;
    while left > 0 && !v.marked(left - 1, y) && source.matches((left, y), (left - 1, y)) {
        v.mark(left - 1, y, source);
        left -= 1;
    }
    let mut right = end;
    while right < w && !v.marked(right, y) && source.matches((right - 1, y), (right, y)) {
        v.mark(right, y, source);
        right += 1;
    }
    (left, right)
}

/// Spread out of tile `t`, reached whole: its top and bottom rows go on the
/// stack for the rows beyond them, and each row widens out of its sides.
fn edges(
    v: &mut Visited,
    source: &mut impl FloodSource,
    stack: &mut Vec<(usize, usize, usize)>,
    t: usize,
) {
    let (x0, y0, tw, th) = v.grid.rect(t);
    let x1 = x0 + tw;
    stack.push((y0, x0, x1));
    if th > 1 {
        stack.push((y0 + th - 1, x0, x1));
    }
    for y in y0..y0 + th {
        let (left, right) = widen(v, source, y, x0, x1);
        if left < x0 {
            stack.push((y, left, x0));
        }
        if right > x1 {
            stack.push((y, x1, right));
        }
    }
}

/// Mark the seeds and push their widened runs; `false` when there is none.
fn seed(
    v: &mut Visited,
    source: &mut impl FloodSource,
    stack: &mut Vec<(usize, usize, usize)>,
    seeds: Seeds<'_>,
) -> bool {
    let grid = v.grid;
    match seeds {
        Seeds::Pixel(x, y) => {
            if x >= grid.w || y >= grid.h {
                return false;
            }
            v.mark(x, y, source);
            let (left, right) = widen(v, source, y, x, x + 1);
            stack.push((y, left, right));
            true
        }
        Seeds::Mask(mask) => {
            let mut any = false;
            let mut row = vec![0u8; TILE];
            for t in 0..grid.count() {
                let (x0, y0, tw, th) = grid.rect(t);
                match mask.uniform_over(x0..x0 + tw, y0..y0 + th) {
                    Some(0) => continue,
                    Some(_) => {
                        any = true;
                        v.fill(t);
                        continue;
                    }
                    None => {}
                }
                for y in y0..y0 + th {
                    let row = &mut row[..tw];
                    row.fill(0);
                    mask.read_rect_into(x0..x0 + tw, y..y + 1, grid.w, row);
                    let mut dx = 0;
                    while dx < tw {
                        if row[dx] == 0 || v.marked(x0 + dx, y) {
                            dx += 1;
                            continue;
                        }
                        let start = dx;
                        while dx < tw && row[dx] != 0 {
                            v.mark(x0 + dx, y, source);
                            dx += 1;
                        }
                        any = true;
                        let (left, right) = widen(v, source, y, x0 + start, x0 + dx);
                        stack.push((y, left, right));
                        dx = right.saturating_sub(x0).max(dx);
                    }
                }
            }
            any
        }
    }
}

/// Run the flood to its end from the seeded `stack`.
fn spread(v: &mut Visited, source: &mut impl FloodSource, stack: &mut Vec<(usize, usize, usize)>) {
    let (w, h) = (v.grid.w, v.grid.h);
    loop {
        if let Some(t) = v.pending.pop() {
            edges(v, source, stack, t);
            continue;
        }
        let Some((y, x0, x1)) = stack.pop() else {
            break;
        };
        for ny in [y.checked_sub(1), (y + 1 < h).then_some(y + 1)]
            .into_iter()
            .flatten()
        {
            let mut x = x0;
            while x < x1 {
                if v.marked(x, ny) || !source.matches((x, y), (x, ny)) {
                    x += 1;
                    continue;
                }
                v.mark(x, ny, source);
                let mut left = x;
                while left > 0
                    && !v.marked(left - 1, ny)
                    && source.matches((left, ny), (left - 1, ny))
                {
                    v.mark(left - 1, ny, source);
                    left -= 1;
                }
                let mut right = x;
                while right + 1 < w
                    && !v.marked(right + 1, ny)
                    && source.matches((right, ny), (right + 1, ny))
                {
                    v.mark(right + 1, ny, source);
                    right += 1;
                }
                stack.push((ny, left, right + 1));
                x = right + 1;
            }
        }
    }
}

/// A 0 / 255 mask of the pixels four-connected to `seeds` that `source`
/// accepts, the seeds included: [`flood`](super::flood) over the tile grid.
/// Tiles reached whole are `Uniform(255)`, tiles never reached `Empty`.
/// `None` when no seed lies in the plane.
pub fn flood_tiled(
    w: usize,
    h: usize,
    seeds: Seeds<'_>,
    source: &mut impl FloodSource,
) -> Option<TiledMask> {
    flood_visited(w, h, seeds, source).map(|(mask, _)| mask)
}

fn flood_visited(
    w: usize,
    h: usize,
    seeds: Seeds<'_>,
    source: &mut impl FloodSource,
) -> Option<(TiledMask, usize)> {
    if w == 0 || h == 0 {
        return None;
    }
    let mut v = Visited::new(Grid::new(w, h));
    let mut stack = Vec::new();
    if !seed(&mut v, source, &mut stack, seeds) {
        return None;
    }
    spread(&mut v, source, &mut stack);
    #[cfg(test)]
    let touched = v.touched();
    #[cfg(not(test))]
    let touched = 0;
    let mut mask = Plane::filled(w, h, 1, 0u8);
    let mut dense = Vec::new();
    for (t, (full, bits)) in v.full.iter().zip(v.bits.iter_mut()).enumerate() {
        if *full {
            mask.set_tile_uniform(t, 255);
        } else if let Some(bits) = bits.take() {
            dense.push((t, bits));
        }
    }
    write_bits(&mut mask, dense);
    Some((mask, touched))
}

/// As [`flood_tiled`], also returning how many tiles held visited state.
#[cfg(test)]
pub(crate) fn flood_tiled_touched(
    w: usize,
    h: usize,
    seeds: Seeds<'_>,
    source: &mut impl FloodSource,
) -> Option<(TiledMask, usize)> {
    flood_visited(w, h, seeds, source)
}
