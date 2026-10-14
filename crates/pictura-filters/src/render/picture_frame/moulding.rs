//! The ruled frames (36 to 41) and moulded frames (42 to 47): lines and a
//! bevelled band drawn from a corner distance field.
//!
//! Ported from perfecto25/photorust (`core/src/filters/frame.rs`), GPL-3.0.

use std::f32::consts::PI;

use super::scale::{Scale, UNIT_AT_FULL};
use crate::photorust::pixmap::{source_over, Pixmap, Rgba8};

// --------------------------------------------------------------- moulding --

/// How wide a ruled or moulded frame is, and how far its corner shapes reach,
/// each at Size 100 as a share of the canvas's shorter side.
const BAND_AT_FULL: f32 = 0.25;
const CORNER_PER_BAND: f32 = 2.4;

/// The six corners the ruled frames (36 to 41) and the moulded ones (42 to 47)
/// share, in CS6's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Corner {
    Square,
    Rounded,
    Inverse,
    InverseSmall,
    Dual,
    DualSmall,
}

/// How deep a point is inside the frame's outline, `(u, v)` being its distance
/// in from the nearer vertical and horizontal edges of the outline: the
/// distance to the outline, positive inside. Worked out on one corner and
/// folded onto the other three, so the frame is symmetrical by construction.
fn depth(corner: Corner, u: f32, v: f32, reach: f32) -> f32 {
    let straight = u.min(v);
    match corner {
        Corner::Square => straight,
        Corner::Rounded => {
            if u < reach && v < reach {
                reach - ((reach - u).powi(2) + (reach - v).powi(2)).sqrt()
            } else {
                straight
            }
        }
        Corner::Inverse => straight.min((u * u + v * v).sqrt() - reach),
        Corner::InverseSmall => straight.min((u * u + v * v).sqrt() - reach * 0.55),
        Corner::Dual | Corner::DualSmall => {
            // Two scallops, one off each edge, meeting in a point on the
            // diagonal.
            let r = if corner == Corner::Dual {
                reach * 0.75
            } else {
                reach * 0.45
            };
            let c = r * 0.93;
            straight
                .min(((u - c).powi(2) + v * v).sqrt() - r)
                .min((u * u + (v - c).powi(2)).sqrt() - r)
        }
    }
}

/// The ruled frames (36 to 41) — lines following the outline, one heavy and one
/// fine — and the moulded ones (42 to 47), a bevelled band lit from the top
/// left, both in the vine colour.
///
/// Drawn from a distance field rather than as outlines: every line and every
/// step of the moulding is a fixed depth inside the same outline, so the
/// corners stay parallel however elaborate they are, which offsetting an
/// outline would not guarantee.
pub(super) fn moulding(
    pixmap: &mut Pixmap,
    frame: u32,
    colour: Rgba8,
    scale: &Scale,
    invert: bool,
) {
    let corner = match (frame - 36) % 6 {
        0 => Corner::Square,
        1 => Corner::Rounded,
        2 => Corner::Inverse,
        3 => Corner::InverseSmall,
        4 => Corner::Dual,
        _ => Corner::DualSmall,
    };
    let art = frame >= 42;
    // The corner shapes follow Size; the band's own width follows Thickness as
    // well.
    let reach = scale.unit / UNIT_AT_FULL * BAND_AT_FULL * CORNER_PER_BAND;
    let band = (scale.unit / UNIT_AT_FULL * BAND_AT_FULL * scale.thickness).min(reach * 1.5);
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    let inset = scale.inset;
    let deep = |x: f32, y: f32| {
        let u = x.min(w - x) - inset;
        let v = y.min(h - y) - inset;
        depth(corner, u, v, reach)
    };
    let heavy = (band * 0.28).max(1.0);
    let fine = (band * 0.1).max(1.0);
    let base = [colour.r as f32, colour.g as f32, colour.b as f32];
    let width = pixmap.width() as i32;
    let reach_in = (inset + band + reach + 2.0) as i32;

    for y in 0..pixmap.height() as i32 {
        let near_y = y < reach_in || y >= pixmap.height() as i32 - reach_in;
        for x in 0..width {
            // Only the border can be touched; skip the middle outright.
            if !near_y && x >= reach_in && x < width - reach_in {
                continue;
            }
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = deep(px, py);
            if d < -1.0 || d > band + 1.0 {
                continue;
            }
            let (tone, cover) = if art {
                // The moulding's profile, outside in: a dark outer edge, a
                // rounded bead lit by its slope, a pale flat and a dark lip.
                let cover = (d + 0.5).clamp(0.0, 1.0) * (band - d + 0.5).clamp(0.0, 1.0);
                // Inverted, the profile runs the other way across the band, so
                // what was lit is in shadow.
                let t = (d / band).clamp(0.0, 1.0);
                let t = if invert { 1.0 - t } else { t };
                let tone = if t < 0.1 {
                    0.55
                } else if t < 0.7 {
                    let slope = (PI * (t - 0.1) / 0.6).cos();
                    // Which way the band faces: the gradient of the depth.
                    let e = 0.5;
                    let gx = deep(px + e, py) - deep(px - e, py);
                    let gy = deep(px, py + e) - deep(px, py - e);
                    let g = (gx * gx + gy * gy).sqrt().max(1e-3);
                    // The bead rises going in, so its outer slope faces out and
                    // its inner slope faces in; the light is at the top left,
                    // facing down and right.
                    let facing = -(gx / g * -0.7 + gy / g * -0.7) * slope;
                    1.0 + 0.45 * facing
                } else if t < 0.86 {
                    1.2
                } else {
                    0.6
                };
                (tone, cover)
            } else {
                let line =
                    |at: f32, width: f32| (width * 0.5 - (d - at).abs() + 0.5).clamp(0.0, 1.0);
                // Inverted, the fine line goes outside and the heavy one in.
                let (outer, inner) = if invert { (fine, heavy) } else { (heavy, fine) };
                let cover =
                    line(outer * 0.5, outer).max(line(outer + band * 0.3 + inner * 0.5, inner));
                (1.0, cover)
            };
            if cover <= 0.0 {
                continue;
            }
            let lit = Rgba8::new(
                (base[0] * tone).round().clamp(0.0, 255.0) as u8,
                (base[1] * tone).round().clamp(0.0, 255.0) as u8,
                (base[2] * tone).round().clamp(0.0, 255.0) as u8,
                255,
            );
            let dst = pixmap.get(x, y);
            pixmap.set(x, y, source_over(dst, lit, cover));
        }
    }
}
