//! Render Grids: antialiased one-pixel lines composited black over a
//! straight-alpha planar buffer.

use pictura_core::PixelBuffer;

use super::geometry::P2;

/// Each pixel's line coverage (0–1): the most any one segment gives, so
/// crossings do not darken twice.
pub(super) fn coverage(w: usize, h: usize, lines: &[[P2; 2]]) -> Vec<f32> {
    let mut cover = vec![0f32; w * h];
    for &[a, b] in lines {
        if ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
            continue;
        }
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        // Walk the major axis and test a band of pixels across it.
        let steep = dy.abs() > dx.abs();
        let (lo, hi) = if steep {
            (a.1.min(b.1), a.1.max(b.1))
        } else {
            (a.0.min(b.0), a.0.max(b.0))
        };
        let major = if steep { h } else { w };
        let minor = if steep { w } else { h };
        let first = (lo - 1.0).floor().max(0.0) as usize;
        let last = ((hi + 1.0).ceil() as isize).min(major as isize - 1);
        if last < 0 || first >= major {
            continue;
        }
        for m in first..=last as usize {
            let mc = m as f64 + 0.5;
            let t = if steep {
                if dy.abs() < 1e-12 {
                    0.0
                } else {
                    (mc - a.1) / dy
                }
            } else if dx.abs() < 1e-12 {
                0.0
            } else {
                (mc - a.0) / dx
            };
            let across = if steep { a.0 + t * dx } else { a.1 + t * dy };
            let from = (across - 2.0).floor().max(0.0) as usize;
            let to = ((across + 2.0).ceil() as isize).min(minor as isize - 1);
            if to < 0 {
                continue;
            }
            for n in from..=to as usize {
                let (x, y) = if steep { (n, m) } else { (m, n) };
                let p = (x as f64 + 0.5, y as f64 + 0.5);
                let s = if len2 > 0.0 {
                    (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = (p.0 - (a.0 + s * dx)).hypot(p.1 - (a.1 + s * dy));
                let c = (1.0 - d).clamp(0.0, 1.0) as f32;
                let at = &mut cover[y * w + x];
                *at = at.max(c);
            }
        }
    }
    cover
}

/// Composite black at `coverage` over `buf`'s colour planes (straight alpha
/// in a fourth plane, when present).
pub(super) fn darken(buf: &mut PixelBuffer, coverage: &[f32]) {
    let n = buf.pixel_count();
    let has_alpha = buf.channels == 4;
    let data = buf.data.as_mut_slice();
    for (i, &c) in coverage.iter().enumerate() {
        if c <= 0.0 {
            continue;
        }
        let c = c as f64;
        let a = if has_alpha {
            data[3 * n + i] as f64 / 255.0
        } else {
            1.0
        };
        let out = a + c * (1.0 - a);
        // Straight colour after "black over": C·a·(1 − c) / a'.
        let keep = if out > 0.0 { a * (1.0 - c) / out } else { 0.0 };
        for plane in 0..3 {
            let v = &mut data[plane * n + i];
            *v = (*v as f64 * keep).round().clamp(0.0, 255.0) as u8;
        }
        if has_alpha {
            data[3 * n + i] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}
