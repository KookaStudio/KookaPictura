//! Selection coverage as a sparse tiled mask (#1214): shapes rasterized into
//! tiles, and tile-wise maps and combines that decide a uniform tile without
//! reading its samples.

use std::ops::Range;

use pictura_core::mask_ops::{empty_mask, Grid, TiledMask};
pub(crate) use pictura_core::mask_ops::{tiled_view, zip_tiles};
use pictura_core::{PsdRect, TILE};
use rayon::prelude::*;

/// One band's rasterized tiles and the bounds of its runs.
type Band = (Vec<(usize, Content)>, Option<PsdRect>);

/// A tile's content as a band rasterizes it.
enum Content {
    Uniform(u8),
    Dense(Vec<u8>),
}

/// A `w × h` mask whose rows `rows` hold, at 255, the half-open runs
/// `runs(y, out)` pushes (ascending, disjoint, inside `0..w`), and the tight
/// bounds of those runs. A tile every run covers whole is `Uniform(255)`, a
/// tile no run meets stays `Empty`; only the tiles along the shape's edge are
/// dense. Bands of tile rows run on every core.
pub(crate) fn rasterize(
    w: usize,
    h: usize,
    rows: Range<usize>,
    runs: impl Fn(usize, &mut Vec<(usize, usize)>) + Sync,
) -> (TiledMask, Option<PsdRect>) {
    let mut mask = empty_mask(w, h);
    if w == 0 || h == 0 {
        return (mask, None);
    }
    let grid = Grid::new(w, h);
    let rows = rows.start.min(h)..rows.end.min(h);
    let bands = (rows.start / TILE)..rows.end.div_ceil(TILE);
    let found: Vec<Band> = bands
        .into_par_iter()
        .map(|ty| rasterize_band(&grid, ty, rows.clone(), &runs))
        .collect();
    let mut bounds: Option<PsdRect> = None;
    let mut dense = Vec::new();
    for (tiles, b) in found {
        if let Some(b) = b {
            bounds = Some(bounds.map_or(b, |a| a.union(b)));
        }
        for (t, content) in tiles {
            match content {
                Content::Uniform(v) => {
                    mask.set_tile_uniform(t, v);
                }
                Content::Dense(bytes) => dense.push((t, bytes)),
            }
        }
    }
    write_dense(&mut mask, dense);
    (mask, bounds)
}

/// One band of [`rasterize`]: each tile it touches and its content.
fn rasterize_band(
    grid: &Grid,
    ty: usize,
    rows: Range<usize>,
    runs: &(impl Fn(usize, &mut Vec<(usize, usize)>) + Sync),
) -> Band {
    let y0 = ty * TILE;
    let th = TILE.min(grid.h - y0);
    let mut per_row: Vec<Vec<(usize, usize)>> = vec![Vec::new(); th];
    let mut full = vec![0usize; grid.cols];
    let mut partial = vec![false; grid.cols];
    let mut bounds: Option<PsdRect> = None;
    for (dy, row_runs) in per_row.iter_mut().enumerate() {
        let y = y0 + dy;
        if !rows.contains(&y) {
            continue;
        }
        runs(y, row_runs);
        row_runs.retain(|&(s, e)| s < e && e <= grid.w);
        for &(s, e) in row_runs.iter() {
            let span = PsdRect {
                top: y as i32,
                left: s as i32,
                bottom: y as i32 + 1,
                right: e as i32,
            };
            bounds = Some(bounds.map_or(span, |b| b.union(span)));
            for tx in s / TILE..=(e - 1) / TILE {
                let (xt0, xt1) = (tx * TILE, ((tx + 1) * TILE).min(grid.w));
                if s <= xt0 && e >= xt1 {
                    full[tx] += 1;
                } else {
                    partial[tx] = true;
                }
            }
        }
    }
    let mut out = Vec::new();
    for tx in 0..grid.cols {
        let t = ty * grid.cols + tx;
        if !partial[tx] && full[tx] == th {
            out.push((t, Content::Uniform(255)));
            continue;
        }
        if !partial[tx] && full[tx] == 0 {
            continue;
        }
        let (xt0, xt1) = (tx * TILE, ((tx + 1) * TILE).min(grid.w));
        let tw = xt1 - xt0;
        let mut bytes = vec![0u8; tw * th];
        for (dy, row_runs) in per_row.iter().enumerate() {
            for &(s, e) in row_runs {
                let (a, b) = (s.max(xt0), e.min(xt1));
                if a < b {
                    bytes[dy * tw + a - xt0..dy * tw + b - xt0].fill(255);
                }
            }
        }
        out.push((t, Content::Dense(bytes)));
    }
    (out, bounds)
}

/// Write each `(tile, bytes)` (the clipped tile, row-major) into `mask`, then
/// settle.
fn write_dense(mask: &mut TiledMask, dense: Vec<(usize, Vec<u8>)>) {
    if dense.is_empty() {
        return;
    }
    let indices: Vec<usize> = dense.iter().map(|(t, _)| *t).collect();
    mask.tiles_mut_at(&indices)
        .into_par_iter()
        .zip(dense.par_iter())
        .for_each(|(mut tile, (_, bytes))| {
            let tw = tile.w;
            for dy in 0..tile.h {
                tile.row_mut(dy)
                    .copy_from_slice(&bytes[dy * tw..dy * tw + tw]);
            }
        });
    mask.settle();
}

/// `f` of every sample of a `w × h` mask, as a new tiled mask; a uniform tile
/// maps without a sample read.
pub(crate) fn map_tiles(
    a: &TiledMask,
    w: usize,
    h: usize,
    f: impl Fn(u8) -> u8 + Sync,
) -> TiledMask {
    let a = tiled_view(a, w, h);
    zip_tiles(&a, &a, w, h, |x, _| f(x))
}

/// `src` (`w × h`) moved by `(dx, dy)`, clipped to the canvas: tile by tile,
/// a destination tile whose source is one value whole is that value.
pub(crate) fn translate_tiles(src: &TiledMask, w: usize, h: usize, dx: i64, dy: i64) -> TiledMask {
    let grid = Grid::new(w, h);
    let mut out = empty_mask(w, h);
    let (wi, hi) = (w as i64, h as i64);
    let mut buf = Vec::new();
    for t in 0..grid.count() {
        let (x0, y0, tw, th) = grid.rect(t);
        // The destination part whose source lies on the canvas.
        let vx0 = (x0 as i64).max(dx);
        let vx1 = ((x0 + tw) as i64).min(wi + dx);
        let vy0 = (y0 as i64).max(dy);
        let vy1 = ((y0 + th) as i64).min(hi + dy);
        if vx0 >= vx1 || vy0 >= vy1 {
            continue;
        }
        let sx = (vx0 - dx) as usize..(vx1 - dx) as usize;
        let sy = (vy0 - dy) as usize..(vy1 - dy) as usize;
        let whole = (vx1 - vx0) as usize == tw && (vy1 - vy0) as usize == th;
        if let Some(v) = src.uniform_over(sx.clone(), sy.clone()) {
            if whole {
                out.set_tile_uniform(t, v);
                continue;
            }
            if v == 0 {
                continue;
            }
        }
        buf.clear();
        buf.resize(sx.len() * sy.len(), 0);
        src.read_rect_into(sx, sy, w, &mut buf);
        out.write_rect_from(
            vx0 as usize..vx1 as usize,
            vy0 as usize..vy1 as usize,
            w,
            &buf,
        );
    }
    out.settle();
    out
}

/// The samples of channel `k` of a planar `w × h` buffer over a tile, one
/// value when its tiles say so.
pub(crate) fn channel_uniform(
    data: &TiledMask,
    h: usize,
    k: usize,
    (x0, y0, tw, th): (usize, usize, usize, usize),
) -> Option<u8> {
    data.uniform_over(x0..x0 + tw, k * h + y0..k * h + y0 + th)
}

/// Channel `k` of a planar `w × h` buffer over a tile, row-major.
pub(crate) fn channel_tile(
    data: &TiledMask,
    w: usize,
    h: usize,
    k: usize,
    (x0, y0, tw, th): (usize, usize, usize, usize),
) -> Vec<u8> {
    let mut out = vec![0u8; tw * th];
    data.read_rect_into(x0..x0 + tw, k * h + y0..k * h + y0 + th, w, &mut out);
    out
}

#[cfg(test)]
mod tests;
