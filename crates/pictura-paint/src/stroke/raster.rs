//! The tiled dab raster: a dab's box cut into the layer's 256² tiles, which
//! run in parallel, each writing only its own tile of the coverage planes and
//! the layer planes.

use std::ops::Range;

use super::{PlaneIndex, Stencil};
use crate::healing::RgbaImage;
use crate::tip::TipParams;
use crate::PaintMode;
use pictura_core::{Layer, Plane, PsdRect, TileMut};
use rayon::prelude::*;

/// The target layer's R, G, B and alpha samples of one tile, in that order.
pub(super) type CellPlanes<'a> = [Option<&'a mut [u8]>; 4];

/// One tile of the layer under a dab: the coverage samples and the plane
/// samples, each `w × h` with stride `w`, at layer-local `(x, y)`. Disjoint
/// from every other cell's.
pub(super) struct Cell<'a> {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) scratch: &'a mut [u8],
    applied: &'a mut [u8],
    pub(super) planes: CellPlanes<'a>,
}

/// One dab's raster inputs, shared by its cells.
pub(super) struct DabRaster<'a> {
    pub(super) params: &'a TipParams,
    pub(super) dab_x: f32,
    pub(super) dab_y: f32,
    pub(super) flow: f32,
    pub(super) opacity: f32,
    pub(super) paint: crate::Rgba,
    pub(super) width: usize,
    pub(super) stencil: &'a Stencil<'a>,
}

impl DabRaster<'_> {
    /// Raster the dab over every cell, returning the layer-local box of the
    /// pixels whose applied coverage changed. With `rng` (Dissolve), the
    /// pixels are visited in raster order across the cells so each roll
    /// lands where the serial raster put it; otherwise the cells run in
    /// parallel, each cut to the strips that meet the dab's rows `ys`.
    pub(super) fn raster(
        &self,
        mut cells: Vec<Cell<'_>>,
        ys: Range<usize>,
        rng: Option<&mut u64>,
    ) -> Option<PsdRect> {
        let Some(rng) = rng else {
            let tasks: Vec<Cell<'_>> = cells
                .into_iter()
                .flat_map(|cell| split_rows(cell, &ys))
                .collect();
            if tasks.len() == 1 {
                // One task runs inline: a small dab is not worth a thread hop.
                return tasks.into_iter().find_map(|mut cell| {
                    (0..cell.h)
                        .filter_map(|r| self.row(&mut cell, r, None))
                        .reduce(PsdRect::union)
                });
            }
            return tasks
                .into_par_iter()
                .filter_map(|mut cell| {
                    (0..cell.h)
                        .filter_map(|r| self.row(&mut cell, r, None))
                        .reduce(PsdRect::union)
                })
                .reduce_with(PsdRect::union);
        };
        let mut bbox: Option<PsdRect> = None;
        for strip in cells.chunk_by_mut(|a, b| a.y == b.y) {
            for r in 0..strip[0].h {
                for cell in strip.iter_mut() {
                    if let Some(b) = self.row(cell, r, Some(&mut *rng)) {
                        bbox = Some(bbox.map_or(b, |a| a.union(b)));
                    }
                }
            }
        }
        bbox
    }

    /// Raster row `r` of `cell`, returning the box of the pixels that changed.
    fn row(&self, cell: &mut Cell<'_>, r: usize, mut rng: Option<&mut u64>) -> Option<PsdRect> {
        let w = self.width;
        let ly = cell.y + r;
        let dy = (ly as f32 + 0.5) - self.dab_y;
        // A round tip's support on this row is a chord, so the corners of its
        // square bounding box cost nothing. A non-round shape returns the full
        // width and its out-of-support pixels fall to the `cov <= 0.0` filter
        // below.
        let (xlo, xhi) = self.params.x_span(dy)?;
        let x0 = ((self.dab_x + xlo).floor() as i32).max(cell.x as i32);
        let x1 = ((self.dab_x + xhi).ceil() as i32)
            .min(w as i32 - 1)
            .min((cell.x + cell.w) as i32 - 1);
        let mut first = None;
        let mut last = 0;
        for lx in x0..=x1 {
            let dx = (lx as f32 + 0.5) - self.dab_x;
            let cov = self.params.coverage(dx, dy);
            if cov <= 0.0 {
                continue;
            }
            let at = r * cell.w + (lx as usize - cell.x);
            let acc = 1.0 - (1.0 - cell.scratch[at] as f32 / 255.0) * (1.0 - self.flow * cov);
            let v = (acc.clamp(0.0, 1.0) * 255.0).round() as u8;
            cell.scratch[at] = v;
            let cfg = &self.stencil.cfg;
            let v = if cfg.wet_edges { crate::wet_edge(v) } else { v };
            // The pencil's paper grain: a fixed alpha modulation keyed to the
            // layer coordinates. It dims the stroke by up to `grain` % where the
            // noise is lowest instead of dropping pixels to zero, which would
            // scatter the stroke into isolated dots; the pattern is fixed, so
            // overlapping dabs only deepen it. The noise is clumped at 2–3 px so
            // the stroke reads as pencil texture rather than static.
            let v = if cfg.grain > 0 {
                let noise = crate::fill::grain_noise(lx, ly as i32);
                let keep = 1.0 - cfg.grain as f32 / 100.0 * (1.0 - noise);
                (v as f32 * keep).round() as u8
            } else {
                v
            };
            if v == cell.applied[at] {
                continue;
            }
            cell.applied[at] = v;
            let i = ly * w + lx as usize;
            if !self.stencil.composite(
                &mut cell.planes,
                at,
                i,
                rng.as_deref_mut(),
                v,
                self.paint,
                self.opacity,
            ) {
                continue;
            }
            first.get_or_insert(lx);
            last = lx;
        }
        first.map(|first| PsdRect {
            top: ly as i32,
            left: first,
            bottom: ly as i32 + 1,
            right: last + 1,
        })
    }
}

/// The cells of the layer's tiles meeting columns `xs` × rows `ys`: the
/// coverage planes' tiles beside the target planes', all forked for writing, so
/// a dab forks only the tiles it touches. The layer planes are tiled first.
#[allow(clippy::too_many_arguments)]
pub(super) fn cells<'a>(
    layer: &'a mut Layer,
    ch: PlaneIndex,
    width: usize,
    height: usize,
    xs: Range<usize>,
    ys: Range<usize>,
    scratch: &'a mut Plane<u8>,
    applied: &'a mut Plane<u8>,
) -> Vec<Cell<'a>> {
    let len = width * height;
    let mut planes: [Option<std::vec::IntoIter<TileMut<'a, u8>>>; 4] = [None, None, None, None];
    for (idx, c) in layer.channels.iter_mut().enumerate() {
        let Some(k) = ch.iter().position(|&x| x == Some(idx)) else {
            continue;
        };
        if c.data.len() == len && c.data.tile(width, height) {
            planes[k] = Some(c.data.tiles_mut(xs.clone(), ys.clone(), width).into_iter());
        }
    }
    let coverage = scratch.tiles_mut(xs.clone(), ys.clone(), width);
    let applied = applied.tiles_mut(xs, ys, width);
    coverage
        .into_iter()
        .zip(applied)
        .map(|(s, a)| Cell {
            x: s.x,
            y: s.y,
            w: s.w,
            h: s.h,
            scratch: s.data,
            applied: a.data,
            planes: planes
                .each_mut()
                .map(|it| it.as_mut().and_then(|it| it.next()).map(|t| t.data)),
        })
        .collect()
}

/// Rows per parallel task: a tile is cut into strips so a dab a few tiles
/// wide still spreads over every core, as the 64-row bands did.
const TASK_ROWS: usize = 64;

/// The strips of [`TASK_ROWS`] rows of `cell` that meet rows `ys`, each its
/// own disjoint cell; strips the dab does not reach are not tasks.
fn split_rows<'a>(cell: Cell<'a>, ys: &Range<usize>) -> Vec<Cell<'a>> {
    let Cell {
        x,
        y,
        w,
        h,
        scratch,
        applied,
        planes,
    } = cell;
    let n = (w * TASK_ROWS).max(1);
    let (mut s, mut a) = (scratch.chunks_mut(n), applied.chunks_mut(n));
    let mut p = planes.map(|plane| plane.map(|d| d.chunks_mut(n)));
    (0..h.div_ceil(TASK_ROWS))
        .filter_map(|k| {
            let strip = (
                s.next()?,
                a.next()?,
                p.each_mut().map(|it| it.as_mut().and_then(Iterator::next)),
            );
            let top = y + k * TASK_ROWS;
            let rows = TASK_ROWS.min(h - k * TASK_ROWS);
            if top >= ys.end || top + rows <= ys.start {
                return None;
            }
            Some(Cell {
                x,
                y: top,
                w,
                h: rows,
                scratch: strip.0,
                applied: strip.1,
                planes: strip.2,
            })
        })
        .collect()
}

/// Copy the layer-local `rect` of a per-dab engine's `pixels` into the layer's
/// planes, forking only the tiles it reaches. Returns the box of the pixels
/// that changed.
pub(super) fn write_back(
    layer: &mut Layer,
    ch: PlaneIndex,
    pixels: &RgbaImage,
    rect: PsdRect,
    scratch: &mut Plane<u8>,
    applied: &mut Plane<u8>,
    serial: bool,
) -> Option<PsdRect> {
    let (w, h) = (pixels.width as usize, pixels.height as usize);
    let xs = rect.left as usize..rect.right as usize;
    let ys = rect.top as usize..rect.bottom as usize;
    let cells = cells(layer, ch, w, h, xs, ys, scratch, applied);
    if serial {
        cells
            .into_iter()
            .filter_map(|cell| write_back_cell(cell, pixels, rect))
            .reduce(PsdRect::union)
    } else {
        cells
            .into_iter()
            .flat_map(|cell| split_rows(cell, &(rect.top as usize..rect.bottom as usize)))
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter_map(|cell| write_back_cell(cell, pixels, rect))
            .reduce_with(PsdRect::union)
    }
}

fn write_back_cell(mut cell: Cell<'_>, pixels: &RgbaImage, rect: PsdRect) -> Option<PsdRect> {
    let w = pixels.width as usize;
    let top = cell.y.max(rect.top as usize);
    let bottom = (cell.y + cell.h).min(rect.bottom as usize);
    let left = cell.x.max(rect.left as usize);
    let right = (cell.x + cell.w).min(rect.right as usize);
    let mut bbox: Option<PsdRect> = None;
    for ly in top..bottom {
        let source = &pixels.data[ly * w..(ly + 1) * w];
        let row = (ly - cell.y) * cell.w;
        let mut first = None;
        let mut last = 0;
        for (lx, &[r, g, b, a]) in source.iter().enumerate().take(right).skip(left) {
            let at = row + lx - cell.x;
            if write_pixel(&mut cell.planes, at, PaintMode::Normal, (r, g, b, a)) {
                first.get_or_insert(lx);
                last = lx;
            }
        }
        if let Some(first) = first {
            let row_box = PsdRect {
                top: ly as i32,
                left: first as i32,
                bottom: ly as i32 + 1,
                right: last as i32 + 1,
            };
            bbox = Some(bbox.map_or(row_box, |b| b.union(row_box)));
        }
    }
    bbox
}

/// Write one pixel at offset `at` of a cell's planes; `true` when a byte
/// changed.
pub(super) fn write_pixel(
    planes: &mut CellPlanes<'_>,
    at: usize,
    mode: PaintMode,
    rgba: (u8, u8, u8, u8),
) -> bool {
    let (r, g, b, a) = rgba;
    let write_rgb = !matches!(mode, PaintMode::Clear) || planes[3].is_some();
    let mut changed = false;
    if write_rgb {
        for (k, v) in [(0, r), (1, g), (2, b)] {
            if let Some(p) = planes[k].as_deref_mut() {
                if p[at] != v {
                    p[at] = v;
                    changed = true;
                }
            }
        }
    }
    if let Some(p) = planes[3].as_deref_mut() {
        if p[at] != a {
            p[at] = a;
            changed = true;
        }
    }
    changed
}
