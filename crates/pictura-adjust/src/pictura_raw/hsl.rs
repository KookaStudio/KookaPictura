//! The HSL / Grayscale tab: eight-range Hue/Saturation/Luminance, or the eight
//! grayscale-mix weights when grayscale is on.

use pictura_core::Hsl;

use crate::common::{hsl_to_rgb, rgb_to_hsl};

use super::ranges::{hue_weight, HUE_CENTERS};
use super::Planes;

fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// How much a pixel actually has a hue: 0 for a neutral, 1 once it is clearly
/// chromatic. Without it a grey reads as hue 0 and every range edit lands on it.
fn presence(r: f32, g: f32, b: f32) -> f64 {
    let chroma = r.max(g).max(b) - r.min(g).min(b);
    (chroma as f64 * 4.0).min(1.0)
}

pub(crate) fn apply(p: &mut Planes, h: &Hsl, grayscale: Option<bool>) {
    if grayscale == Some(true) {
        apply_grayscale(p, h);
        return;
    }
    let active = (0..8).any(|i| {
        h.hue[i].is_some_and(|v| v != 0)
            || h.saturation[i].is_some_and(|v| v != 0)
            || h.luminance[i].is_some_and(|v| v != 0)
    });
    if !active {
        return;
    }
    let get = |a: &[Option<i32>; 8], i: usize| a[i].unwrap_or(0) as f64;
    for i in 0..p.len() {
        let presence = presence(p.r[i], p.g[i], p.b[i]);
        if presence == 0.0 {
            continue;
        }
        let (hue, s, l) = rgb_to_hsl(p.r[i] as f64, p.g[i] as f64, p.b[i] as f64);
        let (mut dh, mut ds, mut dl) = (0.0, 0.0, 0.0);
        for (r, center) in HUE_CENTERS.iter().enumerate() {
            let w = hue_weight(hue, *center) * presence;
            if w == 0.0 {
                continue;
            }
            dh += w * get(&h.hue, r);
            ds += w * get(&h.saturation, r);
            dl += w * get(&h.luminance, r);
        }
        // ponytail: the slider-to-shift scales are approximate.
        let new_h = (hue + dh * 0.5).rem_euclid(360.0);
        let new_s = (s * (1.0 + ds / 100.0)).clamp(0.0, 1.0);
        let new_l = (l + dl / 100.0 * 0.5).clamp(0.0, 1.0);
        let (nr, ng, nb) = hsl_to_rgb(new_h, new_s, new_l);
        p.r[i] = nr as f32;
        p.g[i] = ng as f32;
        p.b[i] = nb as f32;
    }
}

/// Convert to grayscale, weighting each of the eight ranges by its mix.
/// ponytail: a single additive term per range; the reference's mix is closed.
fn apply_grayscale(p: &mut Planes, h: &Hsl) {
    let any_mix = h.grayscale_mix.iter().any(|v| v.is_some_and(|x| x != 0));
    for i in 0..p.len() {
        let presence = presence(p.r[i], p.g[i], p.b[i]);
        let (hue, _s, _l) = rgb_to_hsl(p.r[i] as f64, p.g[i] as f64, p.b[i] as f64);
        let mut gray = luma(p.r[i], p.g[i], p.b[i]) as f64;
        if any_mix {
            for (r, center) in HUE_CENTERS.iter().enumerate() {
                gray +=
                    hue_weight(hue, *center) * presence * h.grayscale_mix[r].unwrap_or(0) as f64
                        / 100.0
                        * 0.5;
            }
        }
        let gray = gray.clamp(0.0, 1.0) as f32;
        p.r[i] = gray;
        p.g[i] = gray;
        p.b[i] = gray;
    }
}
