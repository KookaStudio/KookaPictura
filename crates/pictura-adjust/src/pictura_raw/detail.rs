//! The Detail tab: sharpening (a variation of Unsharp Mask) and luminance /
//! chroma noise reduction.

use pictura_core::Detail;

use super::{box_blur, Planes};

fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

pub(crate) fn apply(p: &mut Planes, d: &Detail) {
    let amount = d.sharpen_amount.unwrap_or(0);
    let luminance = d.noise_luminance.unwrap_or(0);
    let color = d.noise_color.unwrap_or(0);
    if amount == 0 && luminance == 0 && color == 0 {
        return;
    }
    let n = p.len();
    let (w, h) = (p.w, p.h);

    if luminance > 0 {
        // ponytail: one blur radius for luminance noise; the reference splits
        // detail/contrast by frequency.
        let yplane: Vec<f32> = (0..n).map(|i| luma(p.r[i], p.g[i], p.b[i])).collect();
        let blurred = box_blur(&yplane, w, h, 1);
        let k = luminance as f32 / 100.0;
        for i in 0..n {
            let delta = (blurred[i] - yplane[i]) * k;
            p.r[i] = (p.r[i] + delta).clamp(0.0, 1.0);
            p.g[i] = (p.g[i] + delta).clamp(0.0, 1.0);
            p.b[i] = (p.b[i] + delta).clamp(0.0, 1.0);
        }
    }

    if amount > 0 {
        // ponytail: blur-difference USM with a gradient edge mask; the reference
        // weights detail by frequency and masks on edge strength (closed).
        let yplane: Vec<f32> = (0..n).map(|i| luma(p.r[i], p.g[i], p.b[i])).collect();
        let radius = d.sharpen_radius.unwrap_or(1.0).round().clamp(1.0, 16.0) as usize;
        let blurred = box_blur(&yplane, w, h, radius);
        let gain = amount as f32 / 100.0 * (0.5 + d.sharpen_detail.unwrap_or(0) as f32 / 200.0);
        let masking = d.sharpen_masking.unwrap_or(0) as f32 / 100.0;
        for i in 0..n {
            let detail = yplane[i] - blurred[i];
            let edge = (detail.abs() * 6.0).min(1.0);
            let mask = 1.0 - masking * (1.0 - edge);
            let adj = detail * gain * mask;
            p.r[i] = (p.r[i] + adj).clamp(0.0, 1.0);
            p.g[i] = (p.g[i] + adj).clamp(0.0, 1.0);
            p.b[i] = (p.b[i] + adj).clamp(0.0, 1.0);
        }
    }

    if color > 0 {
        // Chroma noise reduction pulls each pixel toward its luminance.
        let k = color as f32 / 100.0 * 0.8;
        for i in 0..n {
            let y = luma(p.r[i], p.g[i], p.b[i]);
            p.r[i] = (y + (p.r[i] - y) * (1.0 - k)).clamp(0.0, 1.0);
            p.g[i] = (y + (p.g[i] - y) * (1.0 - k)).clamp(0.0, 1.0);
            p.b[i] = (y + (p.b[i] - y) * (1.0 - k)).clamp(0.0, 1.0);
        }
    }
}
