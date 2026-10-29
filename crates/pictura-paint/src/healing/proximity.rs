//! Spot Healing's Proximity Match: the margin a spot heal rebuilds past the
//! brush edge, and the search for a source around the hole
//! (`docs/03-tools/healing-brushes.md`, "Spot Healing source search").
//!
//! Adobe's metric is undocumented. ponytail: the margin fraction and bounds,
//! the context width, the interior weight, and the coarse grid are inferred,
//! not measured against Photoshop.

use super::{RgbaImage, Work};
use pictura_core::PsdRect;

/// Coverage at or above which a pixel is part of the brush core.
const CORE_COVERAGE: f32 = 0.99;

/// The margin as a fraction of the core's thickness, and its bounds in pixels.
const MARGIN_FRACTION: f32 = 0.3;
const MIN_MARGIN: i32 = 3;
const MAX_MARGIN: i32 = 8;

/// How far outside the hole the context compared between candidates reaches.
const CONTEXT: f32 = 4.0;

/// The weight of a candidate's interior against the smooth fill, relative to
/// its context match. Keeps a neighbouring defect from being chosen.
const INTERIOR_WEIGHT: f32 = 0.5;

/// Per-axis candidate count of the coarse search grid.
const COARSE_STEPS: i32 = 48;

/// The most context or interior pixels a candidate is scored on.
const MAX_SAMPLES: usize = 1024;

/// Distance from each pixel of a `w`×`h` grid to the nearest `seed` pixel, by a
/// two-pass chamfer (steps 1 and √2). `f32::INFINITY` when there is no seed.
pub(super) fn distance_to(seed: &[bool], w: usize, h: usize) -> Vec<f32> {
    let diag = std::f32::consts::SQRT_2;
    let mut d: Vec<f32> = seed
        .iter()
        .map(|&s| if s { 0.0 } else { f32::INFINITY })
        .collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + 1.0);
            }
            if y > 0 {
                v = v.min(d[i - w] + 1.0);
                if x > 0 {
                    v = v.min(d[i - w - 1] + diag);
                }
                if x + 1 < w {
                    v = v.min(d[i - w + 1] + diag);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = d[i];
            if x + 1 < w {
                v = v.min(d[i + 1] + 1.0);
            }
            if y + 1 < h {
                v = v.min(d[i + w] + 1.0);
                if x + 1 < w {
                    v = v.min(d[i + w + 1] + diag);
                }
                if x > 0 {
                    v = v.min(d[i + w - 1] + diag);
                }
            }
            d[i] = v;
        }
    }
    d
}

/// The brush core of a coverage mask: its fully covered pixels.
pub(super) fn core(coverage: &[f32]) -> Vec<bool> {
    coverage.iter().map(|&c| c >= CORE_COVERAGE).collect()
}

/// How far past the brush core a spot heal rebuilds, from the core's
/// thickness (about the brush radius for a click or a drag). Zero without a
/// fully covered pixel, so a soft or partial mask keeps its plain blend.
pub(super) fn margin(region: PsdRect, coverage: &[f32]) -> i32 {
    let (rw, rh) = (
        region.width().max(0) as usize,
        region.height().max(0) as usize,
    );
    if coverage.len() != rw * rh {
        return 0;
    }
    let inside = core(coverage);
    if !inside.iter().any(|&c| c) {
        return 0;
    }
    // Pad by one so the region edge counts as outside the core.
    let (pw, ph) = (rw + 2, rh + 2);
    let mut outside = vec![true; pw * ph];
    for y in 0..rh {
        for x in 0..rw {
            outside[(y + 1) * pw + x + 1] = !inside[y * rw + x];
        }
    }
    let thickness = distance_to(&outside, pw, ph)
        .into_iter()
        .fold(0.0f32, f32::max);
    ((thickness * MARGIN_FRACTION).round() as i32).clamp(MIN_MARGIN, MAX_MARGIN)
}

/// Write-back weight of a pixel `d` from the core with margin `m`: full across
/// the margin, so a defect's rim inside it is wholly rebuilt, then falling to
/// zero over one more pixel. The fill already meets its boundary there; the
/// fade only softens the last step.
pub(super) fn margin_weight(d: f32, m: i32) -> f32 {
    ((m + 1) as f32 - d).clamp(0.0, 1.0)
}

/// The offset of the best source for the hole in `work`: its surroundings
/// match the hole's (`dest`), and its interior is closest to the `smooth`
/// fill. A candidate must lie inside `img` and not read the hole. `None` when
/// no candidate fits.
pub(super) fn best_source(
    img: &RgbaImage,
    work: &Work,
    dest: &[[f32; 4]],
    smooth: &[[f32; 4]],
) -> Option<(i32, i32)> {
    let (w, h) = (work.w, work.h);
    let near = distance_to(&work.hole, w, h);
    let mut context = Vec::new();
    let mut interior = Vec::new();
    let mut hole = None;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let at = (work.rect.left + x as i32, work.rect.top + y as i32, i);
            if work.hole[i] {
                interior.push(at);
                hole = Some(grow(hole, at.0, at.1));
            } else if near[i] <= CONTEXT {
                context.push(at);
            }
        }
    }
    let hole = hole?;
    if context.is_empty() {
        return None;
    }
    let context = thin(context);
    let interior = thin(interior);
    let footprint = super::inflate(hole, CONTEXT as i32 + 1);
    let canvas = img.rect();
    let fits = |(dx, dy): (i32, i32)| {
        let f = shift(footprint, dx, dy);
        f.left >= canvas.left
            && f.top >= canvas.top
            && f.right <= canvas.right
            && f.bottom <= canvas.bottom
            && (f.right <= hole.left
                || f.left >= hole.right
                || f.bottom <= hole.top
                || f.top >= hole.bottom)
    };
    let mse = |set: &[(i32, i32, usize)], want: &[[f32; 4]], (dx, dy): (i32, i32)| {
        let sum: f32 = set
            .iter()
            .map(|&(x, y, i)| {
                let s = img.get(x + dx, y + dy);
                (0..4)
                    .map(|c| (f32::from(s[c]) - want[i][c]).powi(2))
                    .sum::<f32>()
            })
            .sum();
        sum / set.len().max(1) as f32
    };
    let score = |off| mse(&context, dest, off) + INTERIOR_WEIGHT * mse(&interior, smooth, off);

    let span = hole.width().max(hole.height());
    let reach = 2 * span + 2 * CONTEXT as i32 + 2;
    let stride = ((2 * reach + 1) + COARSE_STEPS - 1) / COARSE_STEPS;
    // Lowest score wins; a tie goes to the nearer offset, then to scan order.
    let consider = |off: (i32, i32), best: &mut Option<(f32, i32, (i32, i32))>| {
        if !fits(off) {
            return;
        }
        let s = score(off);
        let d2 = off.0 * off.0 + off.1 * off.1;
        if best.is_none_or(|(bs, bd, _)| s < bs || (s == bs && d2 < bd)) {
            *best = Some((s, d2, off));
        }
    };
    let mut best = None;
    for dy in (-reach..=reach).step_by(stride as usize) {
        for dx in (-reach..=reach).step_by(stride as usize) {
            consider((dx, dy), &mut best);
        }
    }
    if stride > 1 {
        let (_, _, (cx, cy)) = best?;
        for dy in cy - stride + 1..cy + stride {
            for dx in cx - stride + 1..cx + stride {
                consider((dx, dy), &mut best);
            }
        }
    }
    best.map(|(_, _, off)| off)
}

/// Every k-th sample, so at most [`MAX_SAMPLES`] remain.
fn thin<T: Copy>(v: Vec<T>) -> Vec<T> {
    let k = v.len().div_ceil(MAX_SAMPLES).max(1);
    v.into_iter().step_by(k).collect()
}

/// `r` grown to include pixel `(x, y)`.
fn grow(r: Option<PsdRect>, x: i32, y: i32) -> PsdRect {
    let px = PsdRect {
        top: y,
        left: x,
        bottom: y + 1,
        right: x + 1,
    };
    r.map_or(px, |r| PsdRect {
        top: r.top.min(px.top),
        left: r.left.min(px.left),
        bottom: r.bottom.max(px.bottom),
        right: r.right.max(px.right),
    })
}

fn shift(r: PsdRect, dx: i32, dy: i32) -> PsdRect {
    PsdRect {
        top: r.top + dy,
        left: r.left + dx,
        bottom: r.bottom + dy,
        right: r.right + dx,
    }
}
