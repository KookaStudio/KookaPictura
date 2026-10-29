//! The banded dab raster: a large dab's rows cut into bands that run in
//! parallel, each writing only its own rows of the coverage buffers and planes.

use super::{union_rect, PlaneIndex, Stencil};
use crate::tip::TipParams;
use crate::PaintMode;
use pictura_core::{Layer, PsdRect, BAND_ROWS};

/// The target layer's R, G, B and alpha planes, or a cut of the same rows of
/// each, in that order.
pub(super) type BandPlanes<'a> = [Option<&'a mut [u8]>; 4];

/// The mutable bands of one channel covering a dab's rows, in band order.
pub(super) struct PlaneCut<'a> {
    bands: Vec<&'a mut [u8]>,
    first_band: usize,
}

/// The per-channel band cuts for a dab.
pub(super) type PlaneCuts<'a> = [Option<PlaneCut<'a>>; 4];

/// Whole rows `y .. y + scratch.len() / width` of the layer: the coverage
/// buffers and the planes a band of a dab writes, disjoint from every other
/// band's.
pub(super) struct Band<'a> {
    pub(super) y: usize,
    pub(super) scratch: &'a mut [u8],
    applied: &'a mut [u8],
    pub(super) planes: BandPlanes<'a>,
}

/// One dab's raster inputs, shared by its bands.
pub(super) struct DabRaster<'a> {
    pub(super) params: &'a TipParams,
    pub(super) dab_x: f32,
    pub(super) dab_y: f32,
    pub(super) flow: f32,
    pub(super) width: usize,
    pub(super) stencil: &'a Stencil<'a>,
}

impl DabRaster<'_> {
    /// Raster the dab over `band`, returning the layer-local box of the pixels
    /// whose applied coverage changed.
    pub(super) fn band(&self, mut band: Band<'_>, mut rng: Option<&mut u64>) -> Option<PsdRect> {
        let w = self.width;
        let mut bbox: Option<PsdRect> = None;
        for row in 0..band.scratch.len() / w {
            let ly = band.y + row;
            let dy = (ly as f32 + 0.5) - self.dab_y;
            // A round tip's support on this row is a chord, so the corners
            // of its square bounding box cost nothing. A non-round shape
            // returns the full width and its out-of-support pixels fall to
            // the `cov <= 0.0` filter below.
            let Some((xlo, xhi)) = self.params.x_span(dy) else {
                continue;
            };
            let x0 = ((self.dab_x + xlo).floor() as i32).max(0);
            let x1 = ((self.dab_x + xhi).ceil() as i32).min(w as i32 - 1);
            let mut first = None;
            let mut last = 0;
            for lx in x0..=x1 {
                let dx = (lx as f32 + 0.5) - self.dab_x;
                let cov = self.params.coverage(dx, dy);
                if cov <= 0.0 {
                    continue;
                }
                let at = row * w + lx as usize;
                let acc = 1.0 - (1.0 - band.scratch[at] as f32 / 255.0) * (1.0 - self.flow * cov);
                let v = (acc.clamp(0.0, 1.0) * 255.0).round() as u8;
                band.scratch[at] = v;
                let cfg = &self.stencil.cfg;
                let v = if cfg.wet_edges { crate::wet_edge(v) } else { v };
                // The pencil's paper grain: a fixed alpha modulation keyed to
                // the layer coordinates. It dims the stroke by up to `grain` %
                // where the noise is lowest instead of dropping pixels to zero,
                // which would scatter the stroke into isolated dots; the pattern
                // is fixed, so overlapping dabs only deepen it. The noise is
                // clumped at 2–3 px so the stroke reads as pencil texture
                // rather than static.
                let v = if cfg.grain > 0 {
                    let noise = crate::fill::grain_noise(lx, ly as i32);
                    let keep = 1.0 - cfg.grain as f32 / 100.0 * (1.0 - noise);
                    (v as f32 * keep).round() as u8
                } else {
                    v
                };
                if v == band.applied[at] {
                    continue;
                }
                band.applied[at] = v;
                let i = ly * w + lx as usize;
                if !self
                    .stencil
                    .composite(&mut band.planes, at, i, rng.as_deref_mut(), v)
                {
                    continue;
                }
                first.get_or_insert(lx);
                last = lx;
            }
            if let Some(first) = first {
                let row_box = PsdRect {
                    top: ly as i32,
                    left: first,
                    bottom: ly as i32 + 1,
                    right: last + 1,
                };
                bbox = Some(bbox.map_or(row_box, |b| union_rect(b, row_box)));
            }
        }
        bbox
    }
}

/// The target layer's planes fractured and cut to the bands covering rows
/// `y0 .. y1`, so a dab forks only the bands it touches.
pub(super) fn plane_slices<'a>(
    layer: &'a mut Layer,
    ch: PlaneIndex,
    width: usize,
    height: usize,
    y0: usize,
    y1: usize,
) -> PlaneCuts<'a> {
    let len = width * height;
    let mut out: PlaneCuts<'a> = [None, None, None, None];
    let first = y0 / BAND_ROWS;
    let last = y1.saturating_sub(1) / BAND_ROWS;
    let indices: Vec<usize> = (first..=last).collect();
    for (idx, c) in layer.channels.iter_mut().enumerate() {
        if let Some(k) = ch.iter().position(|&x| x == Some(idx)) {
            if c.data.len() == len {
                c.data.fracture(width);
                if let Some(bands) = c.data.write_bands(&indices) {
                    out[k] = Some(PlaneCut {
                        bands,
                        first_band: first,
                    });
                }
            }
        }
    }
    out
}

/// Cut rows `y0 .. y0 + rows` of the coverage buffers and the plane bands into
/// one band per plane band. Rows of a band outside the dab are harmless: the
/// tip has no coverage there.
pub(super) fn split_bands<'a>(
    y0: usize,
    rows: usize,
    w: usize,
    scratch: &'a mut [u8],
    applied: &'a mut [u8],
    cuts: PlaneCuts<'a>,
) -> Vec<Band<'a>> {
    let end = y0 + rows;
    let first_band = cuts
        .iter()
        .flatten()
        .next()
        .map_or(y0 / BAND_ROWS, |cut| cut.first_band);
    let mut iters: [Option<std::vec::IntoIter<&'a mut [u8]>>; 4] =
        cuts.map(|cut| cut.map(|cut| cut.bands.into_iter()));
    let total = scratch.len();
    let applied_total = applied.len();
    let height = total.checked_div(w).unwrap_or(0);
    let mut scratch_rest = scratch;
    let mut applied_rest = applied;
    let mut row = 0usize;
    let mut bands = Vec::new();
    let mut band = first_band;
    while band * BAND_ROWS < end {
        let sy = band * BAND_ROWS;
        let ey = (sy + BAND_ROWS).min(height);
        if ey <= sy {
            break;
        }
        let b = ey * w;
        if b > total || b > applied_total || sy < row {
            break;
        }
        let slice = std::mem::take(&mut scratch_rest);
        let (_, rest) = slice.split_at_mut((sy - row) * w);
        let (band_scratch, rest) = rest.split_at_mut((ey - sy) * w);
        scratch_rest = rest;
        let slice = std::mem::take(&mut applied_rest);
        let (_, rest) = slice.split_at_mut((sy - row) * w);
        let (band_applied, rest) = rest.split_at_mut((ey - sy) * w);
        applied_rest = rest;
        let planes = iters
            .each_mut()
            .map(|it| it.as_mut().and_then(|it| it.next()));
        bands.push(Band {
            y: sy,
            scratch: band_scratch,
            applied: band_applied,
            planes,
        });
        row = ey;
        band += 1;
    }
    bands
}

/// `write_pixel` for a band's planes.
pub(super) fn write_band(
    planes: &mut BandPlanes<'_>,
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
