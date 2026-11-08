//! Neighbourhood operations and scans of a [`TiledMask`] that never hold the
//! whole mask contiguously (#1214).

use std::borrow::Cow;

use rayon::prelude::*;

use super::flood_tiled::Grid;
use super::{empty_mask, TiledMask};
use crate::tiled_plane::TILE;
use crate::Plane;

/// Run a neighbourhood kernel over `mask` (`w × h`) one row of tiles at a
/// time, returning a tiled mask.
///
/// `f(window, ww, wh)` computes a `ww × wh` window and returns its result at
/// the same size. Each band's window is the band's tiles grown by `pad`
/// rows and columns, clipped to the canvas, so a kernel that reads at most
/// `pad - 1` pixels away and treats the canvas edge itself gives every centre
/// pixel the whole-mask answer. A tile whose `pad`-grown window holds one value
/// is that value without running `f` when the window lies inside the canvas,
/// or when the value is zero: the kernels in use (morphology, median, blur)
/// map a constant to itself, and a zero window to zero at any border.
pub fn map_bands(
    mask: &TiledMask,
    w: usize,
    h: usize,
    pad: usize,
    f: impl Fn(&[u8], usize, usize) -> Vec<u8> + Sync,
) -> TiledMask {
    let grid = Grid::new(w, h);
    let mut out = Plane::filled(w, h, 1, 0u8);
    if w == 0 || h == 0 {
        return out;
    }
    // Past `pad` from every covered pixel a kernel reads only zeros and
    // gives zero, so only the coverage's box grown by `pad` is computed.
    let Some((bx0, by0, bx1, by1)) = coverage_bounds(mask, w, h) else {
        return out;
    };
    let (cx0, cy0) = (bx0.saturating_sub(pad), by0.saturating_sub(pad));
    let (cx1, cy1) = ((bx1 + pad).min(w), (by1 + pad).min(h));
    // The runs of tiles to compute in each band of tile rows.
    let mut bands: Vec<Band> = Vec::new();
    for ty in 0..grid.rows {
        let mut compute = Vec::new();
        for tx in 0..grid.cols {
            let t = ty * grid.cols + tx;
            let (x0, y0, tw, th) = grid.rect(t);
            // The window clipped to the canvas. A window of zeros stays zero
            // whatever the kernel does at the canvas edge; any other value is
            // kept only when no edge is in reach.
            let inside = x0 >= pad && y0 >= pad && x0 + tw + pad <= w && y0 + th + pad <= h;
            let xs = x0.saturating_sub(pad)..(x0 + tw + pad).min(w);
            let ys = y0.saturating_sub(pad)..(y0 + th + pad).min(h);
            match mask.uniform_over(xs, ys) {
                Some(0) => continue,
                Some(v) if inside => {
                    out.set_tile_uniform(t, v);
                    continue;
                }
                _ => {}
            }
            compute.push(tx);
        }
        if compute.is_empty() {
            continue;
        }
        let (y0, y1) = ((ty * TILE).max(cy0), ((ty + 1) * TILE).min(h).min(cy1));
        let runs: Vec<(usize, usize)> = runs(&compute)
            .into_iter()
            .map(|(first, last)| ((first * TILE).max(cx0), ((last + 1) * TILE).min(w).min(cx1)))
            .filter(|(x0, x1)| x0 < x1)
            .collect();
        if y0 >= y1 || runs.is_empty() {
            continue;
        }
        bands.push(Band {
            y0,
            rows: y1 - y0,
            runs,
        });
    }
    // Neighbouring bands share one window while it stays small: each window
    // repeats `pad` rows of its neighbours, so a short one wastes work, and
    // the cap keeps what a window holds bounded rather than region-sized.
    let windows = merge(bands, pad, w, h);
    let mut start = 0;
    while start < windows.len() {
        // As many windows at once as the cores take, within the cap.
        let mut end = start + 1;
        let mut held = windows[start].pixels(pad, w, h);
        while end < windows.len() && held + windows[end].pixels(pad, w, h) <= IN_FLIGHT_PX {
            held += windows[end].pixels(pad, w, h);
            end += 1;
        }
        let parts: Vec<Vec<Part>> = windows[start..end]
            .par_iter()
            .map(|win| win.compute(mask, pad, w, h, &f))
            .collect();
        for (x0, x1, y0, rows, centre) in parts.into_iter().flatten() {
            out.write_rect_from(x0..x1, y0..y0 + rows, w, &centre);
        }
        start = end;
    }
    out.settle();
    out
}

/// The most pixels one [`map_bands`] window holds.
const WINDOW_PX: usize = 8 << 20;

/// The most window pixels [`map_bands`] computes at once.
const IN_FLIGHT_PX: usize = 24 << 20;

/// One row of tiles to compute: its first row, its rows, and the column runs
/// `(x0, x1)` whose tiles need the kernel.
struct Band {
    y0: usize,
    rows: usize,
    runs: Vec<(usize, usize)>,
}

/// One centre written back: columns `x0..x1` of rows `y0..y0 + rows`.
type Part = (usize, usize, usize, usize, Vec<u8>);

/// Consecutive bands computed from one window.
struct Window {
    bands: Vec<Band>,
    x0: usize,
    x1: usize,
}

impl Window {
    fn rows(&self) -> (usize, usize) {
        let first = &self.bands[0];
        let last = &self.bands[self.bands.len() - 1];
        (first.y0, last.y0 + last.rows)
    }

    /// The pixels of the window grown by `pad`, clipped to the canvas.
    fn pixels(&self, pad: usize, w: usize, h: usize) -> usize {
        let (y0, y1) = self.rows();
        let cols = (self.x1 + pad).min(w) - self.x0.saturating_sub(pad);
        let rows = (y1 + pad).min(h) - y0.saturating_sub(pad);
        cols * rows
    }

    /// Run `f` over the window and cut out each band's runs.
    fn compute(
        &self,
        mask: &TiledMask,
        pad: usize,
        w: usize,
        h: usize,
        f: &(impl Fn(&[u8], usize, usize) -> Vec<u8> + Sync),
    ) -> Vec<Part> {
        let (y0, y1) = self.rows();
        let (wy0, wy1) = (y0.saturating_sub(pad), (y1 + pad).min(h));
        let (wx0, wx1) = (self.x0.saturating_sub(pad), (self.x1 + pad).min(w));
        let (ww, wh) = (wx1 - wx0, wy1 - wy0);
        let mut src = vec![0u8; ww * wh];
        mask.read_rect_into(wx0..wx1, wy0..wy1, w, &mut src);
        let result = f(&src, ww, wh);
        drop(src);
        let mut parts = Vec::new();
        for band in &self.bands {
            for &(x0, x1) in &band.runs {
                let cw = x1 - x0;
                let mut centre = vec![0u8; cw * band.rows];
                for (dy, dst) in centre.chunks_exact_mut(cw).enumerate() {
                    let at = (band.y0 - wy0 + dy) * ww + (x0 - wx0);
                    dst.copy_from_slice(&result[at..at + cw]);
                }
                parts.push((x0, x1, band.y0, band.rows, centre));
            }
        }
        parts
    }
}

/// Group consecutive bands into windows of at most [`WINDOW_PX`] pixels.
fn merge(bands: Vec<Band>, pad: usize, w: usize, h: usize) -> Vec<Window> {
    let mut windows: Vec<Window> = Vec::new();
    for band in bands {
        let (bx0, bx1) = (band.runs[0].0, band.runs[band.runs.len() - 1].1);
        if let Some(win) = windows.last_mut() {
            let adjacent = win.rows().1 == band.y0;
            let grown = Window {
                bands: Vec::new(),
                x0: win.x0.min(bx0),
                x1: win.x1.max(bx1),
            };
            let (y0, _) = win.rows();
            let cols = (grown.x1 + pad).min(w) - grown.x0.saturating_sub(pad);
            let rows = (band.y0 + band.rows + pad).min(h) - y0.saturating_sub(pad);
            if adjacent && cols * rows <= WINDOW_PX {
                win.x0 = grown.x0;
                win.x1 = grown.x1;
                win.bands.push(band);
                continue;
            }
        }
        windows.push(Window {
            x0: bx0,
            x1: bx1,
            bands: vec![band],
        });
    }
    windows
}

/// The runs of consecutive values in an ascending list, as `(first, last)`.
fn runs(sorted: &[usize]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &v in sorted {
        match out.last_mut() {
            Some((_, last)) if *last + 1 == v => *last = v,
            _ => out.push((v, v)),
        }
    }
    out
}

/// The first covered (`> 0`) sample of `row`, eight bytes at a time so an
/// empty row costs one word compare per eight pixels.
pub fn first_covered(row: &[u8]) -> Option<usize> {
    let (chunks, tail) = row.as_chunks::<8>();
    for (i, chunk) in chunks.iter().enumerate() {
        if u64::from_le_bytes(*chunk) != 0 {
            return Some(i * 8 + chunk.iter().position(|&v| v > 0)?);
        }
    }
    tail.iter()
        .position(|&v| v > 0)
        .map(|p| chunks.len() * 8 + p)
}

/// The last covered (`> 0`) sample of `row`, from the end.
pub fn last_covered(row: &[u8]) -> Option<usize> {
    let (chunks, tail) = row.as_chunks::<8>();
    if let Some(p) = tail.iter().rposition(|&v| v > 0) {
        return Some(chunks.len() * 8 + p);
    }
    for (i, chunk) in chunks.iter().enumerate().rev() {
        if u64::from_le_bytes(*chunk) != 0 {
            return Some(i * 8 + chunk.iter().rposition(|&v| v > 0)?);
        }
    }
    None
}

/// The box `(x0, y0, x1, y1)` (half-open) of the covered pixels of a `w × h`
/// mask, or `None` when it covers nothing. A tiled mask skips its `Empty`
/// tiles and takes a covered `Uniform` tile whole; a flat one is scanned row
/// by row.
pub fn coverage_bounds(
    mask: &TiledMask,
    w: usize,
    h: usize,
) -> Option<(usize, usize, usize, usize)> {
    if w == 0 || h == 0 || mask.len() < w * h {
        return None;
    }
    let mut found: Option<(usize, usize, usize, usize)> = None;
    let mut add = |x0: usize, y0: usize, x1: usize, y1: usize| {
        found = Some(match found {
            None => (x0, y0, x1, y1),
            Some((a, b, c, d)) => (a.min(x0), b.min(y0), c.max(x1), d.max(y1)),
        });
    };
    if mask.geometry() == Some((w, h, 1)) {
        let grid = Grid::new(w, h);
        for t in 0..grid.count() {
            let (x0, y0, tw, th) = grid.rect(t);
            let Some(tile) = mask.tile_at(t) else {
                continue;
            };
            match tile.uniform() {
                Some(0) => {}
                Some(_) => add(x0, y0, x0 + tw, y0 + th),
                None => {
                    for dy in 0..th {
                        let row = tile.row(dy).unwrap_or(&[]);
                        if let Some(first) = first_covered(row) {
                            let last = last_covered(row).unwrap_or(first);
                            add(x0 + first, y0 + dy, x0 + last + 1, y0 + dy + 1);
                        }
                    }
                }
            }
        }
        return found;
    }
    for y in 0..h {
        let Some(row) = mask.row(y, w) else {
            break;
        };
        if let Some(first) = first_covered(&row) {
            let last = last_covered(&row).unwrap_or(first);
            add(first, y, last + 1, y + 1);
        }
    }
    found
}

/// `plane` tiled as a `w × h` mask: borrowed when it already is, otherwise a
/// tiled copy (a flat literal or a channel loaded flat).
pub fn tiled_view(plane: &TiledMask, w: usize, h: usize) -> Cow<'_, TiledMask> {
    if plane.geometry() == Some((w, h, 1)) {
        return Cow::Borrowed(plane);
    }
    let mut copy = plane.clone();
    if !copy.tile(w, h) {
        return Cow::Owned(empty_mask(w, h));
    }
    Cow::Owned(copy)
}

/// `f` of `a` and `b` (two `w × h` masks) per pixel, as a new tiled mask. A
/// tile uniform in both is `Uniform(f(a, b))` without a sample read; the rest
/// are computed on every core and settled.
pub fn zip_tiles(
    a: &TiledMask,
    b: &TiledMask,
    w: usize,
    h: usize,
    f: impl Fn(u8, u8) -> u8 + Sync,
) -> TiledMask {
    let (a, b) = (tiled_view(a, w, h), tiled_view(b, w, h));
    let grid = Grid::new(w, h);
    let mut out = empty_mask(w, h);
    let mut dense = Vec::new();
    for t in 0..grid.count() {
        let (ta, tb) = (a.tile_at(t), b.tile_at(t));
        match (ta.and_then(|x| x.uniform()), tb.and_then(|x| x.uniform())) {
            (Some(x), Some(y)) => {
                out.set_tile_uniform(t, f(x, y));
            }
            _ => dense.push(t),
        }
    }
    if !dense.is_empty() {
        out.tiles_mut_at(&dense)
            .into_par_iter()
            .zip(dense.par_iter())
            .for_each(|(mut tile, &t)| {
                let (ta, tb) = (a.tile_at(t), b.tile_at(t));
                let (Some(ta), Some(tb)) = (ta, tb) else {
                    return;
                };
                for dy in 0..tile.h {
                    let row = tile.row_mut(dy);
                    for (dx, px) in row.iter_mut().enumerate() {
                        *px = f(ta.get(dx, dy), tb.get(dx, dy));
                    }
                }
            });
        out.settle();
    }
    out
}
