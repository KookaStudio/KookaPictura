//! Other family: Maximum, Minimum, Offset, High Pass, Custom (`FILT-070`).
//!
//! Maximum/Minimum/Offset are moving kernels and take a fourth (alpha) plane;
//! High Pass and Custom are value kernels and cap at the colour planes.
//! Maximum/Minimum are separable square-footprint morphology, Custom is a 5×5
//! convolution with f64 accumulation, and High Pass splits a Gaussian band
//! around mid-gray. Borders clamp to the edge; the exact kernels are closed,
//! so the choices marked `ponytail:` are documented approximations, not
//! verified parity.

use pictura_core::mask_ops::{self, Border};
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
/// image right/down. `repeat` clamps to the nearest edge sample and takes
/// precedence over `wrap`; otherwise `wrap` cycles, or the exposed area takes
/// `background` on the colour planes and, on alpha, 255 (CS6's "Set to
/// Background") or 0 when `transparent` ("Set to Transparent").
pub fn offset(
    buf: &mut PixelBuffer,
    horizontal: i32,
    vertical: i32,
    wrap: bool,
    repeat: bool,
    background: [u8; 3],
    transparent: bool,
) -> Result<(), FilterError> {
    let n = validate(buf)?;
    if horizontal == 0 && vertical == 0 {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = buf.channels as usize;
    let fill = [
        background[0],
        background[1],
        background[2],
        if transparent { 0 } else { 255 },
    ];
    for (c, &bg) in fill.iter().enumerate().take(planes) {
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        for y in 0..h {
            for x in 0..w {
                let sx = x as i64 - horizontal as i64;
                let sy = y as i64 - vertical as i64;
                let value = if repeat {
                    let rx = clamp_index(sx as isize, w);
                    let ry = clamp_index(sy as isize, h);
                    src[ry * w + rx]
                } else if wrap {
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

/// Max (dilate) or min (erode) over the `(2r+1)²` clamped square, delegated to
/// the shared separable sliding-window kernel.
fn morphology(buf: &mut PixelBuffer, radius: u32, dilate: bool) -> Result<(), FilterError> {
    let n = validate(buf)?;
    if radius == 0 {
        return Ok(());
    }
    let r = radius.min(MAX_RADIUS) as usize;
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = buf.channels as usize;
    for c in 0..planes {
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        let out = if dilate {
            mask_ops::dilate(&src, w, h, r, Border::Clamp)
        } else {
            mask_ops::erode(&src, w, h, r, Border::Clamp)
        };
        buf.data[base..base + n].copy_from_slice(&out);
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
    fn morphology_runs_over_the_alpha_plane_too() {
        let base = buf4(3, 1, &[[10, 0, 0, 111], [20, 0, 0, 222], [30, 0, 0, 255]]);
        let n = base.pixel_count();
        let mut a = base.clone();
        maximum(&mut a, 1).unwrap();
        assert_eq!(&a.data[3 * n..], &[222, 255, 255]);
        let mut b = base.clone();
        minimum(&mut b, 1).unwrap();
        assert_eq!(&b.data[3 * n..], &[111, 111, 222]);
    }

    #[test]
    fn offset_wraps_and_fills_background_with_known_values() {
        let base = gray3(3, 1, &[1, 2, 3]);
        let mut wrapped = base.clone();
        offset(&mut wrapped, 1, 0, true, false, [9, 9, 9], false).unwrap();
        assert_eq!(px3(&wrapped, 0, 0), [3, 3, 3]);
        assert_eq!(px3(&wrapped, 1, 0), [1, 1, 1]);
        assert_eq!(px3(&wrapped, 2, 0), [2, 2, 2]);

        let mut filled = base.clone();
        offset(&mut filled, 1, 0, false, false, [7, 8, 9], false).unwrap();
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
        offset(&mut wrapped, 0, 1, true, false, [0, 0, 0], false).unwrap();
        assert_eq!(px3(&wrapped, 0, 0), [3, 3, 3]);
        assert_eq!(px3(&wrapped, 0, 1), [1, 1, 1]);
        assert_eq!(px3(&wrapped, 0, 2), [2, 2, 2]);

        let mut noop = base.clone();
        offset(&mut noop, 0, 0, false, false, [0, 0, 0], false).unwrap();
        assert_eq!(noop, base);
        let mut repeat_noop = base.clone();
        offset(&mut repeat_noop, 0, 0, false, true, [0, 0, 0], false).unwrap();
        assert_eq!(repeat_noop, base);

        // Wrap moves the alpha plane with the colour; no-wrap fills what it
        // exposes opaque (Set to Background) or clear (Set to Transparent).
        let alpha_base = buf4(2, 1, &[[1, 2, 3, 10], [4, 5, 6, 20]]);
        let n = alpha_base.pixel_count();
        let mut shifted = alpha_base.clone();
        offset(&mut shifted, 1, 0, true, false, [0, 0, 0], false).unwrap();
        assert_eq!(&shifted.data[3 * n..], &[20, 10]);

        let mut filled = alpha_base.clone();
        offset(&mut filled, 1, 0, false, false, [9, 9, 9], false).unwrap();
        assert_eq!(&filled.data[3 * n..], &[255, 10]);

        let mut cleared = alpha_base.clone();
        offset(&mut cleared, 1, 0, false, false, [9, 9, 9], true).unwrap();
        assert_eq!(&cleared.data[3 * n..], &[0, 10]);
        assert_eq!(
            px3(&cleared, 0, 0),
            [9, 9, 9],
            "colour still takes the background"
        );
    }

    #[test]
    fn offset_repeat_edge_pixels_clamps() {
        let base = gray3(4, 1, &[10, 20, 30, 40]);
        let mut repeated = base.clone();
        offset(&mut repeated, 2, 0, false, true, [0, 0, 0], false).unwrap();
        assert_eq!(px3(&repeated, 0, 0), [10, 10, 10]);
        assert_eq!(px3(&repeated, 1, 0), [10, 10, 10]);
        assert_eq!(px3(&repeated, 2, 0), [10, 10, 10]);
        assert_eq!(px3(&repeated, 3, 0), [20, 20, 20]);

        // Repeat wins over wrap.
        let mut both = base.clone();
        offset(&mut both, 2, 0, true, true, [0, 0, 0], false).unwrap();
        assert_eq!(both, repeated);
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
        assert!(offset(&mut a, 5, -3, true, false, [0, 0, 0], false).is_ok());
        let mut b = base.clone();
        assert!(offset(&mut b, 5, -3, false, false, [1, 2, 3], true).is_ok());
        let mut c = base.clone();
        assert!(high_pass(&mut c, 1.0).is_ok());
        let mut d = base.clone();
        assert!(custom(&mut d, &impulse(), 1.0, 0.0).is_ok());
    }
}
