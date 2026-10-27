//! Noise family: Add Noise, Median, Despeckle, Reduce Noise (`FILT-030`).
//!
//! Add Noise is the only randomized filter in the crate; the seed is part of
//! the parameters so a re-apply is bit-identical. Median, Despeckle and Reduce
//! Noise are deterministic. Median and Despeckle are moving kernels — they run
//! over every plane, alpha included; Add Noise and Reduce Noise leave alpha
//! bit-identical. The reference's kernels are closed, so the choices marked
//! `ponytail:` below are documented approximations, not verified parity.

use pictura_core::mask_ops;
use pictura_core::PixelBuffer;

use crate::{kernel::clamp_index, validate, FilterError};

mod reduce;
pub use reduce::{reduce_noise, ChannelDetail, ReduceNoiseParams};

/// Despeckle: how far outside a flat neighborhood the center must sit before
/// it reads as an isolated outlier rather than texture.
const DESPECKLE_SPIKE: i32 = 12;
/// Despeckle: neighborhood spread at or below this counts as "flat" (no edge).
const DESPECKLE_BAND: i32 = 12;

/// Replace each pixel with the per-channel median of its `(2r+1)²` window.
///
/// `radius == 0` is a no-op. Clamp-to-edge at borders.
pub fn median(buf: &mut PixelBuffer, radius: u32) -> Result<(), FilterError> {
    let n = validate(buf)?;
    if radius == 0 {
        return Ok(());
    }
    if radius > 100 {
        return Err(FilterError::InvalidParams(format!(
            "median radius {radius} is outside 0..=100"
        )));
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    // Cap the radius so an oversized request cannot allocate a giant window;
    // beyond max(w, h) every clamped sample is interior anyway.
    let r = (radius as usize).min(w.max(h));
    let planes = buf.channels as usize;
    for plane in 0..planes {
        let base = plane * n;
        let med = mask_ops::median(&buf.data[base..base + n], w, h, r);
        buf.data[base..base + n].copy_from_slice(&med);
    }
    Ok(())
}

/// Edge-gated smoothing: replace isolated outliers, leave edges alone.
///
/// A pixel is an outlier when its 8 neighbors are flat (spread <= band) and
/// the center lies beyond them, in which case it takes the 3×3 median. A
/// genuine edge makes the neighborhood spread wide, so its pixels are kept.
pub fn despeckle(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    let n = validate(buf)?;
    let w = buf.width as usize;
    let h = buf.height as usize;
    if w < 3 || h < 3 {
        return Ok(());
    }
    let planes = buf.channels as usize;
    // ponytail: fixed internal thresholds; FILT-030 leaves the detector closed.
    // A per-tile adaptive threshold or a real gradient gate is the upgrade path.
    for plane in 0..planes {
        let base = plane * n;
        let src = buf.data[base..base + n].to_vec();
        for y in 0..h {
            for x in 0..w {
                let mut vals = [0i32; 9];
                let mut nmin = 255i32;
                let mut nmax = 0i32;
                for dy in 0..3usize {
                    let sy = clamp_index(y as isize + dy as isize - 1, h);
                    for dx in 0..3usize {
                        let sx = clamp_index(x as isize + dx as isize - 1, w);
                        let v = src[sy * w + sx] as i32;
                        vals[dy * 3 + dx] = v;
                        if dy != 1 || dx != 1 {
                            nmin = nmin.min(v);
                            nmax = nmax.max(v);
                        }
                    }
                }
                let center = vals[4];
                if nmax - nmin <= DESPECKLE_BAND
                    && (center > nmax + DESPECKLE_SPIKE || center < nmin - DESPECKLE_SPIKE)
                {
                    vals.sort_unstable();
                    buf.data[base + y * w + x] = vals[4] as u8;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(w: u32, h: u32, channels: u8, fill: [u8; 3]) -> PixelBuffer {
        let n = w as usize * h as usize;
        let mut data = vec![0u8; n * channels as usize];
        for p in 0..n {
            for (c, &v) in fill.iter().enumerate() {
                data[c * n + p] = v;
            }
            if channels == 4 {
                data[3 * n + p] = 200;
            }
        }
        PixelBuffer {
            width: w,
            height: h,
            channels,
            data: data.into(),
        }
    }

    #[test]
    fn median_removes_salt_and_pepper_and_keeps_step_edge() {
        let mut salt = plane(8, 8, 3, [100, 100, 100]);
        let w = 8usize;
        let n = salt.pixel_count();
        salt.data[3 * w + 4] = 255;
        median(&mut salt, 1).unwrap();
        assert!(
            salt.data.iter().all(|&v| v == 100),
            "isolated pixel should be removed"
        );

        let mut step = plane(8, 8, 3, [50, 50, 50]);
        for p in 0..n {
            if p % w >= 4 {
                step.data[p] = 200;
                step.data[n + p] = 200;
                step.data[2 * n + p] = 200;
            }
        }
        median(&mut step, 1).unwrap();
        for y in 0..8usize {
            assert_eq!(step.data[y * w], 50);
            assert_eq!(step.data[y * w + w - 1], 200);
        }
        assert!(
            step.data.iter().all(|&v| v == 50 || v == 200),
            "step edge should stay sharp"
        );
    }

    #[test]
    fn despeckle_removes_isolated_pixel_and_keeps_strong_edge() {
        let mut b = plane(8, 8, 3, [100, 100, 100]);
        let w = 8usize;
        let n = b.pixel_count();
        b.data[3 * w + 4] = 255;
        despeckle(&mut b).unwrap();
        assert_eq!(b.data[3 * w + 4], 100);

        let mut e = plane(8, 8, 3, [0, 0, 0]);
        for p in 0..n {
            if p % w >= 4 {
                e.data[p] = 255;
                e.data[n + p] = 255;
                e.data[2 * n + p] = 255;
            }
        }
        let before = e.data.clone();
        despeckle(&mut e).unwrap();
        assert_eq!(e.data, before, "strong step edge must be preserved");
    }

    #[test]
    fn median_radius_changes_a_structured_image() {
        let mut base = plane(9, 9, 3, [0, 0, 0]);
        let w = 9usize;
        let n = base.pixel_count();
        for y in 0..9 {
            for x in 0..9 {
                if x % 3 == 0 || y % 3 == 0 {
                    base.data[y * w + x] = 200;
                    base.data[n + y * w + x] = 200;
                    base.data[2 * n + y * w + x] = 200;
                }
            }
        }
        let mut r1 = base.clone();
        median(&mut r1, 1).unwrap();
        let mut r3 = base.clone();
        median(&mut r3, 3).unwrap();
        assert_ne!(r1.data, r3.data, "radius 1 and radius 3 smooth differently");
    }
}
