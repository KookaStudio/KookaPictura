//! The Effects tab: film grain, Dehaze, and post-crop vignette.

use pictura_core::Effects;

use super::{box_blur, Planes};

pub(crate) fn apply(p: &mut Planes, e: &Effects) {
    let grain = e.grain_amount.unwrap_or(0);
    let dehaze = e.dehaze.unwrap_or(0);
    let vig = e.vignette_amount.unwrap_or(0);
    if grain == 0 && dehaze == 0 && vig == 0 {
        return;
    }
    if dehaze != 0 {
        apply_dehaze(p, dehaze);
    }
    if grain != 0 {
        apply_grain(p, e, grain);
    }
    if vig != 0 {
        apply_vignette(p, e, vig);
    }
}

/// Dehaze as a contrast stretch with a black-point lift. ponytail: the
/// reference's atmospheric model is closed.
fn apply_dehaze(p: &mut Planes, amount: i32) {
    let a = amount as f32 / 100.0;
    let scale = 1.0 + a * 0.4;
    let black = a * 0.05;
    for i in 0..p.len() {
        p.r[i] = ((p.r[i] - black) * scale).clamp(0.0, 1.0);
        p.g[i] = ((p.g[i] - black) * scale).clamp(0.0, 1.0);
        p.b[i] = ((p.b[i] - black) * scale).clamp(0.0, 1.0);
    }
}

/// A deterministic integer hash in `0..1`.
fn hash2(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x
        .wrapping_mul(0x1f1f_1f1f)
        .wrapping_add(y.wrapping_mul(0x27d4_eb2d))
        ^ seed;
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

/// Film grain from a fixed seed so the render is deterministic. ponytail: one
/// additive noise field; the reference's grain spectrum is closed.
fn apply_grain(p: &mut Planes, e: &Effects, amount: i32) {
    let n = p.len();
    let (w, h) = (p.w, p.h);
    let amp = amount as f32 / 100.0 * 0.15;
    let mut field: Vec<f32> = (0..n)
        .map(|i| {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            (hash2(x, y, 0x9e37_79b9) - 0.5) * 2.0
        })
        .collect();
    let size = e.grain_size.unwrap_or(25);
    if size > 25 {
        let radius = (((size - 25) / 25) as usize).clamp(1, 8);
        field = box_blur(&field, w, h, radius);
    }
    let rough = 0.5 + e.grain_frequency.unwrap_or(50) as f32 / 100.0;
    for (i, &f) in field.iter().enumerate() {
        let d = f * amp * rough;
        p.r[i] = (p.r[i] + d).clamp(0.0, 1.0);
        p.g[i] = (p.g[i] + d).clamp(0.0, 1.0);
        p.b[i] = (p.b[i] + d).clamp(0.0, 1.0);
    }
}

/// Post-crop vignette. A positive amount lightens the corners, a negative
/// darkens. `vignette_style` selects Highlight Priority (0), Color Priority
/// (1), or Paint Overlay (2); only Highlight Priority pulls the corners back
/// where the pixel is already bright. ponytail: one radial gain.
fn apply_vignette(p: &mut Planes, e: &Effects, amount: i32) {
    let (w, h) = (p.w, p.h);
    let (cx, cy) = ((w as f32 - 1.0) / 2.0, (h as f32 - 1.0) / 2.0);
    let maxr = (cx * cx + cy * cy).sqrt().max(1.0);
    let mid = (e.vignette_midpoint.unwrap_or(50) as f32 / 100.0).clamp(0.0, 0.95);
    let feather = e.vignette_feather.unwrap_or(50) as f32 / 100.0;
    let k = amount as f32 / 100.0;
    let highlight_priority = e.vignette_style.unwrap_or(0) == 0;
    let highlights = e.vignette_highlights.unwrap_or(0) as f32 / 100.0;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let rn = (dx * dx + dy * dy).sqrt() / maxr;
            let mut t = ((rn - mid) / (1.0 - mid)).clamp(0.0, 1.0);
            t = (t * (0.5 + feather)).min(1.0);
            let i = y * w + x;
            let luma = 0.299 * p.r[i] + 0.587 * p.g[i] + 0.114 * p.b[i];
            let hp = if highlight_priority {
                1.0 - highlights * luma
            } else {
                1.0
            };
            let gain = 1.0 + k * t * hp;
            p.r[i] = (p.r[i] * gain).clamp(0.0, 1.0);
            p.g[i] = (p.g[i] * gain).clamp(0.0, 1.0);
            p.b[i] = (p.b[i] * gain).clamp(0.0, 1.0);
        }
    }
}
