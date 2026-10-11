//! Other family: Maximum, Minimum, Offset, High Pass, Custom (`FILT-070`).
//!
//! Color planes only; alpha is never modified. Maximum/Minimum are separable
//! square-footprint morphology, Custom is a 5×5 convolution with f64
//! accumulation, and High Pass splits a Gaussian band around mid-gray.
//! Borders clamp to the edge; the exact kernels are closed, so the choices
//! marked `ponytail:` are documented approximations, not verified parity.

use pictura_core::PixelBuffer;

use crate::kernel::{clamp_index, gaussian_blur_planes, sigma_from_radius};
use crate::{validate, FilterError};
use rayon::prelude::*;

/// Scriptable radius ceiling for Maximum/Minimum per `FILT-070`; larger
/// requests clamp rather than error.
const MAX_RADIUS: u32 = 100;

pub fn maximum(buf: &mut PixelBuffer, radius: u32) -> Result<(), FilterError> {
    morphology(buf, radius, true)
}

pub fn minimum(buf: &mut PixelBuffer, radius: u32) -> Result<(), FilterError> {
    morphology(buf, radius, false)
}

/// Translate the color planes by `(horizontal, vertical)`; positive moves the
/// image right/down. `wrap` cycles, otherwise the exposed area is `background`.
pub fn offset(
    buf: &mut PixelBuffer,
    horizontal: i32,
    vertical: i32,
    wrap: bool,
    background: [u8; 3],
) -> Result<(), FilterError> {
    let n = validate(buf)?;
    if horizontal == 0 && vertical == 0 {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = (buf.channels as usize).min(3);
    for (c, &bg) in background.iter().enumerate().take(planes) {
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        for y in 0..h {
            for x in 0..w {
                let sx = x as i64 - horizontal as i64;
                let sy = y as i64 - vertical as i64;
                let value = if wrap {
                    let wx = sx.rem_euclid(w as i64) as usize;
                    let wy = sy.rem_euclid(h as i64) as usize;
                    src[wy * w + wx]
                } else if sx >= 0 && (sx as usize) < w && sy >= 0 && (sy as usize) < h {
                    src[sy as usize * w + sx as usize]
                } else {
                    bg
                };
                buf.data[base + y * w + x] = value;
            }
        }
    }
    Ok(())
}

/// `out = clamp(orig − gaussian(orig, sigma) + 128)` per color channel.
pub fn high_pass(buf: &mut PixelBuffer, radius: f64) -> Result<(), FilterError> {
    validate(buf)?;
    if !radius.is_finite() || radius <= 0.0 {
        return Err(FilterError::InvalidParams(format!(
            "high pass radius {radius} must be finite and positive"
        )));
    }
    let n = buf.pixel_count();
    let planes = (buf.channels as usize).min(3);
    let orig = buf.data.clone();
    let mut blurred = buf.clone();
    gaussian_blur_planes(&mut blurred, sigma_from_radius(radius));
    buf.data[..planes * n]
        .par_iter_mut()
        .zip(orig[..planes * n].par_iter())
        .zip(blurred.data[..planes * n].par_iter())
        .for_each(|((out, &o), &b)| {
            *out = (o as f64 - b as f64 + 128.0).round().clamp(0.0, 255.0) as u8;
        });
    Ok(())
}

/// 5×5 convolution: `out = clamp(Σ kernel·neighbor / scale + offset)` per
/// color channel, f64 accumulation, clamp-to-edge.
pub fn custom(
    buf: &mut PixelBuffer,
    kernel: &[[f64; 5]; 5],
    scale: f64,
    offset: f64,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !scale.is_finite() || scale == 0.0 {
        return Err(FilterError::InvalidParams(format!(
            "custom scale {scale} must be finite and non-zero"
        )));
    }
    if !offset.is_finite() {
        return Err(FilterError::InvalidParams(format!(
            "custom offset {offset} must be finite"
        )));
    }
    if kernel.iter().flatten().any(|k| !k.is_finite()) {
        return Err(FilterError::InvalidParams(
            "custom kernel entries must be finite".into(),
        ));
    }
    let n = buf.pixel_count();
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = (buf.channels as usize).min(3);
    for c in 0..planes {
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, out)| {
                for (x, out) in out.iter_mut().enumerate() {
                    let mut acc = 0f64;
                    for (ky, row) in kernel.iter().enumerate() {
                        let sy = clamp_index(y as isize + ky as isize - 2, h);
                        for (kx, &kv) in row.iter().enumerate() {
                            let sx = clamp_index(x as isize + kx as isize - 2, w);
                            acc += kv * src[sy * w + sx] as f64;
                        }
                    }
                    *out = (acc / scale + offset).round().clamp(0.0, 255.0) as u8;
                }
            });
    }
    Ok(())
}

/// Max (dilate) or min (erode) over the `(2r+1)²` clamped square, applied
/// separably (per-row then per-column) because both operators are associative.
///
/// `ponytail:` separable naive scan is O(pixels · r); a monotonic-deque sliding
/// max would make it O(pixels) if radius 100 on huge documents ever gets hot.
fn morphology(buf: &mut PixelBuffer, radius: u32, dilate: bool) -> Result<(), FilterError> {
    let n = validate(buf)?;
    if radius == 0 {
        return Ok(());
    }
    let r = radius.min(MAX_RADIUS) as usize;
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = (buf.channels as usize).min(3);
    let mut horizontal = vec![0u8; n];
    for c in 0..planes {
        let base = c * n;
        let src = &buf.data[base..base + n];
        horizontal
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, out)| {
                let line = &src[y * w..y * w + w];
                for (x, out) in out.iter_mut().enumerate() {
                    let mut best = if dilate { u8::MIN } else { u8::MAX };
                    for dx in 0..=2 * r {
                        let v = line[clamp_index(x as isize + dx as isize - r as isize, w)];
                        best = if dilate { best.max(v) } else { best.min(v) };
                    }
                    *out = best;
                }
            });
        // Max and min are order-free, so the column pass folds whole rows.
        let horizontal = &horizontal;
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, out)| {
                out.fill(if dilate { u8::MIN } else { u8::MAX });
                for dy in 0..=2 * r {
                    let sy = clamp_index(y as isize + dy as isize - r as isize, h);
                    let line = &horizontal[sy * w..sy * w + w];
                    for (best, &v) in out.iter_mut().zip(line) {
                        *best = if dilate {
                            (*best).max(v)
                        } else {
                            (*best).min(v)
                        };
                    }
                }
            });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
        let n = (w * h) as usize;
        assert_eq!(px.len(), n);
        let mut data = vec![0u8; n * 3];
        for (i, p) in px.iter().enumerate() {
            data[i] = p[0];
            data[n + i] = p[1];
            data[2 * n + i] = p[2];
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        }
    }

    fn buf4(w: u32, h: u32, px: &[[u8; 4]]) -> PixelBuffer {
        let n = (w * h) as usize;
        assert_eq!(px.len(), n);
        let mut data = vec![0u8; n * 4];
        for (i, p) in px.iter().enumerate() {
            data[i] = p[0];
            data[n + i] = p[1];
            data[2 * n + i] = p[2];
            data[3 * n + i] = p[3];
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    fn gray3(w: u32, h: u32, values: &[u8]) -> PixelBuffer {
        let px: Vec<[u8; 3]> = values.iter().map(|&v| [v, v, v]).collect();
        buf3(w, h, &px)
    }

    fn px3(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 3] {
        let w = buf.width as usize;
        let n = buf.pixel_count();
        let i = y as usize * w + x as usize;
        [buf.data[i], buf.data[n + i], buf.data[2 * n + i]]
    }

    fn impulse() -> [[f64; 5]; 5] {
        let mut k = [[0.0f64; 5]; 5];
        k[2][2] = 1.0;
        k
    }

    #[test]
    fn maximum_brightens_and_minimum_darkens_known_values() {
        let base = gray3(3, 1, &[10, 20, 30]);
        let mut mx = base.clone();
        maximum(&mut mx, 1).unwrap();
        assert_eq!(px3(&mx, 0, 0), [20, 20, 20]);
        assert_eq!(px3(&mx, 1, 0), [30, 30, 30]);
        assert_eq!(px3(&mx, 2, 0), [30, 30, 30]);

        let mut mn = base.clone();
        minimum(&mut mn, 1).unwrap();
        assert_eq!(px3(&mn, 0, 0), [10, 10, 10]);
        assert_eq!(px3(&mn, 1, 0), [10, 10, 10]);
        assert_eq!(px3(&mn, 2, 0), [20, 20, 20]);

        // Vertical axis exercises the separable column pass.
        let col = gray3(1, 3, &[10, 20, 30]);
        let mut vx = col.clone();
        maximum(&mut vx, 1).unwrap();
        assert_eq!(px3(&vx, 0, 0), [20, 20, 20]);
        assert_eq!(px3(&vx, 0, 2), [30, 30, 30]);
        let mut vn = col.clone();
        minimum(&mut vn, 1).unwrap();
        assert_eq!(px3(&vn, 0, 0), [10, 10, 10]);
        assert_eq!(px3(&vn, 0, 2), [20, 20, 20]);
    }

    #[test]
    fn morphology_radius_zero_is_noop() {
        let base = gray3(3, 1, &[10, 20, 30]);
        let mut a = base.clone();
        maximum(&mut a, 0).unwrap();
        assert_eq!(a, base);
        let mut b = base.clone();
        minimum(&mut b, 0).unwrap();
        assert_eq!(b, base);
    }

    #[test]
    fn morphology_preserves_alpha() {
        let base = buf4(3, 1, &[[10, 0, 0, 111], [20, 0, 0, 222], [30, 0, 0, 255]]);
        let n = base.pixel_count();
        let alpha = base.data[3 * n..].to_vec();
        let mut a = base.clone();
        maximum(&mut a, 1).unwrap();
        assert_eq!(&a.data[3 * n..], &alpha[..]);
        let mut b = base.clone();
        minimum(&mut b, 1).unwrap();
        assert_eq!(&b.data[3 * n..], &alpha[..]);
    }

    #[test]
    fn offset_wraps_and_fills_background_with_known_values() {
        let base = gray3(3, 1, &[1, 2, 3]);
        let mut wrapped = base.clone();
        offset(&mut wrapped, 1, 0, true, [9, 9, 9]).unwrap();
        assert_eq!(px3(&wrapped, 0, 0), [3, 3, 3]);
        assert_eq!(px3(&wrapped, 1, 0), [1, 1, 1]);
        assert_eq!(px3(&wrapped, 2, 0), [2, 2, 2]);

        let mut filled = base.clone();
        offset(&mut filled, 1, 0, false, [7, 8, 9]).unwrap();
        assert_eq!(
            px3(&filled, 0, 0),
            [7, 8, 9],
            "exposed pixel takes background"
        );
        assert_eq!(px3(&filled, 1, 0), [1, 1, 1]);
        assert_eq!(px3(&filled, 2, 0), [2, 2, 2]);
    }

    #[test]
    fn offset_vertical_shift_and_zero_is_noop() {
        let base = gray3(1, 3, &[1, 2, 3]);
        let mut wrapped = base.clone();
        offset(&mut wrapped, 0, 1, true, [0, 0, 0]).unwrap();
        assert_eq!(px3(&wrapped, 0, 0), [3, 3, 3]);
        assert_eq!(px3(&wrapped, 0, 1), [1, 1, 1]);
        assert_eq!(px3(&wrapped, 0, 2), [2, 2, 2]);

        let mut noop = base.clone();
        offset(&mut noop, 0, 0, false, [0, 0, 0]).unwrap();
        assert_eq!(noop, base);

        let alpha_base = buf4(2, 1, &[[1, 2, 3, 10], [4, 5, 6, 20]]);
        let n = alpha_base.pixel_count();
        let alpha = alpha_base.data[3 * n..].to_vec();
        let mut shifted = alpha_base.clone();
        offset(&mut shifted, 1, 0, true, [0, 0, 0]).unwrap();
        assert_eq!(&shifted.data[3 * n..], &alpha[..]);
    }

    #[test]
    fn high_pass_flat_image_is_mid_gray() {
        let base = gray3(5, 4, &[100; 20]);
        let mut out = base.clone();
        high_pass(&mut out, 2.0).unwrap();
        for &v in &out.data {
            assert!(
                (v as i32 - 128).abs() <= 1,
                "flat pixel {v} is not mid-gray"
            );
        }
    }

    #[test]
    fn high_pass_rejects_bad_radius() {
        let base = gray3(4, 1, &[10, 20, 30, 40]);
        for radius in [0.0, -1.0, f64::NAN] {
            let mut out = base.clone();
            assert!(high_pass(&mut out, radius).is_err(), "radius {radius}");
            assert_eq!(out, base, "rejected radius must not change the buffer");
        }
    }

    #[test]
    fn custom_identity_is_bit_exact_and_preserves_alpha() {
        let base = buf4(3, 1, &[[1, 2, 3, 10], [4, 5, 6, 20], [7, 8, 9, 30]]);
        let mut out = base.clone();
        custom(&mut out, &impulse(), 1.0, 0.0).unwrap();
        assert_eq!(out, base);
    }

    #[test]
    fn custom_three_by_three_mean_matches_known_values() {
        let base = gray3(3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let mut kernel = [[0.0f64; 5]; 5];
        for row in kernel.iter_mut().take(4).skip(1) {
            for v in row.iter_mut().take(4).skip(1) {
                *v = 1.0;
            }
        }
        let mut out = base.clone();
        custom(&mut out, &kernel, 9.0, 0.0).unwrap();
        let expected = [2u8, 3, 4, 4, 5, 6, 6, 7, 8];
        for (i, &e) in expected.iter().enumerate() {
            let x = i as u32 % 3;
            let y = i as u32 / 3;
            assert_eq!(px3(&out, x, y), [e, e, e], "pixel {i}");
        }
    }

    #[test]
    fn custom_rejects_zero_scale_and_non_finite_params() {
        let base = gray3(2, 1, &[10, 20]);
        let mut out = base.clone();
        assert!(custom(&mut out, &impulse(), 0.0, 0.0).is_err());
        assert!(custom(&mut out, &impulse(), 1.0, f64::NAN).is_err());
        assert!(custom(&mut out, &impulse(), f64::INFINITY, 0.0).is_err());
        let mut bad = impulse();
        bad[0][0] = f64::INFINITY;
        assert!(custom(&mut out, &bad, 1.0, 0.0).is_err());
        assert_eq!(out, base, "rejected parameters leave the buffer unchanged");
    }

    #[test]
    fn tiny_images_and_oversized_radii_do_not_panic() {
        let base = buf4(1, 1, &[[100, 150, 200, 255]]);
        for radius in [0u32, 1, 1000] {
            let mut a = base.clone();
            assert!(maximum(&mut a, radius).is_ok());
            let mut b = base.clone();
            assert!(minimum(&mut b, radius).is_ok());
        }
        let mut a = base.clone();
        assert!(offset(&mut a, 5, -3, true, [0, 0, 0]).is_ok());
        let mut b = base.clone();
        assert!(offset(&mut b, 5, -3, false, [1, 2, 3]).is_ok());
        let mut c = base.clone();
        assert!(high_pass(&mut c, 1.0).is_ok());
        let mut d = base.clone();
        assert!(custom(&mut d, &impulse(), 1.0, 0.0).is_ok());
    }
}
