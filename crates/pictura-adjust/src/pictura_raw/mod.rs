//! Pictura Raw: our reimplementation of the Camera Raw image-adjustment tabs,
//! applied to an 8-bit RGB/RGBA buffer in `f32`.
//!
//! Pipeline order: Basic (white balance, exposure, contrast,
//! highlights/shadows/whites/blacks, clarity, vibrance, saturation) -> Tone Curve
//! -> Detail -> HSL/Grayscale -> Split Toning -> Lens Corrections -> Effects ->
//! Camera Calibration. Adobe's PV2012 curves are closed, so every stage is an
//! approximation and marked `ponytail:` inline. The pass is deterministic and a
//! default (all-`None`) [`PicturaRawSettings`] is a byte-identical no-op.

mod basic;
mod calibration;
mod detail;
mod effects;
mod hsl;
mod lens;
mod ranges;
mod split_toning;
mod tone_curve;

#[cfg(test)]
mod tests;

use pictura_core::{PicturaRawSettings, PixelBuffer};

use crate::types::AdjustError;

/// The shared working set: three `f32` planes plus their dimensions. Stages
/// mutate these in place so no stage quantizes through 8-bit between them.
pub(crate) struct Planes {
    pub(crate) r: Vec<f32>,
    pub(crate) g: Vec<f32>,
    pub(crate) b: Vec<f32>,
    pub(crate) w: usize,
    pub(crate) h: usize,
}

impl Planes {
    pub(crate) fn len(&self) -> usize {
        self.r.len()
    }
}

/// Apply the Pictura Raw controls to `input`, working in `f32` and preserving
/// the channel count and alpha.
pub fn render_pictura_raw(
    input: &PixelBuffer,
    settings: &PicturaRawSettings,
) -> Result<PixelBuffer, AdjustError> {
    let ch = input.channels as usize;
    if ch != 3 && ch != 4 {
        return Err(AdjustError::Unsupported(format!(
            "channel count {ch} is not supported (expected 3 or 4)"
        )));
    }
    let n = input.pixel_count();
    if n == 0 {
        return Err(AdjustError::InvalidParams("empty buffer".into()));
    }
    if input.data.len() != n * ch {
        return Err(AdjustError::InvalidParams(
            "buffer length does not match width*height*channels".into(),
        ));
    }
    // An entirely unset settings value is a byte-identical no-op.
    if settings == &PicturaRawSettings::default() {
        return Ok(input.clone());
    }

    let (w, h) = (input.width as usize, input.height as usize);
    let mut p = Planes {
        r: vec![0f32; n],
        g: vec![0f32; n],
        b: vec![0f32; n],
        w,
        h,
    };
    for i in 0..n {
        p.r[i] = input.data[i] as f32 / 255.0;
        p.g[i] = input.data[n + i] as f32 / 255.0;
        p.b[i] = input.data[2 * n + i] as f32 / 255.0;
    }

    basic::apply(&mut p, &settings.basic);
    tone_curve::apply(&mut p, &settings.tone_curve);
    detail::apply(&mut p, &settings.detail);
    hsl::apply(&mut p, &settings.hsl, settings.mode.grayscale);
    split_toning::apply(&mut p, &settings.split_toning);
    lens::apply(&mut p, &settings.lens);
    effects::apply(&mut p, &settings.effects);
    calibration::apply(&mut p, &settings.calibration);

    let mut out = PixelBuffer::new(input.width, input.height, input.channels);
    for i in 0..n {
        out.data[i] = (p.r[i] * 255.0).round().clamp(0.0, 255.0) as u8;
        out.data[n + i] = (p.g[i] * 255.0).round().clamp(0.0, 255.0) as u8;
        out.data[2 * n + i] = (p.b[i] * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    if ch == 4 {
        out.data[3 * n..4 * n].copy_from_slice(&input.data[3 * n..4 * n]);
    }
    Ok(out)
}

/// Separable box blur with clamped edges, `O(pixels)`. `radius` is the half
/// window; the caller keeps it below `min(w, h)`.
pub(crate) fn box_blur(src: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let mut tmp = vec![0f32; src.len()];
    let mut out = vec![0f32; src.len()];
    let win = (2 * radius + 1) as f32;
    for y in 0..h {
        let row = y * w;
        let mut sum = src[row] * radius as f32;
        for k in 0..=radius {
            sum += src[row + k.min(w - 1)];
        }
        tmp[row] = sum / win;
        for x in 1..w {
            sum += src[row + (x + radius).min(w - 1)] - src[row + (x - 1).saturating_sub(radius)];
            tmp[row + x] = sum / win;
        }
    }
    for x in 0..w {
        let mut sum = tmp[x] * radius as f32;
        for k in 0..=radius {
            sum += tmp[k.min(h - 1) * w + x];
        }
        out[x] = sum / win;
        for y in 1..h {
            sum +=
                tmp[(y + radius).min(h - 1) * w + x] - tmp[(y - 1).saturating_sub(radius) * w + x];
            out[y * w + x] = sum / win;
        }
    }
    out
}
