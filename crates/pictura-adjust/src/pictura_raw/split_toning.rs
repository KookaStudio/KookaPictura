//! The Split Toning tab: hue + saturation tints for shadows and highlights,
//! with a balance that biases the crossover luminance.

use pictura_core::SplitToning;

use crate::common::hsl_to_rgb;

use super::Planes;

pub(crate) fn apply(p: &mut Planes, s: &SplitToning) {
    let sh_sat = s.shadow_saturation.unwrap_or(0);
    let hi_sat = s.highlight_saturation.unwrap_or(0);
    if sh_sat == 0 && hi_sat == 0 {
        return;
    }
    let balance = s.balance.unwrap_or(0) as f64 / 100.0;
    let mid = (0.5 + balance * 0.5).clamp(0.05, 0.95);
    let shadow = tint(s.shadow_hue.unwrap_or(0), sh_sat);
    let highlight = tint(s.highlight_hue.unwrap_or(0), hi_sat);
    for i in 0..p.len() {
        let y =
            (0.299 * p.r[i] as f64 + 0.587 * p.g[i] as f64 + 0.114 * p.b[i] as f64).clamp(0.0, 1.0);
        let sw = ((mid - y) / mid).max(0.0);
        let hw = ((y - mid) / (1.0 - mid)).max(0.0);
        p.r[i] = (p.r[i] as f64 + sw * shadow.0 + hw * highlight.0).clamp(0.0, 1.0) as f32;
        p.g[i] = (p.g[i] as f64 + sw * shadow.1 + hw * highlight.1).clamp(0.0, 1.0) as f32;
        p.b[i] = (p.b[i] as f64 + sw * shadow.2 + hw * highlight.2).clamp(0.0, 1.0) as f32;
    }
}

/// A tint's signed RGB offset from grey, from a hue (degrees) and saturation
/// (0..=100). ponytail: the saturation-to-strength scale is approximate.
fn tint(hue: i32, sat: i32) -> (f64, f64, f64) {
    let a = sat as f64 / 100.0 * 0.25;
    let (r, g, b) = hsl_to_rgb(hue.rem_euclid(360) as f64, 1.0, 0.5);
    ((r - 0.5) * a, (g - 0.5) * a, (b - 0.5) * a)
}
