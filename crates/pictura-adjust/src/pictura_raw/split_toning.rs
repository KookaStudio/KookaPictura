//! The Split Toning tab: hue + saturation tints for shadows and highlights,
//! with a balance that biases the crossover luminance.

use pictura_core::SplitToning;

use crate::common::{hsl_to_rgb, rgb_to_hsl};

use super::Planes;

/// Half the width of the shadow-to-highlight crossover. The two tints overlap
/// across the midtones, so a moderate saturation is visible on the upper
/// midtones a highlight tint is aimed at, not only near white.
const CROSSOVER: f64 = 0.25;

pub(crate) fn apply(p: &mut Planes, s: &SplitToning) {
    let shadow_strength = s.shadow_saturation.unwrap_or(0).clamp(0, 100) as f64 / 100.0;
    let highlight_strength = s.highlight_saturation.unwrap_or(0).clamp(0, 100) as f64 / 100.0;
    if shadow_strength <= 0.0 && highlight_strength <= 0.0 {
        return;
    }
    // A positive balance moves the pivot toward the shadows, so the highlight
    // tint reaches further down the tonal range (`+ to Highlights`).
    let balance = s.balance.unwrap_or(0) as f64 / 100.0;
    let pivot = (0.5 - balance * 0.5).clamp(0.05, 0.95);
    let shadow_hue = s.shadow_hue.unwrap_or(0).rem_euclid(360) as f64;
    let highlight_hue = s.highlight_hue.unwrap_or(0).rem_euclid(360) as f64;
    // ponytail: the crossover width and the fully saturated tint are a visible
    // approximation of Camera Raw's split toning, not a measured match.
    for i in 0..p.len() {
        let (r, g, b) = (p.r[i] as f64, p.g[i] as f64, p.b[i] as f64);
        let y = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0);
        let highlight_weight = smoothstep(pivot - CROSSOVER, pivot + CROSSOVER, y);
        let shadow_mix = (1.0 - highlight_weight) * shadow_strength;
        let highlight_mix = highlight_weight * highlight_strength;
        let total = shadow_mix + highlight_mix;
        if total <= 0.0 {
            continue;
        }
        // Blend toward each active tint at the pixel's own lightness, then
        // scale back to the original luminance so the tonal value is kept.
        let (_, _, lightness) = rgb_to_hsl(r, g, b);
        let mut tr = 0.0;
        let mut tg = 0.0;
        let mut tb = 0.0;
        if shadow_mix > 0.0 {
            let (sr, sg, sb) = hsl_to_rgb(shadow_hue, 1.0, lightness);
            tr += sr * shadow_mix;
            tg += sg * shadow_mix;
            tb += sb * shadow_mix;
        }
        if highlight_mix > 0.0 {
            let (hr, hg, hb) = hsl_to_rgb(highlight_hue, 1.0, lightness);
            tr += hr * highlight_mix;
            tg += hg * highlight_mix;
            tb += hb * highlight_mix;
        }
        let mix = total.min(1.0);
        let mut nr = r + (tr / total - r) * mix;
        let mut ng = g + (tg / total - g) * mix;
        let mut nb = b + (tb / total - b) * mix;
        let luma = 0.299 * nr + 0.587 * ng + 0.114 * nb;
        if luma > 1e-9 {
            let scale = (y / luma).clamp(0.0, 4.0);
            nr *= scale;
            ng *= scale;
            nb *= scale;
        }
        let peak = nr.max(ng).max(nb);
        if peak > 1.0 {
            // Desaturate toward the neutral of the same luma until the colour
            // fits, so the tonal value is kept instead of clipping the peak.
            let t = (peak - 1.0) / (peak - y).max(1e-9);
            nr += (y - nr) * t;
            ng += (y - ng) * t;
            nb += (y - nb) * t;
        }
        p.r[i] = nr.clamp(0.0, 1.0) as f32;
        p.g[i] = ng.clamp(0.0, 1.0) as f32;
        p.b[i] = nb.clamp(0.0, 1.0) as f32;
    }
}

/// A smooth 0→1 ramp between `edge0` and `edge1`, flat outside the edges.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge1 <= edge0 {
        return if x >= edge1 { 1.0 } else { 0.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
