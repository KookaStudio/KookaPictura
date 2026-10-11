//! Lens Blur: depth-map-driven aperture convolution (`FILT-010`).
//!
//! Adobe's kernel is closed; this is the documented model — a per-pixel
//! circle of confusion read from a depth map, an iris-shaped aperture, an
//! additive specular highlight, and re-injected grain. Only the color planes
//! are written; the render layer's alpha pass blurs the alpha channel.

use std::f64::consts::PI;

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::kernel::clamp_index;
use crate::{validate, FilterError, NoiseDistribution};

/// The UI's largest radius (`FILT-010`).
pub const MAX_LENS_RADIUS: u32 = 100;

/// The aperture polygon: CS6's blade-count presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrisShape {
    Triangle,
    Square,
    Pentagon,
    Hexagon,
    Heptagon,
    Octagon,
}

impl IrisShape {
    fn blades(self) -> u32 {
        match self {
            Self::Triangle => 3,
            Self::Square => 4,
            Self::Pentagon => 5,
            Self::Hexagon => 6,
            Self::Heptagon => 7,
            Self::Octagon => 8,
        }
    }
}

/// A Lens Blur depth map: one 8-bit plane, 0 near/front and 255 far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthMap {
    width: u32,
    height: u32,
    samples: Vec<u8>,
}

impl DepthMap {
    /// `None` unless `samples` holds `width * height` samples and the map is
    /// not empty.
    pub fn new(width: u32, height: u32, samples: Vec<u8>) -> Option<Self> {
        let n = width as usize * height as usize;
        (n > 0 && samples.len() == n).then_some(Self {
            width,
            height,
            samples,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn at(&self, x: u32, y: u32) -> u8 {
        self.samples[y as usize * self.width as usize + x as usize]
    }

    /// The sample at normalized `(u, v)`, bilinear and clamped, so a small map
    /// stretched over a larger buffer bends smoothly instead of in steps.
    fn stretched(&self, u: f32, v: f32) -> f32 {
        let (max_x, max_y) = (self.width - 1, self.height - 1);
        let u = u.clamp(0.0, max_x as f32);
        let v = v.clamp(0.0, max_y as f32);
        let (x0, y0) = (u as u32, v as u32);
        let (x1, y1) = ((x0 + 1).min(max_x), (y0 + 1).min(max_y));
        let (fx, fy) = (u - x0 as f32, v - y0 as f32);
        let [a, b, c, d] = [
            self.at(x0, y0),
            self.at(x1, y0),
            self.at(x0, y1),
            self.at(x1, y1),
        ]
        .map(f32::from);
        let top = a + (b - a) * fx;
        let bottom = c + (d - c) * fx;
        top + (bottom - top) * fy
    }
}

/// Lens Blur's control values.
#[derive(Debug, Clone, Copy)]
pub struct LensParams {
    pub focal: u8,
    pub invert: bool,
    pub shape: IrisShape,
    pub radius: u32,
    pub curvature: u8,
    pub rotation: f64,
    pub specular_brightness: u8,
    pub specular_threshold: u8,
    pub noise_amount: u8,
    pub noise_distribution: NoiseDistribution,
    pub monochromatic: bool,
    pub faster: bool,
    pub seed: u64,
}

/// Depth-driven aperture blur. `None` acts as a uniform far (255) depth.
pub fn lens_blur(
    buf: &mut PixelBuffer,
    depth: Option<&DepthMap>,
    lens: &LensParams,
) -> Result<(), FilterError> {
    validate(buf)?;
    if lens.radius > MAX_LENS_RADIUS {
        return Err(FilterError::InvalidParams(format!(
            "lens blur radius {} is outside 0..=100",
            lens.radius
        )));
    }
    if lens.curvature > 100 || lens.specular_brightness > 100 || lens.noise_amount > 100 {
        return Err(FilterError::InvalidParams(
            "lens blur curvature, brightness, and noise amount span 0..=100".into(),
        ));
    }
    if !lens.rotation.is_finite() {
        return Err(FilterError::InvalidParams(format!(
            "lens blur rotation {} must be finite",
            lens.rotation
        )));
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let radii = radii_map(w, h, depth, lens);
    let max_r = radii.iter().copied().max().unwrap_or(0) as usize;
    if max_r == 0 && lens.noise_amount == 0 {
        return Ok(());
    }
    let tables = TapTables::build(lens, max_r);
    let src = buf.data.clone();
    let luma: Vec<u8> = (0..n)
        .map(|i| {
            let l =
                0.299 * src[i] as f32 + 0.587 * src[n + i] as f32 + 0.114 * src[2 * n + i] as f32;
            l.round().clamp(0.0, 255.0) as u8
        })
        .collect();
    let brightness = lens.specular_brightness as f32 / 100.0;
    let threshold = lens.specular_threshold;
    let noise = lens.noise_amount as f32 / 100.0 * 127.5;
    let alpha_base = (buf.channels == 4).then_some(3 * n);
    for c in 0..planes {
        let base = c * n;
        let channel = if lens.monochromatic { 0 } else { c as u32 };
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, out) in row.iter_mut().enumerate() {
                    let i = y * w + x;
                    let r = radii[i] as usize;
                    let mut value = if r == 0 {
                        src[base + i] as f32
                    } else {
                        let mut sum = 0f32;
                        let mut wsum = 0f32;
                        let mut highlight = 0f32;
                        for &(dx, dy, weight) in tables.taps(r) {
                            let sx = clamp_index(x as isize + dx as isize, w);
                            let sy = clamp_index(y as isize + dy as isize, h);
                            let si = sy * w + sx;
                            let v = src[base + si] as f32;
                            sum += v * weight;
                            wsum += weight;
                            if brightness > 0.0 && luma[si] >= threshold {
                                highlight += v * weight;
                            }
                        }
                        let base_value = sum / wsum;
                        if brightness > 0.0 {
                            base_value + highlight / wsum * brightness
                        } else {
                            base_value
                        }
                    };
                    if noise > 0.0 {
                        let visible = alpha_base.is_none_or(|a| src[a + i] != 0);
                        if visible {
                            value += noise * jitter(lens, x as u32, y as u32, channel);
                        }
                    }
                    *out = value.round().clamp(0.0, 255.0) as u8;
                }
            });
    }
    Ok(())
}

/// The circle-of-confusion radius at every pixel: `radius × |d − focal| / 255`.
fn radii_map(w: usize, h: usize, depth: Option<&DepthMap>, lens: &LensParams) -> Vec<u8> {
    let focal = i32::from(lens.focal);
    let radius = lens.radius as f32;
    let radius_of = |d: i32| -> u8 {
        (radius * (d - focal).unsigned_abs() as f32 / 255.0)
            .round()
            .clamp(0.0, radius) as u8
    };
    let mut radii = vec![0u8; w * h];
    match depth {
        None => radii.fill(radius_of(255)),
        Some(map) if map.width() as usize == w && map.height() as usize == h => {
            for y in 0..h {
                for x in 0..w {
                    let raw = i32::from(map.at(x as u32, y as u32));
                    radii[y * w + x] = radius_of(if lens.invert { 255 - raw } else { raw });
                }
            }
        }
        Some(map) => {
            for y in 0..h {
                let v = (y as f32 + 0.5) / h as f32 * map.height() as f32 - 0.5;
                for x in 0..w {
                    let u = (x as f32 + 0.5) / w as f32 * map.width() as f32 - 0.5;
                    let raw = map.stretched(u, v).round().clamp(0.0, 255.0) as i32;
                    radii[y * w + x] = radius_of(if lens.invert { 255 - raw } else { raw });
                }
            }
        }
    }
    radii
}

/// The iris taps for every integer radius up to the largest used, built once
/// per apply and read-only across the row workers.
struct TapTables {
    taps: Vec<Vec<(i16, i16, f32)>>,
}

impl TapTables {
    fn build(lens: &LensParams, max_r: usize) -> Self {
        let mut taps = Vec::with_capacity(max_r + 1);
        taps.push(Vec::new());
        let blades = lens.shape.blades() as f64;
        let sector = 2.0 * PI / blades;
        let apothem = (PI / blades).cos();
        let curvature = lens.curvature as f64 / 100.0;
        let rotation = lens.rotation.to_radians();
        let step = if lens.faster { 2 } else { 1 };
        for r in 1..=max_r {
            let rf = r as f64;
            let edge = r as i32;
            let mut list = Vec::new();
            for dy in (-edge..=edge).step_by(step) {
                for dx in (-edge..=edge).step_by(step) {
                    let rho = ((dx * dx + dy * dy) as f64).sqrt();
                    if rho > rf + 0.5 {
                        continue;
                    }
                    let theta = (dy as f64).atan2(dx as f64);
                    let phi = (theta - rotation).rem_euclid(sector);
                    let polygon = apothem / (phi - PI / blades).cos();
                    let boundary = polygon + (1.0 - polygon) * curvature;
                    let weight = (boundary * rf - rho + 0.5).clamp(0.0, 1.0);
                    if weight > 0.0 {
                        list.push((dx as i16, dy as i16, weight as f32));
                    }
                }
            }
            taps.push(list);
        }
        Self { taps }
    }

    fn taps(&self, r: usize) -> &[(i16, i16, f32)] {
        self.taps.get(r).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// One noise sample in `[-1, 1)`, uniform or a near-Gaussian bell, from the
/// filter's own seed so equal inputs replay bit-identically.
fn jitter(lens: &LensParams, x: u32, y: u32, channel: u32) -> f32 {
    let salt = (lens.seed as u32) ^ ((lens.seed >> 32) as u32).rotate_left(13);
    let base = hash2(x ^ salt, y.wrapping_add(salt.rotate_left(16)));
    let draw = |k: u32| -> f32 {
        let h = hash2(base.wrapping_add(k), salt);
        (h % 2001) as f32 / 1000.0 - 1.0
    };
    if lens.noise_distribution == NoiseDistribution::Uniform {
        return draw(channel);
    }
    (draw(channel) + draw(channel + 101) + draw(channel + 211)) / 3.0 * 1.732
}

#[inline]
fn hash2(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    h
}

#[cfg(test)]
mod tests;
