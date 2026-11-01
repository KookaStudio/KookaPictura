//! The merged layer: a multiband pixel blend of the processed layers. Each
//! layer's Laplacian pyramid is added under a Gaussian pyramid of its weight, so
//! low frequencies mix widely and detail stays crisp. Approximated: Adobe's method
//! is unpublished.
//!
//! ponytail: memory follows the blended area at about 5 float planes per colour
//! channel, and transparent pixels take the layer's mean colour instead of a
//! push-pull fill.

use pictura_core::{
    BitDepth, BlendMode, Channel, Document, Layer, PsdRect, Samples, SourceChannels,
};
use pictura_paint::healing::{heal_region, HealMode, RgbaImage};

use super::pyramid::Level;
use super::sources::{color_ids, unit_row, Grid};
use super::{Cancelled, Control};

const MAX_LEVELS: usize = 8;
const TONE_FEATURE_SHARE: usize = 32;

/// What the blend needs beyond the layers.
pub(super) struct BlendInput<'a> {
    pub domain: PsdRect,
    pub grid: &'a Grid,
    pub labels: &'a [u16],
    pub gains: &'a [[f32; 3]],
    /// Levels from this one up take tone from the lowest covering layer.
    pub base_from: Option<usize>,
    /// Where weights stay on the hard label, `0..=1` once scaled.
    pub conflict: Option<&'a [f32]>,
}

/// The blended colour and alpha over `rect`.
pub(super) struct Merged {
    rect: PsdRect,
    channels: Vec<Vec<f32>>,
    alpha: Vec<f32>,
}

/// The first level treated as tone: a feature about 1/24 of the short side.
pub(super) fn tone_level(domain: PsdRect) -> usize {
    let short = domain.width().min(domain.height()).max(1) as usize;
    ((short / TONE_FEATURE_SHARE).max(1).ilog2() as usize).max(1)
}

fn level_count(w: usize, h: usize) -> usize {
    (w.min(h).max(1).ilog2() as usize).clamp(1, MAX_LEVELS)
}

/// One colour channel of `layer` over `domain`: gained, with transparent pixels
/// at the visible mean. Also returns the layer's alpha when `want_alpha`.
fn read_plane(
    doc: &Document,
    layer: &Layer,
    id: i16,
    domain: PsdRect,
    gain: f32,
    alpha: &[f32],
) -> Vec<f32> {
    let (w, h) = (domain.width() as usize, domain.height() as usize);
    let mut plane = vec![0f32; w * h];
    let (lw, lh) = (
        layer.rect.width().max(0) as usize,
        layer.rect.height().max(0) as usize,
    );
    let mut row = vec![0f32; lw];
    let (mut total, mut seen) = (0f64, 0f64);
    for y in 0..lh {
        let dy = layer.rect.top + y as i32 - domain.top;
        if dy < 0 || dy as usize >= h {
            continue;
        }
        unit_row(doc, layer, id, y, &mut row, 0.0);
        for (x, &value) in row.iter().enumerate() {
            let dx = layer.rect.left + x as i32 - domain.left;
            if dx < 0 || dx as usize >= w {
                continue;
            }
            let i = dy as usize * w + dx as usize;
            if alpha[i] > 0.0 {
                plane[i] = (value * gain).clamp(0.0, 1.0);
                total += f64::from(plane[i] * alpha[i]);
                seen += f64::from(alpha[i]);
            }
        }
    }
    let mean = if seen > 0.0 {
        (total / seen) as f32
    } else {
        0.0
    };
    for (value, &a) in plane.iter_mut().zip(alpha) {
        if a <= 0.0 {
            *value = mean;
        }
    }
    plane
}

fn read_alpha(doc: &Document, layer: &Layer, domain: PsdRect) -> Vec<f32> {
    let (w, h) = (domain.width() as usize, domain.height() as usize);
    let mut plane = vec![0f32; w * h];
    let lw = layer.rect.width().max(0) as usize;
    let mut row = vec![0f32; lw];
    for y in 0..layer.rect.height().max(0) as usize {
        let dy = layer.rect.top + y as i32 - domain.top;
        if dy < 0 || dy as usize >= h {
            continue;
        }
        unit_row(doc, layer, -1, y, &mut row, 1.0);
        for (x, &value) in row.iter().enumerate() {
            let dx = layer.rect.left + x as i32 - domain.left;
            if dx >= 0 && (dx as usize) < w {
                plane[dy as usize * w + dx as usize] = value;
            }
        }
    }
    plane
}

/// The layer's weight at each level.
fn weight_levels(weight: Vec<f32>, w: usize, h: usize, count: usize) -> Vec<Level> {
    super::pyramid::pyramid(weight, w, h, count)
}

/// From level `from` up, weigh this layer by what it adds over the layers below
/// it, not by its label: broad tone follows the base and detail the sharpest
/// layer, so kept detail takes on the tone beneath it.
fn base_tone(
    levels: &[Level],
    below: &[f32],
    alpha: &[f32],
    from: usize,
    w: usize,
    h: usize,
    count: usize,
) -> Vec<Level> {
    let exposed: Vec<f32> = below
        .iter()
        .zip(alpha)
        .map(|(&b, &a)| a * (1.0 - b))
        .collect();
    let tone = super::pyramid::pyramid(exposed, w, h, count);
    levels
        .iter()
        .zip(tone)
        .enumerate()
        .map(|(k, (label, base))| {
            if k < from {
                Level {
                    w: label.w,
                    h: label.h,
                    data: label.data.clone(),
                }
            } else {
                base
            }
        })
        .collect()
}

/// Replace RGB planes with luma and two colour differences, so brightness and
/// colour can follow different layers. A single plane passes through.
fn to_opponent(planes: &mut [Vec<f32>]) {
    let [r, g, b] = planes else {
        return;
    };
    for ((r, g), b) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let y = 0.299 * *r + 0.587 * *g + 0.114 * *b;
        (*r, *g, *b) = (y, *r - y, *b - y);
    }
}

fn from_opponent(planes: &mut [Vec<f32>]) {
    let [y, cr, cb] = planes else {
        return;
    };
    for ((y, cr), cb) in y.iter_mut().zip(cr.iter_mut()).zip(cb.iter_mut()) {
        let (r, b) = (*y + *cr, *y + *cb);
        let g = (*y - 0.299 * r - 0.114 * b) / 0.587;
        (*y, *cr, *cb) = (r, g, b);
    }
}

/// Move a coarse weight to the hard weight where layers disagree, so low bands
/// do not ghost across a seam. `hard` and `conflict` are full-resolution.
fn tighten(levels: &mut [Level], hard: &[f32], conflict: &[f32], width: usize) {
    for (k, level) in levels.iter_mut().enumerate().skip(1) {
        let scale = 1usize << k;
        let (lw, lh) = (level.w, level.h);
        for y in 0..lh {
            for x in 0..lw {
                let fine =
                    ((y * scale).min(hard.len() / width - 1)) * width + (x * scale).min(width - 1);
                let c = conflict[fine];
                let i = y * lw + x;
                level.data[i] += (hard[fine] - level.data[i]) * c;
            }
        }
    }
}

fn add_bands(result: &mut [Level], plane: Vec<f32>, w: usize, h: usize, weights: &[Level]) {
    let gaussian = super::pyramid::pyramid(plane, w, h, result.len());
    let top = gaussian.len() - 1;
    for k in 0..=top {
        let fine = &gaussian[k];
        let (lw, target) = (fine.w, &mut result[k].data);
        for (i, out) in target.iter_mut().enumerate() {
            let band = if k == top {
                fine.data[i]
            } else {
                fine.data[i] - gaussian[k + 1].sample(i % lw, i / lw, 2)
            };
            *out += band * weights[k].data[i];
        }
    }
}

fn collapse(mut bands: Vec<Level>, totals: &[Level]) -> Vec<f32> {
    for (band, total) in bands.iter_mut().zip(totals) {
        for (value, &sum) in band.data.iter_mut().zip(&total.data) {
            *value = if sum > 1e-6 { *value / sum } else { 0.0 };
        }
    }
    for k in (0..bands.len() - 1).rev() {
        let (fine, coarse) = bands.split_at_mut(k + 1);
        let (fine, coarse) = (&mut fine[k], &coarse[0]);
        let lw = fine.w;
        for (i, value) in fine.data.iter_mut().enumerate() {
            *value += coarse.sample(i % lw, i / lw, 2);
        }
    }
    bands.swap_remove(0).data
}

pub(super) fn blend(
    doc: &Document,
    layers: &[&Layer],
    input: &BlendInput,
    control: &Control,
) -> Result<Merged, Cancelled> {
    let domain = input.domain;
    let (w, h) = (domain.width() as usize, domain.height() as usize);
    let count = level_count(w, h);
    let ids = color_ids(doc);
    let empty = || super::pyramid::pyramid(vec![0f32; w * h], w, h, count);
    let mut bands: Vec<Vec<Level>> = ids.iter().map(|_| empty()).collect();
    let mut totals = empty();
    let mut luma_totals = empty();
    let mut alpha_out = vec![0f32; w * h];
    let mut covered_below = vec![0f32; w * h];
    let conflict: Option<Vec<f32>> = input.conflict.map(|plane| {
        (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as i32 + domain.left, (i / w) as i32 + domain.top);
                (input.grid.sample(plane, x, y) * CONFLICT_GAIN).clamp(0.0, 1.0)
            })
            .collect()
    });
    for (index, layer) in layers.iter().enumerate() {
        let alpha = read_alpha(doc, layer, domain);
        let hot: Vec<f32> = input
            .labels
            .iter()
            .map(|&id| f32::from(u8::from(id as usize == index)))
            .collect();
        let weight: Vec<f32> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as i32 + domain.left, (i / w) as i32 + domain.top);
                input.grid.sample(&hot, x, y) * alpha[i]
            })
            .collect();
        let mut weights = weight_levels(weight.clone(), w, h, count);
        let mut tone = None;
        if let Some(from) = input.base_from {
            tone = Some(base_tone(
                &weights,
                &covered_below,
                &alpha,
                from,
                w,
                h,
                count,
            ));
            for (below, &a) in covered_below.iter_mut().zip(&alpha) {
                *below = *below + a - *below * a;
            }
        }
        if let Some(conflict) = &conflict {
            tighten(&mut weights, &weight, conflict, w);
        }
        control.check()?;
        let mut planes: Vec<Vec<f32>> = ids
            .iter()
            .enumerate()
            .map(|(slot, &id)| read_plane(doc, layer, id, domain, input.gains[index][slot], &alpha))
            .collect();
        to_opponent(&mut planes);
        let luma_weights = tone.as_ref().unwrap_or(&weights);
        for (slot, plane) in planes.into_iter().enumerate() {
            control.check()?;
            let (level_weights, level_totals) = if slot == 0 {
                (luma_weights, &mut luma_totals)
            } else {
                (&weights, &mut totals)
            };
            add_bands(&mut bands[slot], plane, w, h, level_weights);
            if slot <= 1 {
                for (total, level) in level_totals.iter_mut().zip(level_weights) {
                    for (sum, value) in total.data.iter_mut().zip(&level.data) {
                        *sum += value;
                    }
                }
            }
        }
        for (out, &a) in alpha_out.iter_mut().zip(&alpha) {
            *out = a + *out * (1.0 - a);
        }
    }
    let mut channels: Vec<Vec<f32>> = bands
        .into_iter()
        .enumerate()
        .map(|(slot, levels)| collapse(levels, if slot == 0 { &luma_totals } else { &totals }))
        .collect();
    from_opponent(&mut channels);
    for plane in &mut channels {
        plane.iter_mut().for_each(|v| *v = v.clamp(0.0, 1.0));
    }
    Ok(Merged {
        rect: domain,
        channels,
        alpha: alpha_out,
    })
}

/// Conflict at which a weight is fully hard.
const CONFLICT_GAIN: f32 = 12.0;

impl Merged {
    /// Grow to span the canvas and synthesize its transparent pixels (a post-CS6
    /// option, not CS6 parity). The synthesis is 8-bit, so callers skip high depth.
    pub(super) fn fill_transparent(&mut self, width: u32, height: u32) {
        let (cw, ch) = (width as i32, height as i32);
        let rect = PsdRect {
            top: self.rect.top.min(0),
            left: self.rect.left.min(0),
            bottom: self.rect.bottom.max(ch),
            right: self.rect.right.max(cw),
        };
        self.grow(rect);
        let (w, h) = (rect.width(), rect.height());
        let gray = self.channels.len() == 1;
        let value = |channel: &[f32], i: usize| (channel[i] * 255.0).round() as u8;
        let mut data = Vec::with_capacity((w * h) as usize);
        for i in 0..(w * h) as usize {
            let r = value(&self.channels[0], i);
            data.push(if gray {
                [r, r, r, 255]
            } else {
                [
                    r,
                    value(&self.channels[1], i),
                    value(&self.channels[2], i),
                    255,
                ]
            });
        }
        let mut image = RgbaImage {
            width: w,
            height: h,
            data,
        };
        let holes: Vec<f32> = self.alpha.iter().map(|a| 1.0 - a).collect();
        let region = PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        };
        if heal_region(&mut image, region, &holes, HealMode::ContentAware).is_none() {
            return;
        }
        let canvas = PsdRect {
            top: -rect.top,
            left: -rect.left,
            bottom: ch - rect.top,
            right: cw - rect.left,
        };
        for (i, px) in image.data.iter().enumerate() {
            let (x, y) = ((i as i32) % w, (i as i32) / w);
            if y < canvas.top || y >= canvas.bottom || x < canvas.left || x >= canvas.right {
                continue;
            }
            for (slot, channel) in self.channels.iter_mut().enumerate() {
                channel[i] = f32::from(px[slot]) / 255.0;
            }
            self.alpha[i] = 1.0;
        }
    }

    fn grow(&mut self, rect: PsdRect) {
        let (w, h) = (rect.width() as usize, rect.height() as usize);
        let (ow, oh) = (self.rect.width() as usize, self.rect.height() as usize);
        let (dx, dy) = (
            (self.rect.left - rect.left) as usize,
            (self.rect.top - rect.top) as usize,
        );
        let move_plane = |plane: &[f32]| {
            let mut out = vec![0f32; w * h];
            for y in 0..oh {
                let from = &plane[y * ow..(y + 1) * ow];
                out[(y + dy) * w + dx..(y + dy) * w + dx + ow].copy_from_slice(from);
            }
            out
        };
        self.channels = self.channels.iter().map(|c| move_plane(c)).collect();
        self.alpha = move_plane(&self.alpha);
        self.rect = rect;
    }

    /// The merged raster, at the document's depth.
    pub(super) fn into_layer(self, doc: &Document, name: String) -> Layer {
        let narrow = |plane: &[f32]| -> Vec<u8> {
            plane
                .iter()
                .map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8)
                .collect()
        };
        let ids = color_ids(doc);
        let mut channels: Vec<Channel> = ids
            .iter()
            .zip(&self.channels)
            .map(|(&id, plane)| Channel {
                id,
                data: narrow(plane).into(),
            })
            .collect();
        channels.push(Channel {
            id: -1,
            data: narrow(&self.alpha).into(),
        });
        let native = |plane: &[f32], depth: BitDepth| match depth {
            BitDepth::Sixteen => Samples::U16(
                plane
                    .iter()
                    .map(|v| (v * 65535.0).round() as u16)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            _ => Samples::F32(plane.to_vec().into()),
        };
        let source_channels = doc
            .source_depth
            .filter(|depth| {
                doc.source_mode.is_none()
                    && matches!(depth, BitDepth::Sixteen | BitDepth::ThirtyTwo)
            })
            .map(|depth| {
                let planes = ids
                    .iter()
                    .zip(&self.channels)
                    .map(|(&id, plane)| (id, native(plane, depth)))
                    .chain([(-1, native(&self.alpha, depth))])
                    .collect();
                SourceChannels::new(depth, self.rect, planes)
            });
        Layer {
            name,
            rect: self.rect,
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            visible: true,
            channels,
            source_channels,
            ..Default::default()
        }
    }
}
