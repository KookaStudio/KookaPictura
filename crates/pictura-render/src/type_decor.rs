//! Underline and strikethrough: one rectangle per laid-out line, painted after
//! the glyphs, either straight into the layer buffer or mapped through the
//! type's matrix.
//!
//! Horizontal type underlines below and strikes through each line from its
//! pen start to its last advance. Vertical type draws both as columns: the
//! underline down the column's right (CS6's default Underline Right), the
//! strikethrough down its centre.

use pictura_core::{PixelBuffer, TextLayout};

use crate::type_style::RunStyle;

/// The decoration rectangles `[x0, y0, x1, y1]` for `layout`, in the frame
/// glyphs paint in (a glyph's baseline is its `y + ascent`).
pub(crate) fn decorations(
    run: &RunStyle,
    layout: &TextLayout,
    vertical: bool,
    ascent: f32,
) -> Vec<[f32; 4]> {
    let mut out = Vec::new();
    for line in &layout.lines {
        let Some(first) = line.glyphs.first() else {
            continue;
        };
        if vertical {
            let (y0, y1) = (first.y, first.y + line.advance);
            let centre = line.baseline;
            if let Some((offset, thickness)) = run.underline {
                let x = centre + run.size * run.h / 2.0 + offset;
                out.push([x - thickness / 2.0, y0, x + thickness / 2.0, y1]);
            }
            if let Some((_, thickness)) = run.strike {
                out.push([centre - thickness / 2.0, y0, centre + thickness / 2.0, y1]);
            }
        } else {
            let (x0, x1) = (first.x, first.x + line.advance);
            let baseline = line.baseline + ascent - run.rise;
            if let Some((offset, thickness)) = run.underline {
                let y = baseline + offset;
                out.push([x0, y - thickness / 2.0, x1, y + thickness / 2.0]);
            }
            if let Some((offset, thickness)) = run.strike {
                let y = baseline - offset;
                out.push([x0, y - thickness / 2.0, x1, y + thickness / 2.0]);
            }
        }
    }
    out
}

/// Raise the pixel at `index` to `coverage` (0..=1) in `color`, keeping the
/// higher coverage of the two.
fn cover(out: &mut PixelBuffer, index: usize, coverage: f32, color: [f64; 4]) {
    let n = out.pixel_count();
    let alpha = ((coverage.clamp(0.0, 1.0) * 255.0).round() as u8).max(out.data[3 * n + index]);
    if alpha == 0 {
        return;
    }
    for (channel, value) in color.iter().take(3).enumerate() {
        out.data[channel * n + index] = (f64::from(alpha) / 255.0 * value * 255.0).round() as u8;
    }
    out.data[3 * n + index] = alpha;
}

/// Fill `rect` into `out` with area coverage at its fractional edges. Whether
/// any pixel painted.
pub(crate) fn fill_rect(out: &mut PixelBuffer, rect: [f32; 4], color: [f64; 4]) -> bool {
    let [x0, y0, x1, y1] = rect;
    let (width, height) = (out.width as i32, out.height as i32);
    let overlap = |a: f32, lo: f32, hi: f32| (hi.min(a + 1.0) - lo.max(a)).max(0.0);
    let mut painted = false;
    for y in (y0.floor() as i32).max(0)..(y1.ceil() as i32).min(height) {
        let cy = overlap(y as f32, y0, y1);
        for x in (x0.floor() as i32).max(0)..(x1.ceil() as i32).min(width) {
            let coverage = cy * overlap(x as f32, x0, x1);
            if coverage > 0.0 {
                cover(
                    out,
                    y as usize * width as usize + x as usize,
                    coverage,
                    color,
                );
                painted = true;
            }
        }
    }
    painted
}

/// Fill `rect` mapped by the affine `map` (rect frame to `out` pixels),
/// sampling each pixel 4 × 4. Whether any pixel painted.
pub(crate) fn fill_mapped(
    out: &mut PixelBuffer,
    rect: [f32; 4],
    map: impl Fn(f32, f32) -> (f32, f32),
    color: [f64; 4],
) -> bool {
    let [x0, y0, x1, y1] = rect;
    let origin = map(x0, y0);
    let e1 = {
        let p = map(x1, y0);
        (p.0 - origin.0, p.1 - origin.1)
    };
    let e2 = {
        let p = map(x0, y1);
        (p.0 - origin.0, p.1 - origin.1)
    };
    let det = e1.0 * e2.1 - e1.1 * e2.0;
    if det.abs() < 1e-6 {
        return false;
    }
    let corners = [
        origin,
        (origin.0 + e1.0, origin.1 + e1.1),
        (origin.0 + e2.0, origin.1 + e2.1),
        (origin.0 + e1.0 + e2.0, origin.1 + e1.1 + e2.1),
    ];
    let lo = |f: fn(&(f32, f32)) -> f32| corners.iter().map(f).fold(f32::MAX, f32::min);
    let hi = |f: fn(&(f32, f32)) -> f32| corners.iter().map(f).fold(f32::MIN, f32::max);
    let (width, height) = (out.width as i32, out.height as i32);
    let inside = |qx: f32, qy: f32| {
        let (dx, dy) = (qx - origin.0, qy - origin.1);
        let s = (dx * e2.1 - dy * e2.0) / det;
        let t = (e1.0 * dy - e1.1 * dx) / det;
        (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t)
    };
    let mut painted = false;
    for y in (lo(|p| p.1).floor() as i32).max(0)..(hi(|p| p.1).ceil() as i32).min(height) {
        for x in (lo(|p| p.0).floor() as i32).max(0)..(hi(|p| p.0).ceil() as i32).min(width) {
            let hits = (0..16)
                .filter(|k| {
                    let (sx, sy) = ((k % 4) as f32 + 0.5, (k / 4) as f32 + 0.5);
                    inside(x as f32 + sx / 4.0, y as f32 + sy / 4.0)
                })
                .count();
            if hits > 0 {
                cover(
                    out,
                    y as usize * width as usize + x as usize,
                    hits as f32 / 16.0,
                    color,
                );
                painted = true;
            }
        }
    }
    painted
}
