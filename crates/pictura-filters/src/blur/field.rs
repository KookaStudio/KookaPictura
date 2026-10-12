//! Field Blur: a pin-interpolated variable blur (`FILT-092`), with the Blur
//! Gallery's Blur Effects (bokeh) and Noise post-passes.
//!
//! Adobe's Blur Gallery shaders are closed; this is the documented model — a
//! per-pixel radius field from inverse-square-distance pin weights, applied
//! through a box-blur level pyramid, then the bokeh lift and re-injected
//! grain. Only the color planes are written; the render layer's alpha pass
//! blurs the alpha channel without the colour-only effects.
//!
//! ponytail: the doc's inferred disc/bokeh gather is approximated by blending
//! two levels of a Gaussian-like box pyramid. A per-pixel disc gather costs
//! `O(Σ CoC area)` per pixel (the Lens Blur path) and cannot drive a live pin
//! drag; out-of-focus highlights therefore blur softly instead of staying
//! disc-shaped. The bokeh lift/tint curves and the grain model are inferred
//! approximations (Adobe's are closed).

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::{validate, FilterError};

/// The UI's largest Field Blur amount (`FILT-092`; the range is unverified).
pub const MAX_FIELD_RADIUS: u32 = 500;

/// The most pins one field carries.
pub const MAX_FIELD_PINS: usize = 16;

/// The distance floor of the inverse-square pin weights, in buffer fractions.
const PIN_EPSILON: f64 = 1.0e-6;

/// One Field Blur pin: `x`/`y` are fractions of the buffer (0..1 inside; a
/// pin outside anchors the border), `blur` is a radius in buffer pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurPin {
    pub x: f64,
    pub y: f64,
    pub blur: f64,
}

/// Blur Gallery Blur Effects (CS6): Light Bokeh, Bokeh Color, and the Light
/// Range that gates both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BokehParams {
    pub enabled: bool,
    /// 0..=100; brightens the blurred areas in the tonal range.
    pub light_bokeh: f64,
    /// 0..=100; adds chroma to the lightened, non-clipped pixels.
    pub bokeh_color: f64,
    /// The 0..=255 tonal window the two effects act in.
    pub light_range: (u8, u8),
}

impl Default for BokehParams {
    fn default() -> Self {
        // The owner's screenshot's opening state.
        Self {
            enabled: true,
            light_bokeh: 14.0,
            bokeh_color: 40.0,
            light_range: (40, 118),
        }
    }
}

/// The Noise tab's grain models, in the dropdown's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurNoiseKind {
    /// A near-Gaussian per-pixel draw.
    Gaussian,
    /// A uniform per-pixel draw.
    Uniform,
    /// Interpolated lattice cells; Size sets the cell, Roughness blends toward
    /// per-pixel jitter.
    Grain,
}

/// The Blur Gallery Noise effect (post-CS6, ported on the owner's request):
/// grain re-injected into the blurred areas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurNoise {
    pub enabled: bool,
    /// 0..=100; the grain amplitude.
    pub amount: f64,
    /// 0..=100; the Grain model's cell size.
    pub size: f64,
    /// 0..=100; blends the Grain model's smooth lattice toward per-pixel noise.
    pub roughness: f64,
    /// 0..=100; 0 keeps the channels equal (monochromatic), 100 draws each.
    pub color: f64,
    /// 0..=100; damps the grain in bright areas.
    pub highlights: f64,
    pub kind: BlurNoiseKind,
    pub seed: u64,
}

impl Default for BlurNoise {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.0,
            size: 50.0,
            roughness: 50.0,
            color: 0.0,
            highlights: 0.0,
            kind: BlurNoiseKind::Gaussian,
            seed: 1,
        }
    }
}

/// Pin-interpolated variable blur with the Blur Effects and Noise passes. An
/// empty pin list is a no-op.
pub fn field_blur(
    buf: &mut PixelBuffer,
    pins: &[BlurPin],
    bokeh: &BokehParams,
    noise: &BlurNoise,
) -> Result<(), FilterError> {
    validate(buf)?;
    validate_pins(pins)?;
    validate_bokeh(bokeh)?;
    validate_noise(noise)?;
    if pins.is_empty() {
        return Ok(());
    }
    let max_pin = pins.iter().map(|pin| pin.blur).fold(0.0f64, f64::max);
    if max_pin <= 0.0 {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let boxes = level_boxes(max_pin);
    let nominal = level_nominals(&boxes);
    let (brackets, weights) = bracket_map(pins, w, h, &nominal, max_pin);
    let src = buf.data.clone();
    for c in 0..planes {
        let base = c * n;
        let src_plane = &src[base..base + n];
        let mut level: Vec<f64> = src_plane.iter().map(|&v| f64::from(v)).collect();
        let mut scratch = vec![0f64; n];
        let mut previous: Vec<u8> = src_plane.to_vec();
        let out = &mut buf.data[base..base + n];
        for (index, &radius) in boxes.iter().enumerate() {
            for _ in 0..3 {
                super::box_blur_axis(&level, &mut scratch, w, h, radius as isize, true);
                super::box_blur_axis(&scratch, &mut level, w, h, radius as isize, false);
            }
            out.par_iter_mut().enumerate().for_each(|(i, pixel)| {
                if usize::from(brackets[i] >> 11) != index {
                    return;
                }
                let t = f64::from(brackets[i] & 0x07FF) / 2047.0;
                let from = f64::from(previous[i]);
                *pixel = (from + (level[i] - from) * t).round().clamp(0.0, 255.0) as u8;
            });
            if index + 1 < boxes.len() {
                previous.par_iter_mut().enumerate().for_each(|(i, pixel)| {
                    *pixel = level[i].round().clamp(0.0, 255.0) as u8;
                });
            }
        }
    }
    apply_effects(buf, &weights, bokeh, noise);
    Ok(())
}

fn validate_pins(pins: &[BlurPin]) -> Result<(), FilterError> {
    if pins.len() > MAX_FIELD_PINS {
        return Err(FilterError::InvalidParams(format!(
            "field blur takes at most {MAX_FIELD_PINS} pins, got {}",
            pins.len()
        )));
    }
    for pin in pins {
        if !pin.x.is_finite() || !pin.y.is_finite() || !pin.blur.is_finite() {
            return Err(FilterError::InvalidParams(
                "field blur pin coordinates and amounts must be finite".into(),
            ));
        }
        if !(0.0..=MAX_FIELD_RADIUS as f64).contains(&pin.blur) {
            return Err(FilterError::InvalidParams(format!(
                "field blur amount {} is outside 0..={MAX_FIELD_RADIUS}",
                pin.blur
            )));
        }
    }
    Ok(())
}

fn validate_bokeh(bokeh: &BokehParams) -> Result<(), FilterError> {
    if !bokeh.light_bokeh.is_finite()
        || !bokeh.bokeh_color.is_finite()
        || !(0.0..=100.0).contains(&bokeh.light_bokeh)
        || !(0.0..=100.0).contains(&bokeh.bokeh_color)
    {
        return Err(FilterError::InvalidParams(
            "bokeh light and color span 0..=100".into(),
        ));
    }
    Ok(())
}

fn validate_noise(noise: &BlurNoise) -> Result<(), FilterError> {
    for (name, value) in [
        ("amount", noise.amount),
        ("size", noise.size),
        ("roughness", noise.roughness),
        ("color", noise.color),
        ("highlights", noise.highlights),
    ] {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            return Err(FilterError::InvalidParams(format!(
                "noise {name} {value} is outside 0..=100"
            )));
        }
    }
    Ok(())
}

/// The Blur Effects and Noise passes over the blurred pixels: the field weight
/// gates both, so sharp pixels never change. Runs on interleaved pixels, then
/// scatters back to the planes.
fn apply_effects(buf: &mut PixelBuffer, weights: &[u8], bokeh: &BokehParams, noise: &BlurNoise) {
    let bokeh_on = bokeh.enabled && (bokeh.light_bokeh > 0.0 || bokeh.bokeh_color > 0.0);
    let noise_on = noise.enabled && noise.amount > 0.0;
    if !bokeh_on && !noise_on {
        return;
    }
    let n = buf.pixel_count();
    let width = buf.width as usize;
    let pixels: Vec<[u8; 3]> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (mut r, mut g, mut b) = (
                f64::from(buf.data[i]),
                f64::from(buf.data[n + i]),
                f64::from(buf.data[2 * n + i]),
            );
            let weight = f64::from(weights[i]) / 255.0;
            if weight <= 0.0 {
                return [r as u8, g as u8, b as u8];
            }
            if bokeh_on {
                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                let (low, high) = bokeh.light_range;
                if luma >= f64::from(low) && luma <= f64::from(high) {
                    // A doubled multiplicative brighten, so a bright,
                    // well-blurred pixel blooms brilliantly toward white as
                    // the reference shows.
                    let lift = bokeh.light_bokeh / 100.0 * (luma / 255.0) * weight;
                    if lift > 0.0 {
                        let scale = 1.0 + 2.0 * lift;
                        r *= scale;
                        g *= scale;
                        b *= scale;
                    }
                    let blown = r > 250.0 && g > 250.0 && b > 250.0;
                    let tint = bokeh.bokeh_color / 100.0 * weight;
                    if tint > 0.0 && !blown {
                        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                        let spread = 1.0 + tint;
                        r = luma + (r - luma) * spread;
                        g = luma + (g - luma) * spread;
                        b = luma + (b - luma) * spread;
                    }
                }
            }
            if noise_on {
                let luma = 0.299 * r + 0.587 * g + 0.114 * b;
                let gate = (1.0 - noise.highlights / 100.0 * luma / 255.0).clamp(0.0, 1.0);
                let amplitude = noise.amount / 100.0 * 127.5 * weight * gate;
                if amplitude > 0.0 {
                    let (x, y) = ((i % width) as u32, (i / width) as u32);
                    let mono = noise_value(noise, x, y, 0);
                    let mix = noise.color / 100.0;
                    let draw =
                        |channel: u32| mono * (1.0 - mix) + noise_value(noise, x, y, channel) * mix;
                    r += draw(0) * amplitude;
                    g += draw(1) * amplitude;
                    b += draw(2) * amplitude;
                }
            }
            [to_u8(r), to_u8(g), to_u8(b)]
        })
        .collect();
    for c in 0..3 {
        let plane = c * n;
        buf.data[plane..plane + n]
            .par_iter_mut()
            .zip(pixels.par_iter())
            .for_each(|(out, pixel)| *out = pixel[c]);
    }
}

fn to_u8(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// One noise sample in `[-1, 1)`; `channel` separates the colour draws.
fn noise_value(noise: &BlurNoise, x: u32, y: u32, channel: u32) -> f64 {
    let salt = (noise.seed as u32)
        ^ ((noise.seed >> 32) as u32).rotate_left(13)
        ^ channel.wrapping_mul(0x9E37_79B1);
    match noise.kind {
        BlurNoiseKind::Uniform => unit(x as i64, y as i64, salt) * 2.0 - 1.0,
        BlurNoiseKind::Gaussian => {
            // Three uniform draws summed give a near-Gaussian bell; 1.732
            // restores the unit variance the uniform draw has.
            let bell = (unit(x as i64, y as i64, salt)
                + unit(x as i64, y as i64, salt ^ 0x9E37_79B1)
                + unit(x as i64, y as i64, salt ^ 0x85EB_CA77))
                / 3.0;
            ((bell * 2.0 - 1.0) * 1.732).clamp(-1.0, 1.0)
        }
        BlurNoiseKind::Grain => {
            let cell = 1.0 + noise.size / 100.0 * 15.0;
            let fx = f64::from(x) / cell;
            let fy = f64::from(y) / cell;
            let (x0, y0) = (fx.floor(), fy.floor());
            let (tx, ty) = (fx - x0, fy - y0);
            let at = |dx: f64, dy: f64| unit(x0 as i64 + dx as i64, y0 as i64 + dy as i64, salt);
            let top = at(0.0, 0.0) * (1.0 - tx) + at(1.0, 0.0) * tx;
            let bottom = at(0.0, 1.0) * (1.0 - tx) + at(1.0, 1.0) * tx;
            let smooth = top * (1.0 - ty) + bottom * ty;
            let rough = unit(x as i64, y as i64, salt);
            let t = noise.roughness / 100.0;
            (smooth * (1.0 - t) + rough * t) * 2.0 - 1.0
        }
    }
}

/// A deterministic hash in `[0, 1)`.
fn unit(x: i64, y: i64, salt: u32) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ u64::from(salt);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// The box radius of each pyramid level after the source. Three box passes of
/// radius `r` add variance `r(r+1)` (`3 × r(r+1)/3`); each level doubles the
/// variance, so a level's radius grows about √2 and the nominal radii (3σ)
/// form a smooth ladder.
fn level_boxes(max_radius: f64) -> Vec<u32> {
    let mut boxes = Vec::new();
    let mut variance = 0.0f64;
    loop {
        let solved = if variance <= 0.0 {
            1.0
        } else {
            ((1.0 + 4.0 * variance).sqrt() - 1.0) / 2.0
        };
        let radius = (solved.round().max(1.0) as u32).max(boxes.last().copied().unwrap_or(0) + 1);
        variance += f64::from(radius) * f64::from(radius + 1);
        boxes.push(radius);
        if 3.0 * variance.sqrt() >= max_radius {
            return boxes;
        }
    }
}

/// The nominal 3σ radius of the source and each pyramid level.
fn level_nominals(boxes: &[u32]) -> Vec<f64> {
    let mut nominal = Vec::with_capacity(boxes.len() + 1);
    let mut variance = 0.0;
    nominal.push(0.0);
    for &radius in boxes {
        variance += f64::from(radius) * f64::from(radius + 1);
        nominal.push(3.0 * variance.sqrt());
    }
    nominal
}

/// Each pixel's bracketing level and blend packed as `(level << 11) | t`, plus
/// its normalized field weight (0 sharp, 255 at the largest pin) for the
/// effects passes.
fn bracket_map(
    pins: &[BlurPin],
    w: usize,
    h: usize,
    nominal: &[f64],
    max_pin: f64,
) -> (Vec<u16>, Vec<u8>) {
    let mut brackets = vec![0u16; w * h];
    let mut weights = vec![0u8; w * h];
    brackets
        .par_iter_mut()
        .zip(weights.par_iter_mut())
        .enumerate()
        .for_each(|(i, (slot, weight))| {
            let x = i % w;
            let y = i / w;
            let u = (x as f64 + 0.5) / w as f64;
            let v = (y as f64 + 0.5) / h as f64;
            let mut weighted = 0.0;
            let mut weight_sum = 0.0;
            for pin in pins {
                let dx = u - pin.x;
                let dy = v - pin.y;
                let w = 1.0 / (dx * dx + dy * dy + PIN_EPSILON);
                weighted += w * pin.blur;
                weight_sum += w;
            }
            let radius = weighted / weight_sum;
            let (level, t) = bracket(nominal, radius);
            *slot = ((level as u16) << 11) | (t * 2047.0).round() as u16;
            *weight = ((radius / max_pin * 255.0).round().clamp(0.0, 255.0)) as u8;
        });
    (brackets, weights)
}

/// The level whose nominal radius starts the interval containing `radius`,
/// with the blend toward the next level.
fn bracket(nominal: &[f64], radius: f64) -> (usize, f64) {
    let last = nominal.len() - 1;
    if radius <= 0.0 {
        return (0, 0.0);
    }
    if radius >= nominal[last] {
        return (last - 1, 1.0);
    }
    let level = nominal.partition_point(|&n| n <= radius) - 1;
    let t = (radius - nominal[level]) / (nominal[level + 1] - nominal[level]);
    (level, t)
}

#[cfg(test)]
mod tests;
