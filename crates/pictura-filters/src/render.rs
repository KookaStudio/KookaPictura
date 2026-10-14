//! Render family: Clouds, Difference Clouds, Fibers, Lens Flare, Lighting
//! Effects (`FILT-060`).
//!
//! All are classified **no-equivalent**: the reference's noise and flare models
//! are closed, so these are deterministic approximations verified by property
//! tests (range, determinism, monotonicity), not delta fitting against
//! reference output. Clouds replace the color planes from a seeded value-noise
//! field and Fibers from photorust's streak model ([`fibers`]); Lens Flare adds
//! photorust's flare model on top of the existing pixels ([`lens_flare`]); Lighting
//! Effects re-lights the picture under CS6's light rig ([`lighting_effects`]).
//! Alpha is never modified.

use pictura_core::PixelBuffer;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha8Rng,
};

use crate::{kernel::unit_f64, validate, FilterError};

mod fibers;
mod lens_flare;
mod lighting;
mod picture_frame;
pub use fibers::fibers;
pub use lens_flare::{lens_flare, LensType};
pub use lighting::{
    lighting_effects, spot_hotspot, Light, LightType, Lighting, TextureChannel, MAX_LIGHTS,
};
pub use picture_frame::{
    picture_frame, uses_lines, FrameOptions, FLOWER_NAMES, FRAME_NAMES, LEAF_NAMES,
};

/// Clouds: base lattice cells across the image.
const CLOUD_CELLS: f64 = 8.0;
/// fBm octaves, lacunarity and gain shared by the noise-driven filters.
const OCTAVES: usize = 5;
const LACUNARITY: f64 = 2.0;
const GAIN: f64 = 0.5;

fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Seeded lattice value noise: a pseudo-random value per integer node from a
/// ChaCha-seeded table, bilinear interpolation over smoothstep-eased
/// fractional parts, and 5-octave fBm (lacunarity 2, gain 0.5) normalized to
/// [0, 1]. Deterministic for a given seed and independent of buffer size.
///
/// ponytail: the reference's noise is closed and lattice value noise shows grid
/// artifacts; switch to gradient/Perlin noise if the look ever matters.
struct ValueNoise {
    perm: [u8; 512],
    vals: [f64; 256],
}

impl ValueNoise {
    fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut half = [0u8; 256];
        rng.fill_bytes(&mut half);
        let mut noise = ValueNoise {
            perm: [0u8; 512],
            vals: [0.0; 256],
        };
        noise.perm[..256].copy_from_slice(&half);
        noise.perm[256..].copy_from_slice(&half);
        for v in &mut noise.vals {
            *v = unit_f64(&mut rng);
        }
        noise
    }

    fn node(&self, ix: i64, iy: i64, octave: usize) -> f64 {
        let x = (ix as usize) & 255;
        let y = ((iy as usize).wrapping_add(octave * 97)) & 255;
        let h = self.perm[self.perm[x] as usize + self.perm[y] as usize];
        self.vals[h as usize]
    }

    fn octave(&self, x: f64, y: f64, octave: usize) -> f64 {
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let tx = smoothstep(x - ix as f64);
        let ty = smoothstep(y - iy as f64);
        let v00 = self.node(ix, iy, octave);
        let v10 = self.node(ix + 1, iy, octave);
        let v01 = self.node(ix, iy + 1, octave);
        let v11 = self.node(ix + 1, iy + 1, octave);
        v00 + (v10 - v00) * tx + (v01 - v00) * ty + (v00 - v10 - v01 + v11) * tx * ty
    }

    fn fbm(&self, x: f64, y: f64) -> f64 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut norm = 0.0;
        let mut fx = x;
        let mut fy = y;
        for o in 0..OCTAVES {
            sum += amp * self.octave(fx, fy, o);
            norm += amp;
            amp *= GAIN;
            fx *= LACUNARITY;
            fy *= LACUNARITY;
        }
        sum / norm
    }
}

/// The normalized Clouds field (starker contrast already applied), row-major.
fn cloud_field(w: usize, h: usize, starker: bool, seed: u64) -> Vec<f64> {
    let noise = ValueNoise::new(seed);
    let mut field = Vec::with_capacity(w * h);
    for y in 0..h {
        let v = (y as f64 + 0.5) / h as f64;
        for x in 0..w {
            let u = (x as f64 + 0.5) / w as f64;
            let mut f = noise.fbm(CLOUD_CELLS * u, CLOUD_CELLS * v);
            if starker {
                f = smoothstep(f);
            }
            field.push(f);
        }
    }
    field
}

fn lerp_channel(a: u8, b: u8, f: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * f).round() as u8
}

/// Clouds: replace the color planes with the color ramp mapped through the
/// seeded fBm field (`starker` applies a smoothstep contrast curve first).
pub fn clouds(
    buf: &mut PixelBuffer,
    color_a: [u8; 3],
    color_b: [u8; 3],
    starker: bool,
    seed: u64,
) -> Result<(), FilterError> {
    let n = validate(buf)?;
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = (buf.channels as usize).min(3);
    let field = cloud_field(w, h, starker, seed);
    for (p, &f) in field.iter().enumerate() {
        for c in 0..planes {
            buf.data[c * n + p] = lerp_channel(color_a[c], color_b[c], f);
        }
    }
    Ok(())
}

/// Difference Clouds: same field as [`clouds`], then set each color sample to
/// the absolute difference between the existing pixel and the mapped cloud.
pub fn difference_clouds(
    buf: &mut PixelBuffer,
    color_a: [u8; 3],
    color_b: [u8; 3],
    starker: bool,
    seed: u64,
) -> Result<(), FilterError> {
    let n = validate(buf)?;
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = (buf.channels as usize).min(3);
    let field = cloud_field(w, h, starker, seed);
    for (p, &f) in field.iter().enumerate() {
        for c in 0..planes {
            let cloud = lerp_channel(color_a[c], color_b[c], f) as i32;
            let existing = buf.data[c * n + p] as i32;
            buf.data[c * n + p] = existing.abs_diff(cloud) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(w: u32, h: u32, fill: [u8; 3]) -> PixelBuffer {
        let n = w as usize * h as usize;
        let mut data = vec![0u8; n * 4];
        for p in 0..n {
            for (c, &v) in fill.iter().enumerate() {
                data[c * n + p] = v;
            }
            data[3 * n + p] = 200;
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    /// Deterministic non-uniform pattern so an additive/replacing filter
    /// cannot be invariant.
    fn pattern(w: u32, h: u32) -> PixelBuffer {
        let n = w as usize * h as usize;
        let mut data = vec![0u8; n * 4];
        for y in 0..h as usize {
            for x in 0..w as usize {
                let p = y * w as usize + x;
                data[p] = (x * 37 + y * 17) as u8;
                data[n + p] = (x * 11 + y * 53) as u8;
                data[2 * n + p] = (x * 71 + y * 5) as u8;
                data[3 * n + p] = 200;
            }
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    fn plane_std(buf: &PixelBuffer, plane: usize) -> f64 {
        let n = buf.pixel_count();
        let vals = &buf.data[plane * n..plane * n + n];
        let mean = vals.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
        let var = vals.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt()
    }

    fn assert_channels_within(buf: &PixelBuffer, a: [u8; 3], b: [u8; 3]) {
        let n = buf.pixel_count();
        for p in 0..n {
            for c in 0..3 {
                let v = buf.data[c * n + p];
                assert!(
                    (a[c].min(b[c])..=a[c].max(b[c])).contains(&v),
                    "channel {c} pixel {p} value {v} outside [{}, {}]",
                    a[c].min(b[c]),
                    a[c].max(b[c])
                );
            }
        }
    }

    #[test]
    fn clouds_same_seed_is_bit_identical_and_different_seed_differs() {
        let base = plane(32, 32, [0, 0, 0]);
        let mut a = base.clone();
        let mut b = base.clone();
        let mut c = base.clone();
        clouds(&mut a, [10, 20, 30], [200, 210, 220], false, 42).unwrap();
        clouds(&mut b, [10, 20, 30], [200, 210, 220], false, 42).unwrap();
        clouds(&mut c, [10, 20, 30], [200, 210, 220], false, 43).unwrap();
        assert_eq!(a.data, b.data);
        assert_ne!(a.data, c.data);
    }

    #[test]
    fn clouds_rgb_stays_within_color_endpoints_and_alpha_untouched() {
        let a = [10, 200, 30];
        let b = [240, 20, 90];
        let mut buf = plane(24, 24, [7, 7, 7]);
        clouds(&mut buf, a, b, false, 5).unwrap();
        assert_channels_within(&buf, a, b);
        let n = buf.pixel_count();
        assert!(buf.data[3 * n..].iter().all(|&v| v == 200));
    }

    #[test]
    fn starker_clouds_differ_and_raise_contrast() {
        let base = plane(32, 32, [0, 0, 0]);
        let mut plain = base.clone();
        let mut stark = base.clone();
        clouds(&mut plain, [0, 0, 0], [255, 255, 255], false, 9).unwrap();
        clouds(&mut stark, [0, 0, 0], [255, 255, 255], true, 9).unwrap();
        assert_ne!(plain.data, stark.data);
        for c in 0..3 {
            assert!(
                plane_std(&stark, c) > plane_std(&plain, c),
                "channel {c}: starker std should exceed plain"
            );
        }
    }

    #[test]
    fn difference_clouds_equals_abs_difference_with_clouds() {
        let mut orig = pattern(16, 16);
        let mut cloud_ref = orig.clone();
        clouds(&mut cloud_ref, [30, 40, 50], [220, 180, 90], true, 11).unwrap();
        let before = orig.data.clone();
        difference_clouds(&mut orig, [30, 40, 50], [220, 180, 90], true, 11).unwrap();
        let n = orig.pixel_count();
        for p in 0..n {
            for c in 0..3 {
                let expected = before[c * n + p].abs_diff(cloud_ref.data[c * n + p]);
                assert_eq!(orig.data[c * n + p], expected, "plane {c} pixel {p}");
            }
            assert_eq!(orig.data[3 * n + p], 200);
        }
    }

    #[test]
    fn second_difference_clouds_application_differs() {
        let mut buf = pattern(16, 16);
        difference_clouds(&mut buf, [0, 0, 0], [255, 255, 255], false, 3).unwrap();
        let first = buf.data.clone();
        difference_clouds(&mut buf, [0, 0, 0], [255, 255, 255], false, 3).unwrap();
        assert_ne!(buf.data, first);
    }
}
