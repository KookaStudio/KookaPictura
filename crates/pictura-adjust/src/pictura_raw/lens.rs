//! The Lens Corrections tab: manual distortion, defringe, and vignette.

use pictura_core::Lens;

use crate::common::{hsl_to_rgb, rgb_to_hsl};

use super::Planes;

pub(crate) fn apply(p: &mut Planes, l: &Lens) {
    let distortion = l.distortion.unwrap_or(0);
    let purple = l.defringe_purple_amount.unwrap_or(0);
    let green = l.defringe_green_amount.unwrap_or(0);
    let vig_amount = l.vignette_amount.unwrap_or(0);
    if distortion == 0 && purple == 0 && green == 0 && vig_amount == 0 {
        return;
    }
    if distortion != 0 {
        apply_distortion(p, distortion);
    }
    if purple != 0 || green != 0 {
        apply_defringe(p, l, purple, green);
    }
    if vig_amount != 0 {
        apply_vignette(p, l, vig_amount);
    }
}

/// Radial barrel/pincushion remap by nearest sampling. ponytail: nearest
/// neighbour and a single quadratic term; no interpolation or profile model.
fn apply_distortion(p: &mut Planes, amount: i32) {
    let (w, h) = (p.w, p.h);
    let (cx, cy) = ((w as f32 - 1.0) / 2.0, (h as f32 - 1.0) / 2.0);
    let maxr2 = (cx * cx + cy * cy).max(1.0);
    let k = amount as f32 / 100.0 * 0.25;
    let (sr, sg, sb) = (p.r.clone(), p.g.clone(), p.b.clone());
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let f = 1.0 + k * (dx * dx + dy * dy) / maxr2;
            let sx = (cx + dx * f).round().clamp(0.0, w as f32 - 1.0) as usize;
            let sy = (cy + dy * f).round().clamp(0.0, h as f32 - 1.0) as usize;
            let (si, i) = (sy * w + sx, y * w + x);
            p.r[i] = sr[si];
            p.g[i] = sg[si];
            p.b[i] = sb[si];
        }
    }
}

/// Desaturate pixels whose hue falls in a fringe window. Each fringe targets a
/// band around its nominal hue: the stored `lo`/`hi` are the reference's 0..100
/// slider positions, mapped onto `center +/- 60` degrees. ponytail: the band
/// width and the reference's highlight-edge gating are closed.
fn apply_defringe(p: &mut Planes, l: &Lens, purple: i32, green: i32) {
    const PURPLE_CENTER: f64 = 300.0;
    const GREEN_CENTER: f64 = 120.0;
    const HALF_SPAN: f64 = 60.0;
    let window = |lo: i32, hi: i32, center: f64| {
        let a = center + (lo as f64 / 100.0 - 0.5) * 2.0 * HALF_SPAN;
        let b = center + (hi as f64 / 100.0 - 0.5) * 2.0 * HALF_SPAN;
        (a.min(b), a.max(b))
    };
    let windows = [
        (
            purple,
            window(
                l.defringe_purple_hue_lo.unwrap_or(30),
                l.defringe_purple_hue_hi.unwrap_or(70),
                PURPLE_CENTER,
            ),
        ),
        (
            green,
            window(
                l.defringe_green_hue_lo.unwrap_or(40),
                l.defringe_green_hue_hi.unwrap_or(60),
                GREEN_CENTER,
            ),
        ),
    ];
    for i in 0..p.len() {
        let (hue, s, li) = rgb_to_hsl(p.r[i] as f64, p.g[i] as f64, p.b[i] as f64);
        let mut k = 0.0f64;
        for (amount, (lo, hi)) in windows {
            if amount == 0 {
                continue;
            }
            let inside = if lo <= hi {
                hue >= lo && hue <= hi
            } else {
                hue >= lo || hue <= hi
            };
            if inside {
                k = k.max(amount as f64 / 100.0);
            }
        }
        if k > 0.0 {
            let (nr, ng, nb) = hsl_to_rgb(hue, (s * (1.0 - k)).clamp(0.0, 1.0), li);
            p.r[i] = nr as f32;
            p.g[i] = ng as f32;
            p.b[i] = nb as f32;
        }
    }
}

/// Radial gain: a positive amount lightens the corners, a negative darkens.
fn apply_vignette(p: &mut Planes, l: &Lens, amount: i32) {
    let (w, h) = (p.w, p.h);
    let (cx, cy) = ((w as f32 - 1.0) / 2.0, (h as f32 - 1.0) / 2.0);
    let maxr = (cx * cx + cy * cy).sqrt().max(1.0);
    let mid = (l.vignette_midpoint.unwrap_or(50) as f32 / 100.0).clamp(0.0, 0.95);
    let k = amount as f32 / 100.0;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let rn = (dx * dx + dy * dy).sqrt() / maxr;
            let t = ((rn - mid) / (1.0 - mid)).clamp(0.0, 1.0);
            let gain = 1.0 + k * t;
            let i = y * w + x;
            p.r[i] = (p.r[i] * gain).clamp(0.0, 1.0);
            p.g[i] = (p.g[i] * gain).clamp(0.0, 1.0);
            p.b[i] = (p.b[i] * gain).clamp(0.0, 1.0);
        }
    }
}
