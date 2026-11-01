//! Reading layers for Auto-Blend at the document's depth, reduced to a bounded
//! work grid. Approximated: see the module header in `mod.rs`.

use pictura_core::{BitDepth, Document, Layer, PsdRect, Samples};

use super::{Cancelled, Control};

/// The largest analysis grid. Larger documents use an integer-reduced grid.
///
/// ponytail: above this size seams and mask edges stop at the work grid.
pub(super) const MAX_WORK_PIXELS: u64 = 16_000_000;

pub(super) const UNLABELLED: u16 = u16::MAX;

/// An integer-reduced view of a document-space rectangle.
pub(super) struct Grid {
    pub left: i32,
    pub top: i32,
    pub w: usize,
    pub h: usize,
    /// Document pixels per cell along each axis.
    pub f: usize,
}

impl Grid {
    pub fn new(domain: PsdRect, max_pixels: u64) -> Grid {
        let (dw, dh) = (
            domain.width().max(0) as usize,
            domain.height().max(0) as usize,
        );
        let f = work_scale(dw as u64 * dh as u64, max_pixels);
        Grid {
            left: domain.left,
            top: domain.top,
            w: dw.div_ceil(f),
            h: dh.div_ceil(f),
            f,
        }
    }

    /// The cell holding document pixel `(x, y)`, if the grid reaches it.
    #[inline]
    fn cell(&self, x: i32, y: i32) -> Option<usize> {
        let cx = usize::try_from(x - self.left).ok()? / self.f;
        let cy = usize::try_from(y - self.top).ok()? / self.f;
        (cx < self.w && cy < self.h).then_some(cy * self.w + cx)
    }

    /// Bilinear sample of `work` at document pixel `(x, y)`, zero off the grid.
    fn sample(&self, work: &[f32], x: i32, y: i32) -> f32 {
        let (dx, dy) = (x - self.left, y - self.top);
        let (span_x, span_y) = (self.w * self.f, self.h * self.f);
        if dx < 0 || dy < 0 || dx as usize >= span_x || dy as usize >= span_y {
            return 0.0;
        }
        let f = self.f as f32;
        let u = ((dx as f32 + 0.5) / f - 0.5).clamp(0.0, (self.w - 1) as f32);
        let v = ((dy as f32 + 0.5) / f - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (u as usize, v as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (a, b) = (u - x0 as f32, v - y0 as f32);
        let at = |x: usize, y: usize| work[y * self.w + x];
        let top = at(x0, y0) * (1.0 - a) + at(x1, y0) * a;
        let bottom = at(x0, y1) * (1.0 - a) + at(x1, y1) * a;
        top * (1.0 - b) + bottom * b
    }

    /// Expand unit `work` to 8-bit samples over `rect`.
    pub fn expand(&self, work: &[f32], rect: PsdRect) -> Vec<u8> {
        let (w, h) = (rect.width().max(0), rect.height().max(0));
        let mut out = Vec::with_capacity(w as usize * h as usize);
        for y in rect.top..rect.top + h {
            for x in rect.left..rect.left + w {
                let value = if self.f == 1 {
                    self.cell(x, y).map_or(0.0, |cell| work[cell])
                } else {
                    self.sample(work, x, y)
                };
                out.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        out
    }
}

/// The smallest integer reduction that fits `area` into `max_pixels`.
pub(super) fn work_scale(area: u64, max_pixels: u64) -> usize {
    ((area as f64 / max_pixels as f64).sqrt().ceil() as usize).max(1)
}

/// One bit per cell.
pub(super) struct Bits(Vec<u64>);

impl Bits {
    pub fn new(cells: usize) -> Bits {
        Bits(vec![0; cells.div_ceil(64)])
    }

    #[inline]
    pub fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }

    #[inline]
    pub fn get(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }
}

/// Read row `y` of channel `id` as unit samples, preferring the native plane;
/// a missing channel reads as `default`.
pub(super) fn unit_row(
    doc: &Document,
    layer: &Layer,
    id: i16,
    y: usize,
    out: &mut [f32],
    default: f32,
) {
    let w = out.len();
    let native = doc
        .authoritative_plane(layer, id)
        .and_then(|plane: &Samples| plane.row(y, w));
    if let Some(row) = native {
        for (x, value) in out.iter_mut().enumerate() {
            *value = row.unit(x).unwrap_or(default);
        }
    } else if let Some(channel) = layer.channels.iter().find(|c| c.id == id) {
        let data = channel.data.as_slice();
        for (x, value) in out.iter_mut().enumerate() {
            *value = data
                .get(y * w + x)
                .map_or(default, |&sample| f32::from(sample) / 255.0);
        }
    } else {
        out.fill(default);
    }
}

/// The colour channels of `doc`'s layers.
pub(super) fn color_ids(doc: &Document) -> &'static [i16] {
    if doc.mode == pictura_core::ColorMode::Grayscale {
        &[0]
    } else {
        &[0, 1, 2]
    }
}

struct Row {
    luma: Vec<f32>,
    alpha: Vec<f32>,
}

fn read_row(doc: &Document, layer: &Layer, y: usize, w: usize) -> Row {
    let mut alpha = vec![0f32; w];
    unit_row(doc, layer, -1, y, &mut alpha, 1.0);
    let ids = color_ids(doc);
    let mut luma = vec![0f32; w];
    if ids.len() == 1 {
        unit_row(doc, layer, 0, y, &mut luma, 0.0);
    } else {
        let mut channel = vec![0f32; w];
        for (&id, weight) in ids.iter().zip([0.299f32, 0.587, 0.114]) {
            unit_row(doc, layer, id, y, &mut channel, 0.0);
            for (out, value) in luma.iter_mut().zip(&channel) {
                *out += value * weight;
            }
        }
    }
    // 32-bit samples are linear; analysis wants a perceptual scale.
    if doc.source_depth == Some(BitDepth::ThirtyTwo) {
        luma.iter_mut().for_each(|v| *v = v.powf(1.0 / 2.2));
    }
    Row { luma, alpha }
}

/// One layer reduced to the work grid.
pub(super) struct Scan {
    pub luma: Vec<f32>,
    pub alpha: Vec<f32>,
    /// Mean `|Laplacian|` of the fully opaque interior, so layer edges do not
    /// read as detail. Empty unless asked for.
    pub energy: Vec<f32>,
}

/// Reduce `layer` onto `grid`. The Laplacian runs at source resolution so fine
/// focus detail survives the reduction, over a three-row window so memory does
/// not follow the layer's size.
pub(super) fn scan(
    doc: &Document,
    layer: &Layer,
    grid: &Grid,
    want_energy: bool,
    control: &Control,
) -> Result<Scan, Cancelled> {
    let cells = grid.w * grid.h;
    let mut luma = vec![0f32; cells];
    let mut alpha = vec![0f32; cells];
    let mut energy = vec![0f32; if want_energy { cells } else { 0 }];
    let (w, h) = (
        layer.rect.width().max(0) as usize,
        layer.rect.height().max(0) as usize,
    );
    let mut above: Option<Row> = None;
    let mut here = (h > 0).then(|| read_row(doc, layer, 0, w));
    for y in 0..h {
        if y.is_multiple_of(256) {
            control.check()?;
        }
        let below = (want_energy && y + 1 < h).then(|| read_row(doc, layer, y + 1, w));
        let row = here.as_ref().expect("a row exists for every y");
        for x in 0..w {
            let (dx, dy) = (layer.rect.left + x as i32, layer.rect.top + y as i32);
            let Some(cell) = grid.cell(dx, dy) else {
                continue;
            };
            let a = row.alpha[x];
            luma[cell] += row.luma[x] * a;
            alpha[cell] += a;
            if !want_energy || x == 0 || x + 1 == w {
                continue;
            }
            if let (Some(up), Some(down)) = (&above, &below) {
                let opaque = [
                    a,
                    row.alpha[x - 1],
                    row.alpha[x + 1],
                    up.alpha[x],
                    down.alpha[x],
                ]
                .iter()
                .all(|&v| v > 0.99);
                if opaque {
                    energy[cell] += (row.luma[x - 1] + row.luma[x + 1] + up.luma[x] + down.luma[x]
                        - 4.0 * row.luma[x])
                        .abs();
                }
            }
        }
        above = here.take();
        here = if want_energy {
            below
        } else {
            (y + 1 < h).then(|| read_row(doc, layer, y + 1, w))
        };
    }
    let area = (grid.f * grid.f) as f32;
    for (l, a) in luma.iter_mut().zip(alpha.iter_mut()) {
        if *a > 0.0 {
            *l /= *a;
        }
        *a /= area;
    }
    energy.iter_mut().for_each(|e| *e /= area);
    Ok(Scan {
        luma,
        alpha,
        energy,
    })
}
