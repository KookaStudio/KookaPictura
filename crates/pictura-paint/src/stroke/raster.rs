//! The banded dab raster: a large dab's rows cut into bands that run in
//! parallel, each writing only its own rows of the coverage buffers and planes.

use super::{union_rect, PlaneIndex, Stencil};
use crate::tip::TipParams;
use crate::PaintMode;
use pictura_core::{Layer, PsdRect};

/// A dab's area at which its rows are split across threads: below it the
/// split costs more than the raster it shares out.
pub(super) const PARALLEL_MIN_PX: usize = 128 * 128;

/// Roughly the pixels of a dab one band covers, so a band is worth a task
/// whatever the brush width.
pub(super) const BAND_PX: usize = 16 * 1024;

/// The target layer's R, G, B and alpha planes, or a cut of the same rows of
/// each, in that order.
pub(super) type BandPlanes<'a> = [Option<&'a mut [u8]>; 4];

/// Whole rows `y .. y + scratch.len() / width` of the layer: the coverage
/// buffers and the planes a band of a dab writes, disjoint from every other
/// band's.
pub(super) struct Band<'a> {
    pub(super) y: usize,
    pub(super) scratch: &'a mut [u8],
    applied: &'a mut [u8],
    planes: BandPlanes<'a>,
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

/// The target layer's planes as mutable slices in R, G, B, alpha order. A
/// plane that is not the layer's size is left out, as `fill_layer` does.
pub(super) fn plane_slices(layer: &mut Layer, ch: PlaneIndex, len: usize) -> BandPlanes<'_> {
    let mut out: BandPlanes<'_> = [None, None, None, None];
    for (idx, c) in layer.channels.iter_mut().enumerate() {
        if let Some(k) = ch.iter().position(|&x| x == Some(idx)) {
            if c.data.len() == len {
                out[k] = Some(c.data.as_mut_slice());
            }
        }
    }
    out
}

/// Cut rows `y0 .. y0 + rows` of the coverage buffers and the planes into
/// bands of `band_rows` rows.
pub(super) fn split_bands<'a>(
    y0: usize,
    rows: usize,
    band_rows: usize,
    w: usize,
    scratch: &'a mut [u8],
    applied: &'a mut [u8],
    planes: &'a mut BandPlanes<'_>,
) -> Vec<Band<'a>> {
    let (a, b) = (y0 * w, (y0 + rows) * w);
    let n = band_rows.max(1) * w;
    let mut scratch = scratch[a..b].chunks_mut(n);
    let mut applied = applied[a..b].chunks_mut(n);
    let mut cut = planes
        .each_mut()
        .map(|p| p.as_deref_mut().map(|p| p[a..b].chunks_mut(n)));
    let mut bands = Vec::with_capacity(rows.div_ceil(band_rows.max(1)));
    let mut y = y0;
    while let (Some(scratch), Some(applied)) = (scratch.next(), applied.next()) {
        let planes = cut.each_mut().map(|c| c.as_mut().and_then(Iterator::next));
        let next = y + scratch.len() / w;
        bands.push(Band {
            y,
            scratch,
            applied,
            planes,
        });
        y = next;
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
