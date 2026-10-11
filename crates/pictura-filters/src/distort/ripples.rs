//! Ocean Ripple (`FILT-040`): the picture seen through rippled glass.
//!
//! An inverse-mapping warp with bilinear color sampling; alpha is never
//! touched. The reference's generator is closed (`docs/dev/m11-distort2.md`),
//! so the model below is an approximation.

use pictura_core::PixelBuffer;
use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};
use rayon::prelude::*;

use crate::kernel::{clamp_index, to_u8, unit_f64};
use crate::{validate, FilterError};

/// Seeded refraction through a bumpy surface.
///
/// The surface is smooth value noise on a lattice whose cell grows with
/// `size` (`1..=15`); each pixel samples the source displaced along the
/// surface's slope, by an amount that grows faster than `magnitude`
/// (`0..=20`; `0` is a no-op). Same seed and parameters are bit-identical.
pub fn ocean_ripple(
    buf: &mut PixelBuffer,
    size: u32,
    magnitude: u32,
    seed: u64,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !(1..=15).contains(&size) {
        return Err(FilterError::InvalidParams(format!(
            "ocean ripple size {size} outside 1..=15"
        )));
    }
    if magnitude > 20 {
        return Err(FilterError::InvalidParams(format!(
            "ocean ripple magnitude {magnitude} outside 0..=20"
        )));
    }
    if magnitude == 0 {
        return Ok(());
    }

    // ponytail: fitted by eye to CS6 Filter Gallery renders of one photograph
    // at (size, magnitude) = (9, 9), (2, 12), (14, 2) and (15, 20). Small
    // sizes give CS6's frosted-glass blobs and large magnitudes its scattered
    // fragments; CS6's softer blob interiors at the extremes are not matched.
    let (w, h) = (buf.width as usize, buf.height as usize);
    let n = w * h;
    let cell = 3.0 + 0.5 * size as f64;
    let reach = 0.15 * (magnitude as f64).powf(1.5) * (cell / 4.0).sqrt();
    let (gw, gh) = (
        (w as f64 / cell) as usize + 2,
        (h as f64 / cell) as usize + 2,
    );
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let lattice: Vec<f64> = (0..gw * gh)
        .map(|_| 2.0 * unit_f64(&mut rng) - 1.0)
        .collect();
    let at = |i: usize, j: usize| lattice[j.min(gh - 1) * gw + i.min(gw - 1)];
    // The slope of the smoothstep-interpolated surface, in lattice units.
    let slope = |x: usize, y: usize| {
        let (u, v) = (x as f64 / cell, y as f64 / cell);
        let (i, j) = (u as usize, v as usize);
        let (fu, fv) = (u - i as f64, v - j as f64);
        let (su, sv) = (fu * fu * (3.0 - 2.0 * fu), fv * fv * (3.0 - 2.0 * fv));
        let (du, dv) = (6.0 * fu * (1.0 - fu), 6.0 * fv * (1.0 - fv));
        let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
        (
            ((b - a) * (1.0 - sv) + (d - c) * sv) * du,
            ((c - a) * (1.0 - su) + (d - b) * su) * dv,
        )
    };
    for c in 0..buf.channels as usize {
        let src = buf.data[c * n..c * n + n].to_vec();
        buf.data[c * n..c * n + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, out) in row.iter_mut().enumerate() {
                    let (gx, gy) = slope(x, y);
                    let (sx, sy) = (x as f64 + reach * gx, y as f64 + reach * gy);
                    *out = to_u8(sample(&src, w, h, sx, sy));
                }
            });
    }
    Ok(())
}

/// Bilinear sample at `(x, y)`, clamp-to-edge.
pub(super) fn sample(plane: &[u8], w: usize, h: usize, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (xi, yi) = (clamp_index(x0 as isize, w), clamp_index(y0 as isize, h));
    let (xi1, yi1) = (
        clamp_index(x0 as isize + 1, w),
        clamp_index(y0 as isize + 1, h),
    );
    let p00 = plane[yi * w + xi] as f64;
    let p10 = plane[yi * w + xi1] as f64;
    let p01 = plane[yi1 * w + xi] as f64;
    let p11 = plane[yi1 * w + xi1] as f64;
    (p00 * (1.0 - fx) + p10 * fx) * (1.0 - fy) + (p01 * (1.0 - fx) + p11 * fx) * fy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
        let mut b = PixelBuffer::new(w, h, 3);
        let n = px.len();
        for (i, p) in px.iter().enumerate() {
            for (c, &v) in p.iter().enumerate() {
                b.data[c * n + i] = v;
            }
        }
        b
    }

    /// Deterministic high-frequency pattern so a warp cannot be invariant.
    fn ramp(w: u32, h: u32) -> PixelBuffer {
        let px: Vec<[u8; 3]> = (0..(w * h) as usize)
            .map(|i| [i as u8, (i * 5) as u8, (i * 11) as u8])
            .collect();
        buf3(w, h, &px)
    }

    #[test]
    fn ocean_ripple_is_seed_deterministic() {
        let base = ramp(32, 32);
        let run = |seed| {
            let mut b = base.clone();
            ocean_ripple(&mut b, 9, 20, seed).unwrap();
            b.data
        };
        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8));
        assert_ne!(run(7), base.data);
    }

    /// Mean horizontal displacement, read off a ramp that rises 4 per pixel.
    fn mean_shift(size: u32, magnitude: u32) -> f64 {
        let (w, h) = (60u32, 60u32);
        let px: Vec<[u8; 3]> = (0..(w * h)).map(|i| [(i % w * 4) as u8, 0, 0]).collect();
        let base = buf3(w, h, &px);
        let mut b = base.clone();
        ocean_ripple(&mut b, size, magnitude, 1).unwrap();
        let n = (w * h) as usize;
        // Columns away from the clamped edges.
        let inner = (0..n).filter(|i| (12..48).contains(&(*i as u32 % w)));
        let total: f64 = inner
            .clone()
            .map(|i| (b.data[i] as f64 - base.data[i] as f64).abs() / 4.0)
            .sum();
        total / inner.count() as f64
    }

    #[test]
    fn ocean_ripple_reach_follows_magnitude() {
        // CS6's defaults visibly break edges up; Magnitude 2 barely wiggles.
        let (faint, default, strong) = (mean_shift(14, 2), mean_shift(9, 9), mean_shift(2, 12));
        assert!(faint < 1.0, "magnitude 2 shifts {faint:.2} px");
        assert!(default > 2.0, "defaults shift {default:.2} px");
        assert!(
            strong > default,
            "magnitude 12 {strong:.2} vs 9 {default:.2}"
        );
    }

    #[test]
    fn ocean_ripple_zero_magnitude_is_noop_and_validates() {
        let base = ramp(8, 8);
        let mut b = base.clone();
        ocean_ripple(&mut b, 9, 0, 1).unwrap();
        assert_eq!(b.data, base.data);

        assert!(ocean_ripple(&mut b, 0, 5, 1).is_err());
        assert!(ocean_ripple(&mut b, 16, 5, 1).is_err());
        assert!(ocean_ripple(&mut b, 9, 21, 1).is_err());
    }
}
