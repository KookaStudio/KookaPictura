//! Pixel compositing: how one source colour laid at a strength over a straight
//! pixel in a paint mode becomes the stored RGBA.

use crate::{PaintMode, Rgba};

/// `s` laid at strength `a` (0.0–1.0) over the straight pixel `dst` in `mode`,
/// or `None` when the pixel is left alone. `roll` (0.0–1.0) decides whether a
/// Dissolve pixel is painted.
pub(crate) fn blend_pixel(
    mode: PaintMode,
    dst: (u8, u8, u8, u8),
    s: &Rgba,
    a: f32,
    roll: f32,
) -> Option<(u8, u8, u8, u8)> {
    let (dr, dg, db, da) = dst;
    match mode {
        PaintMode::Normal => Some(normal_pixel(dr, dg, db, da, s, a)),
        PaintMode::Multiply => {
            // W3C separable blending: the product shows in proportion to the
            // backdrop's alpha, so a transparent pixel takes the plain colour.
            let da_f = da as f32 / 255.0;
            let m = |src: u8, dst: u8| {
                let src = src as f32;
                to_u8((1.0 - da_f) * src + da_f * src * dst as f32 / 255.0)
            };
            let product = Rgba {
                r: m(s.r, dr),
                g: m(s.g, dg),
                b: m(s.b, db),
                a: s.a,
            };
            Some(normal_pixel(dr, dg, db, da, &product, a))
        }
        PaintMode::Dissolve => (roll < a).then_some((s.r, s.g, s.b, s.a)),
        PaintMode::Behind => (da != 255).then(|| behind_pixel(dr, dg, db, da, s, a)),
        PaintMode::Clear => {
            let out_a = (da as f32 / 255.0) * (1.0 - a);
            let (r, g, b) = if a >= 1.0 { (0, 0, 0) } else { (dr, dg, db) };
            Some((r, g, b, to_u8(out_a * 255.0)))
        }
    }
}

fn normal_pixel(dr: u8, dg: u8, db: u8, da: u8, s: &Rgba, a: f32) -> (u8, u8, u8, u8) {
    let sa = s.a as f32 / 255.0;
    let da_f = da as f32 / 255.0;
    let src_a = sa * a;
    let out_a = src_a + da_f * (1.0 - src_a);
    if out_a <= 0.0 {
        return (0, 0, 0, 0);
    }
    let mix = |src: u8, dst: u8| {
        ((src as f32 * src_a + dst as f32 * da_f * (1.0 - src_a)) / out_a).round() as u8
    };
    (
        mix(s.r, dr),
        mix(s.g, dg),
        mix(s.b, db),
        to_u8(out_a * 255.0),
    )
}

fn behind_pixel(dr: u8, dg: u8, db: u8, da: u8, s: &Rgba, a: f32) -> (u8, u8, u8, u8) {
    let sa = s.a as f32 / 255.0;
    let da_f = da as f32 / 255.0;
    let src_a = sa * a;
    let out_a = da_f + src_a * (1.0 - da_f);
    if out_a <= 0.0 {
        return (0, 0, 0, 0);
    }
    let mix = |src: u8, dst: u8| {
        ((dst as f32 * da_f + src as f32 * src_a * (1.0 - da_f)) / out_a).round() as u8
    };
    (
        mix(s.r, dr),
        mix(s.g, dg),
        mix(s.b, db),
        to_u8(out_a * 255.0),
    )
}

fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}
