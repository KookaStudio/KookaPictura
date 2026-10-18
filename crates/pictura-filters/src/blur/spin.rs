//! Spin Blur: rotational motion blur inside ellipses (`FILT-092`; a post-CS6
//! Blur Gallery effect ported on the owner's request, not CS6 parity), with
//! the gallery's Noise pass.
//!
//! Each spin is an ellipse with a feather ring and a rotation centre (its
//! own centre unless moved). A pixel's mask is 1 inside the feather ring, 0
//! outside the ellipse, and a smoothstep between; in several spins it takes
//! the one with the largest mask. The pixel then averages taps along the
//! circle through it round that spin's rotation centre, centred on the
//! pixel, with the strobe flashes mixed in. Inside the feather ring the arc
//! sweeps the full Blur Angle; outside it the arc keeps the ring's length
//! times the mask, so the blur weakens toward the outline (as the owner's
//! CC screenshots do) instead of lengthening with the radius.
//!
//! ponytail: Adobe's mask and flash model are closed. The smoothstep, the
//! largest-mask combine, the arc gather (at most 128 bilinear taps, so long
//! arcs far from the centre at large angles can ghost), and the flash
//! windows are inferred.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use super::field::{apply_effects, validate_noise, BlurNoise, MAX_FIELD_PINS};
use super::path::BOKEH_OFF;
use super::tilt::bilinear;
use crate::{validate, FilterError};

/// The largest Blur Angle, in degrees.
pub const MAX_SPIN_ANGLE: f64 = 360.0;

/// The smallest spin radius, in buffer widths.
const MIN_SPIN_RADIUS: f64 = 1.0e-4;

/// The largest spin radius, in buffer widths (as Iris's).
const MAX_SPIN_RADIUS: f64 = 10.0;

/// The most taps one pixel's arc samples.
const MAX_SPIN_TAPS: usize = 128;

/// One spin. `x`/`y` are its ellipse's centre as fractions of the buffer
/// (outside allowed); `radius_x`/`radius_y` the semi-axes as fractions of the
/// buffer **width**; `angle` turns the `radius_x` axis clockwise from +x, in
/// degrees; `feather` (0..=1) is the inner ring as a fraction of the outline;
/// `blur_angle` is the arc's sweep in degrees; `pivot_x`/`pivot_y` offset the
/// rotation centre from the ellipse's centre, as fractions of the buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinPin {
    pub x: f64,
    pub y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub angle: f64,
    pub feather: f64,
    pub blur_angle: f64,
    pub pivot_x: f64,
    pub pivot_y: f64,
}

impl Default for SpinPin {
    fn default() -> Self {
        // The owner's opening screenshot: a centred circle 0.13 of the width
        // across at 15°; the feather ring is unverified.
        Self {
            x: 0.5,
            y: 0.5,
            radius_x: 0.13,
            radius_y: 0.13,
            angle: 0.0,
            feather: 0.5,
            blur_angle: 15.0,
            pivot_x: 0.0,
            pivot_y: 0.0,
        }
    }
}

/// The Motion Effects tab's strobe, shared by every spin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinStrobe {
    /// 0..=100; mixes the continuous arc toward the flashes.
    pub strength: f64,
    /// 1..=100 flashes spread over the arc.
    pub flashes: u32,
    /// 0..=360 degrees of the arc each flash lights.
    pub duration: f64,
}

impl Default for SpinStrobe {
    fn default() -> Self {
        // The owner's screenshots' opening state.
        Self {
            strength: 0.0,
            flashes: 4,
            duration: 1.0,
        }
    }
}

/// Rotational blur inside the spins with the Noise pass. An empty spin list
/// is a no-op.
pub fn spin_blur(
    buf: &mut PixelBuffer,
    spins: &[SpinPin],
    strobe: &SpinStrobe,
    noise: &BlurNoise,
) -> Result<(), FilterError> {
    validate(buf)?;
    validate_spins(spins)?;
    validate_strobe(strobe)?;
    validate_noise(noise)?;
    if let Some(weights) = spin_pass(buf, spins, strobe) {
        apply_effects(buf, &weights, &BOKEH_OFF, noise);
    }
    Ok(())
}

/// The gather alone. Returns each pixel's mask as a weight (0 sharp, 255
/// fully spun) for the effects passes, or `None` (buffer untouched) when no
/// spin sweeps. Callers validate first.
pub(super) fn spin_pass(
    buf: &mut PixelBuffer,
    spins: &[SpinPin],
    strobe: &SpinStrobe,
) -> Option<Vec<u8>> {
    if spins.iter().all(|spin| spin.blur_angle <= 0.0) {
        return None;
    }
    let (w, h) = (buf.width as usize, buf.height as usize);
    let shapes: Vec<Spin> = spins
        .iter()
        .map(|spin| Spin::new(spin, w as f64, h as f64))
        .collect();
    let n = w * h;
    let planes = (buf.channels as usize).min(4);
    let src = buf.data[..planes * n].to_vec();
    let gathered: Vec<([u8; 4], u8)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            let mut out = [0u8; 4];
            for (c, slot) in out.iter_mut().enumerate().take(planes) {
                *slot = src[c * n + i];
            }
            let mut best: Option<(&Spin, f64, f64)> = None;
            for shape in &shapes {
                let (mask, rho) = shape.mask(x, y);
                if mask > best.map_or(0.0, |(_, m, _)| m) {
                    best = Some((shape, mask, rho));
                }
            }
            let Some((spin, mask, rho)) = best else {
                return (out, 0);
            };
            let sweep = spin.sweep_at(mask, rho);
            let (dx, dy) = (x - spin.px, y - spin.py);
            let length = dx.hypot(dy) * sweep;
            if length < 1.0 {
                return (out, 0);
            }
            let mut taps = [(0.0, 0.0, 0.0); MAX_SPIN_TAPS];
            let count = arc(
                strobe,
                (spin.px, spin.py),
                (dx, dy),
                length,
                sweep,
                &mut taps,
            );
            for (c, slot) in out.iter_mut().enumerate().take(planes) {
                let plane = &src[c * n..(c + 1) * n];
                let sum: f64 = taps[..count]
                    .iter()
                    .map(|&(tx, ty, weight)| weight * bilinear(plane, w, h, tx - 0.5, ty - 0.5))
                    .sum();
                *slot = sum.round().clamp(0.0, 255.0) as u8;
            }
            (out, (mask * 255.0).round() as u8)
        })
        .collect();
    for c in 0..planes {
        buf.data[c * n..(c + 1) * n]
            .par_iter_mut()
            .zip(gathered.par_iter())
            .for_each(|(out, (pixel, _))| *out = pixel[c]);
    }
    Some(gathered.into_iter().map(|(_, weight)| weight).collect())
}

/// Fill `taps` with a pixel's arc — `(x, y, weight)` in pixel-centre
/// coordinates, weights summing to 1 — round `centre`, the pixel at `offset`
/// from it, sweeping `sweep` radians over `length` pixels; return how many.
fn arc(
    strobe: &SpinStrobe,
    centre: (f64, f64),
    offset: (f64, f64),
    length: f64,
    sweep: f64,
    taps: &mut [(f64, f64, f64); MAX_SPIN_TAPS],
) -> usize {
    let count = (length.ceil() as usize + 1).clamp(2, MAX_SPIN_TAPS);
    let step = sweep / (count - 1) as f64;
    let phase = |k: usize| -sweep / 2.0 + step * k as f64;
    for (k, tap) in taps.iter_mut().enumerate().take(count) {
        let (sin, cos) = phase(k).sin_cos();
        *tap = (
            centre.0 + offset.0 * cos - offset.1 * sin,
            centre.1 + offset.0 * sin + offset.1 * cos,
            1.0 / count as f64,
        );
    }
    let strength = strobe.strength / 100.0;
    if strength <= 0.0 {
        return count;
    }
    // Each flash lights the taps within half its duration, and at least the
    // nearest one, so a short flash on a coarse arc still lands.
    let mut lit = [false; MAX_SPIN_TAPS];
    let half = strobe.duration.to_radians() / 2.0;
    let flashes = strobe.flashes as usize;
    for j in 0..flashes {
        let at = if flashes == 1 {
            0.0
        } else {
            -sweep / 2.0 + sweep * j as f64 / (flashes - 1) as f64
        };
        let nearest = ((at + sweep / 2.0) / step).round() as usize;
        lit[nearest.min(count - 1)] = true;
        for (k, on) in lit.iter_mut().enumerate().take(count) {
            *on |= (phase(k) - at).abs() <= half;
        }
    }
    let lit_count = lit[..count].iter().filter(|&&on| on).count() as f64;
    for (tap, &on) in taps.iter_mut().zip(&lit).take(count) {
        let flash = if on { 1.0 / lit_count } else { 0.0 };
        tap.2 = tap.2 * (1.0 - strength) + flash * strength;
    }
    count
}

/// One spin resolved to buffer pixels.
struct Spin {
    cx: f64,
    cy: f64,
    px: f64,
    py: f64,
    cos: f64,
    sin: f64,
    rx: f64,
    ry: f64,
    feather: f64,
    sweep: f64,
}

impl Spin {
    fn new(spin: &SpinPin, w: f64, h: f64) -> Self {
        let (sin, cos) = spin.angle.to_radians().sin_cos();
        let (cx, cy) = (spin.x * w, spin.y * h);
        Self {
            cx,
            cy,
            px: cx + spin.pivot_x * w,
            py: cy + spin.pivot_y * h,
            cos,
            sin,
            rx: spin.radius_x * w,
            ry: spin.radius_y * w,
            feather: spin.feather,
            sweep: spin.blur_angle.to_radians(),
        }
    }

    /// The arc's sweep in radians at a mask and normalized radius. Past the
    /// feather ring the angle shrinks with the radius, so the arc is the
    /// ring's arc (exact with the rotation centre on the ellipse's) faded by
    /// the mask.
    fn sweep_at(&self, mask: f64, rho: f64) -> f64 {
        let ring = if rho <= self.feather {
            1.0
        } else {
            self.feather / rho
        };
        self.sweep * mask * ring
    }

    /// The mask — 1 inside the feather ring, 0 outside the ellipse, a
    /// smoothstep fall across the band between them — and the normalized
    /// elliptical radius.
    fn mask(&self, x: f64, y: f64) -> (f64, f64) {
        let (dx, dy) = (x - self.cx, y - self.cy);
        let u = (dx * self.cos + dy * self.sin) / self.rx;
        let v = (dy * self.cos - dx * self.sin) / self.ry;
        let rho = u.hypot(v);
        if rho >= 1.0 {
            return (0.0, rho);
        }
        if rho <= self.feather {
            return (1.0, rho);
        }
        let t = (rho - self.feather) / (1.0 - self.feather);
        (1.0 - t * t * (3.0 - 2.0 * t), rho)
    }
}

pub(super) fn validate_spins(spins: &[SpinPin]) -> Result<(), FilterError> {
    if spins.len() > MAX_FIELD_PINS {
        return Err(FilterError::InvalidParams(format!(
            "spin blur takes at most {MAX_FIELD_PINS} spins, got {}",
            spins.len()
        )));
    }
    for spin in spins {
        let values = [
            spin.x,
            spin.y,
            spin.radius_x,
            spin.radius_y,
            spin.angle,
            spin.feather,
            spin.blur_angle,
            spin.pivot_x,
            spin.pivot_y,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(FilterError::InvalidParams(
                "spin blur values must be finite".into(),
            ));
        }
        for (name, value) in [("x radius", spin.radius_x), ("y radius", spin.radius_y)] {
            if !(MIN_SPIN_RADIUS..=MAX_SPIN_RADIUS).contains(&value) {
                return Err(FilterError::InvalidParams(format!(
                    "spin {name} {value} is outside {MIN_SPIN_RADIUS}..={MAX_SPIN_RADIUS}"
                )));
            }
        }
        if !(0.0..=1.0).contains(&spin.feather) {
            return Err(FilterError::InvalidParams(format!(
                "spin feather {} is outside 0..=1",
                spin.feather
            )));
        }
        if !(0.0..=MAX_SPIN_ANGLE).contains(&spin.blur_angle) {
            return Err(FilterError::InvalidParams(format!(
                "spin blur angle {} is outside 0..={MAX_SPIN_ANGLE}",
                spin.blur_angle
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_strobe(strobe: &SpinStrobe) -> Result<(), FilterError> {
    for (name, value, top) in [
        ("strobe strength", strobe.strength, 100.0),
        ("strobe flash duration", strobe.duration, MAX_SPIN_ANGLE),
    ] {
        if !value.is_finite() || !(0.0..=top).contains(&value) {
            return Err(FilterError::InvalidParams(format!(
                "spin blur {name} {value} is outside 0..={top}"
            )));
        }
    }
    if !(1..=100).contains(&strobe.flashes) {
        return Err(FilterError::InvalidParams(format!(
            "strobe flashes {} is outside 1..=100",
            strobe.flashes
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
