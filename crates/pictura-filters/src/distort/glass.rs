//! Glass (`FILT-040`): the picture seen through a textured pane.
//!
//! Each built-in surface is a height map; every pixel samples the source
//! displaced along the map's slope, so each bump bends the picture like a
//! small lens. Smoothness blurs the map, Scaling sizes it, Invert turns its
//! bumps into dents. CS6 ships its surfaces as bitmaps and its algorithm is
//! closed; these procedural maps and the model are tuned by eye against CS6
//! Filter Gallery renders of one photograph, an approximation, not parity.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use super::ripples::sample;
use crate::kernel::{blur_plane, to_u8};
use crate::{validate, FilterError, GlassTexture};

// Pixels of displacement per Distortion step, at one RMS unit of slope.
const REACH_PER_STEP: f32 = 1.4;
// The slope is measured on this square of the surface, so the strength does
// not depend on the picture's size.
const REFERENCE: usize = 256;
// Smoothness blurs the map by this σ per step, in texture pixels.
const SMOOTH_PER_STEP: f32 = 0.4;
// Feature sizes at 100 %, in pixels.
const FROSTED_CELLS: (f32, f32) = (6.0, 13.0);
const LENS_PITCH: f32 = 16.0;
const BLOCK: f32 = 40.0;
const WEAVE_PITCH: f32 = 24.0;

/// Refract `buf` through `texture`.
///
/// `distortion` in `0..=20` (0 is a no-op), `smoothness` in `1..=15`,
/// `scaling` in `50..=200` percent. Deterministic; alpha is untouched.
pub fn glass(
    buf: &mut PixelBuffer,
    distortion: u32,
    smoothness: u32,
    texture: GlassTexture,
    scaling: u32,
    invert: bool,
) -> Result<(), FilterError> {
    validate(buf)?;
    for (name, value, range) in [
        ("distortion", distortion, 0..=20),
        ("smoothness", smoothness, 1..=15),
        ("scaling", scaling, 50..=200),
    ] {
        if !range.contains(&value) {
            return Err(FilterError::InvalidParams(format!(
                "glass {name} {value} outside {range:?}"
            )));
        }
    }
    if distortion == 0 {
        return Ok(());
    }
    let (w, h) = (buf.width as usize, buf.height as usize);
    let n = w * h;
    let zoom = scaling as f32 / 100.0;
    let sigma = SMOOTH_PER_STEP * smoothness as f32 * zoom;

    let height = surface(texture, w, h, zoom, sigma);
    let reference = surface(texture, REFERENCE, REFERENCE, zoom, sigma);
    let (mut sum, mut count) = (0f64, 0usize);
    for y in 0..REFERENCE {
        for x in 0..REFERENCE {
            let (gx, gy) = slope(&reference, REFERENCE, REFERENCE, x, y);
            sum += f64::from(gx * gx + gy * gy);
            count += 1;
        }
    }
    let rms = (sum / count as f64).sqrt() as f32;
    if rms <= f32::EPSILON {
        return Ok(());
    }
    let sign = if invert { -1.0 } else { 1.0 };
    let reach = sign * REACH_PER_STEP * distortion as f32 * relief(texture) / rms;

    for c in 0..buf.channels as usize {
        let src = buf.data[c * n..c * n + n].to_vec();
        buf.data[c * n..c * n + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, out) in row.iter_mut().enumerate() {
                    let (gx, gy) = slope(&height, w, h, x, y);
                    let sx = x as f64 + f64::from(reach * gx);
                    let sy = y as f64 + f64::from(reach * gy);
                    *out = to_u8(sample(&src, w, h, sx, sy));
                }
            });
    }
    Ok(())
}

/// How deep each surface's relief runs; CS6's Frosted and Canvas bend the
/// picture less than its Blocks and Tiny Lens at the same Distortion.
fn relief(texture: GlassTexture) -> f32 {
    match texture {
        GlassTexture::Frosted => 0.6,
        GlassTexture::Canvas => 0.45,
        GlassTexture::Blocks | GlassTexture::TinyLens => 1.0,
    }
}

/// The surface's height map over a `w`×`h` picture, blurred by `sigma`.
fn surface(texture: GlassTexture, w: usize, h: usize, zoom: f32, sigma: f32) -> Vec<f32> {
    let mut field = vec![0f32; w * h];
    field.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let v = y as f32 / zoom;
        for (x, out) in row.iter_mut().enumerate() {
            *out = height(texture, x as f32 / zoom, v);
        }
    });
    if texture == GlassTexture::Blocks {
        // Glass blocks are cast, not cut: their faces swell into each other.
        field = blur_plane(&field, w, h, 0.25 * BLOCK * zoom);
    }
    blur_plane(&field, w, h, sigma)
}

/// One surface's height at texture coordinates `(u, v)`, roughly 0..1.
fn height(texture: GlassTexture, u: f32, v: f32) -> f32 {
    match texture {
        GlassTexture::Frosted => {
            0.6 * value_noise(u, v, FROSTED_CELLS.0, 1)
                + 0.4 * value_noise(u, v, FROSTED_CELLS.1, 2)
        }
        GlassTexture::TinyLens => {
            // Domes on a lattice turned 45°.
            let a = (u + v) * std::f32::consts::FRAC_1_SQRT_2;
            let b = (u - v) * std::f32::consts::FRAC_1_SQRT_2;
            let cx = a.rem_euclid(LENS_PITCH) / LENS_PITCH - 0.5;
            let cy = b.rem_euclid(LENS_PITCH) / LENS_PITCH - 0.5;
            (1.0 - 4.0 * (cx * cx + cy * cy)).max(0.0).sqrt()
        }
        GlassTexture::Blocks => {
            // Courses of blocks, every other one offset by half.
            let row = (v / BLOCK).floor();
            let col = (u / BLOCK + 0.5 * row.rem_euclid(2.0)).floor();
            unit(col as i32, row as i32, 3)
        }
        GlassTexture::Canvas => {
            use std::f32::consts::PI;
            let (a, b) = (
                (u / WEAVE_PITCH * PI).sin().powi(2),
                (v / WEAVE_PITCH * PI).sin().powi(2),
            );
            let over = ((u / WEAVE_PITCH).floor() + (v / WEAVE_PITCH).floor()).rem_euclid(2.0);
            let weave = if over > 0.5 {
                0.6 * a + 0.3 * b
            } else {
                0.3 * a + 0.6 * b
            };
            0.5 * weave + 0.6 * value_noise(u, v, 0.7 * WEAVE_PITCH, 7)
        }
    }
}

/// Central-difference slope of `field` at `(x, y)`, one-sided at the edges.
fn slope(field: &[f32], w: usize, h: usize, x: usize, y: usize) -> (f32, f32) {
    let at = |x: usize, y: usize| field[y * w + x];
    let (x0, x1) = (x.saturating_sub(1), (x + 1).min(w - 1));
    let (y0, y1) = (y.saturating_sub(1), (y + 1).min(h - 1));
    let gx = (at(x1, y) - at(x0, y)) / (x1 - x0).max(1) as f32;
    let gy = (at(x, y1) - at(x, y0)) / (y1 - y0).max(1) as f32;
    (gx, gy)
}

/// Smooth 0..1 noise on a lattice `cell` pixels apart.
fn value_noise(u: f32, v: f32, cell: f32, salt: u32) -> f32 {
    let (gu, gv) = (u / cell, v / cell);
    let (iu, iv) = (gu.floor(), gv.floor());
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fu, fv) = (smooth(gu - iu), smooth(gv - iv));
    let at = |du: i32, dv: i32| unit(iu as i32 + du, iv as i32 + dv, salt);
    let top = at(0, 0) * (1.0 - fu) + at(1, 0) * fu;
    let bottom = at(0, 1) * (1.0 - fu) + at(1, 1) * fu;
    top * (1.0 - fv) + bottom * fv
}

/// A uniform `[0, 1)` value for one lattice point.
fn unit(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXTURES: [GlassTexture; 4] = [
        GlassTexture::Blocks,
        GlassTexture::Canvas,
        GlassTexture::Frosted,
        GlassTexture::TinyLens,
    ];

    /// A horizontal ramp rising 4 per pixel in red, so a shifted sample
    /// reads back as its horizontal displacement; alpha 90.
    fn ramp(w: u32, h: u32) -> PixelBuffer {
        let n = (w * h) as usize;
        let mut data = vec![0u8; n * 4];
        for i in 0..n {
            data[i] = (i as u32 % w * 4).min(255) as u8;
            data[3 * n + i] = 90;
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    /// Mean horizontal shift in pixels, away from the clamped edges.
    fn mean_shift(texture: GlassTexture, distortion: u32, smoothness: u32) -> f64 {
        let (w, h) = (60u32, 60u32);
        let base = ramp(w, h);
        let mut b = base.clone();
        glass(&mut b, distortion, smoothness, texture, 100, false).unwrap();
        let inner: Vec<usize> = (0..(w * h) as usize)
            .filter(|i| (14..46).contains(&(*i as u32 % w)))
            .collect();
        let total: f64 = inner
            .iter()
            .map(|&i| (f64::from(b.data[i]) - f64::from(base.data[i])).abs() / 4.0)
            .sum();
        total / inner.len() as f64
    }

    #[test]
    fn zero_distortion_is_a_no_op() {
        for texture in TEXTURES {
            let mut b = ramp(24, 24);
            let before = b.data.clone();
            glass(&mut b, 0, 3, texture, 100, false).unwrap();
            assert_eq!(b.data, before);
        }
    }

    #[test]
    fn distortion_bends_every_surface_and_grows() {
        for texture in TEXTURES {
            let (low, high) = (mean_shift(texture, 3, 3), mean_shift(texture, 15, 3));
            assert!(low > 0.3, "{texture:?} at 3 shifts {low:.2} px");
            assert!(
                high > 2.0 * low,
                "{texture:?}: 15 → {high:.2}, 3 → {low:.2}"
            );
        }
    }

    #[test]
    fn invert_flips_the_bend_and_alpha_stays() {
        let run = |invert| {
            let mut b = ramp(40, 40);
            glass(&mut b, 10, 3, GlassTexture::Frosted, 100, invert).unwrap();
            b
        };
        let (plain, inverted) = (run(false), run(true));
        assert_ne!(plain.data, inverted.data);
        assert_eq!(plain.data, run(false).data, "deterministic");
        let n = 40 * 40;
        assert!(
            plain.data[3 * n..].iter().all(|&a| a == 90),
            "alpha untouched"
        );
    }

    #[test]
    fn out_of_range_parameters_are_rejected_untouched() {
        for (d, s, sc) in [
            (21, 3, 100),
            (5, 0, 100),
            (5, 16, 100),
            (5, 3, 49),
            (5, 3, 201),
        ] {
            let mut b = ramp(8, 8);
            let before = b.data.clone();
            assert!(matches!(
                glass(&mut b, d, s, GlassTexture::Frosted, sc, false),
                Err(FilterError::InvalidParams(_))
            ));
            assert_eq!(b.data, before);
        }
    }
}
