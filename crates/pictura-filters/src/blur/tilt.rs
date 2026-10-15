//! Tilt-Shift: a sharp band with a fade to blur on either side
//! (`FILT-092`), on the Blur Gallery's shared pyramid and Blur Effects /
//! Noise passes.
//!
//! Each pin carries a band through its centre at `angle`: inside the focus
//! lines (solid in the workspace) the picture stays sharp, across the fade
//! (out to the dashed lines) the blur ramps up (smoothstep), and beyond it is
//! the pin's full amount. Several pins multiply their falloffs and blend
//! their amounts by inverse-square distance, as Iris Blur does. Distortion
//! smears the blurred zone below the band (both zones when symmetric) along
//! the radius from the pin (positive) or around it (negative).
//!
//! ponytail: Adobe's band falloff and distortion shader are closed; the
//! smoothstep fade, the product combine, and the line-sampled smear (at most
//! 33 bilinear taps over twice the local radius) are inferred. The blur
//! itself inherits Field Blur's box-pyramid approximation of the disc gather.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use super::field::{
    apply_effects, pyramid_blur, validate_bokeh, validate_noise, BlurNoise, BokehParams,
    MAX_FIELD_PINS, MAX_FIELD_RADIUS,
};
use crate::{validate, FilterError};

/// The distance floor of the inverse-square amount weights, in buffer widths.
const PIN_EPSILON: f64 = 1.0e-6;

/// The widest focus or fade extent, in buffer heights.
pub const MAX_TILT_EXTENT: f64 = 10.0;

/// The most taps one smeared pixel samples.
const MAX_SMEAR_TAPS: usize = 33;

/// One Tilt-Shift pin. `x`/`y` are fractions of the buffer (outside
/// allowed); `angle` turns the band clockwise from horizontal, in degrees.
/// The four extents are buffer **heights** measured across the band:
/// `focus_*` from the centre line to each solid line, `fade_*` from each
/// solid line to its dashed line. *Above* is the side the band's normal
/// points away from (above the band at angle 0). `distortion` (−100..=100)
/// smears the blurred zone below the band, both zones when `symmetric`;
/// `blur` is the full radius in buffer pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TiltPin {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub focus_above: f64,
    pub focus_below: f64,
    pub fade_above: f64,
    pub fade_below: f64,
    pub distortion: f64,
    pub symmetric: bool,
    pub blur: f64,
}

impl Default for TiltPin {
    fn default() -> Self {
        // The owner's screenshots' opening band (focus ≈ 0.1, fade ≈ 0.15 of
        // the height each way); 15 px is the gallery's default amount
        // (unverified).
        Self {
            x: 0.5,
            y: 0.5,
            angle: 0.0,
            focus_above: 0.1,
            focus_below: 0.1,
            fade_above: 0.15,
            fade_below: 0.15,
            distortion: 0.0,
            symmetric: false,
            blur: 15.0,
        }
    }
}

/// Band-variable blur with the distortion smear and the Blur Effects and
/// Noise passes. An empty pin list is a no-op.
pub fn tilt_shift(
    buf: &mut PixelBuffer,
    pins: &[TiltPin],
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
    let (w, h) = (f64::from(buf.width), f64::from(buf.height));
    let field = TiltField::new(pins, w, h);
    let Some(weights) = pyramid_blur(buf, max_pin, |x, y| field.sample(x, y).0) else {
        return Ok(());
    };
    if field.distorts() {
        smear(buf, &field);
    }
    apply_effects(buf, &weights, bokeh, noise);
    Ok(())
}

/// One pin resolved to buffer pixels.
struct Band {
    cx: f64,
    cy: f64,
    cos: f64,
    sin: f64,
    focus: [f64; 2],
    fade: [f64; 2],
    distortion: f64,
    symmetric: bool,
    blur: f64,
}

impl Band {
    fn new(pin: &TiltPin, w: f64, h: f64) -> Self {
        let (sin, cos) = pin.angle.to_radians().sin_cos();
        Self {
            cx: pin.x * w,
            cy: pin.y * h,
            cos,
            sin,
            focus: [pin.focus_above * h, pin.focus_below * h],
            fade: [pin.fade_above * h, pin.fade_below * h],
            distortion: pin.distortion / 100.0,
            symmetric: pin.symmetric,
            blur: pin.blur,
        }
    }

    /// The signed distance across the band, positive below it.
    fn across(&self, x: f64, y: f64) -> f64 {
        (y - self.cy) * self.cos - (x - self.cx) * self.sin
    }

    /// 0 between the focus lines, 1 past the fade, a smoothstep between.
    fn falloff(&self, x: f64, y: f64) -> f64 {
        let v = self.across(x, y);
        let side = usize::from(v > 0.0);
        let d = v.abs() - self.focus[side];
        if d <= 0.0 {
            return 0.0;
        }
        let fade = self.fade[side];
        if d >= fade {
            return 1.0;
        }
        let t = d / fade;
        t * t * (3.0 - 2.0 * t)
    }

    /// The unit smear direction scaled by the signed distortion, or zero on a
    /// side the distortion does not reach.
    fn smear(&self, x: f64, y: f64) -> (f64, f64) {
        if self.distortion == 0.0 || (!self.symmetric && self.across(x, y) <= 0.0) {
            return (0.0, 0.0);
        }
        let (dx, dy) = (x - self.cx, y - self.cy);
        let length = dx.hypot(dy);
        if length <= 0.0 {
            return (0.0, 0.0);
        }
        let (ux, uy) = (dx / length, dy / length);
        let k = self.distortion.abs();
        if self.distortion > 0.0 {
            (ux * k, uy * k)
        } else {
            (-uy * k, ux * k)
        }
    }
}

pub(super) struct TiltField {
    bands: Vec<Band>,
    w: f64,
}

impl TiltField {
    pub(super) fn new(pins: &[TiltPin], w: f64, h: f64) -> Self {
        let bands = pins.iter().map(|pin| Band::new(pin, w, h)).collect();
        Self { bands, w }
    }

    pub(super) fn distorts(&self) -> bool {
        self.bands.iter().any(|band| band.distortion != 0.0)
    }

    /// A pixel centre's blur radius and its smear vector in buffer pixels.
    pub(super) fn sample(&self, x: f64, y: f64) -> (f64, f64, f64) {
        let mut falloff = 1.0;
        let (mut weighted, mut weight_sum) = (0.0, 0.0);
        let (mut sx, mut sy) = (0.0, 0.0);
        for band in &self.bands {
            falloff *= band.falloff(x, y);
            let (dx, dy) = ((x - band.cx) / self.w, (y - band.cy) / self.w);
            let weight = 1.0 / (dx * dx + dy * dy + PIN_EPSILON);
            weighted += weight * band.blur;
            weight_sum += weight;
            let (ux, uy) = band.smear(x, y);
            sx += weight * ux;
            sy += weight * uy;
        }
        let radius = falloff * weighted / weight_sum;
        // Twice the local radius at full distortion.
        let scale = 2.0 * radius / weight_sum;
        (radius, sx * scale, sy * scale)
    }
}

/// Average each pixel along its smear vector (centred on it) from the blurred
/// planes, alpha included; pixels with a sub-pixel smear keep their value.
pub(super) fn smear(buf: &mut PixelBuffer, field: &TiltField) {
    let (w, h) = (buf.width as usize, buf.height as usize);
    let n = w * h;
    let planes = (buf.channels as usize).min(4);
    let src = buf.data[..planes * n].to_vec();
    let vectors: Vec<(f64, f64)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (_, sx, sy) = field.sample((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            (sx, sy)
        })
        .collect();
    for c in 0..planes {
        let plane = &src[c * n..(c + 1) * n];
        buf.data[c * n..(c + 1) * n]
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, out)| {
                let (sx, sy) = vectors[i];
                let length = sx.hypot(sy);
                if length < 1.0 {
                    return;
                }
                let taps = (length.ceil() as usize + 1).clamp(2, MAX_SMEAR_TAPS);
                let (x, y) = ((i % w) as f64, (i / w) as f64);
                let mut sum = 0.0;
                for k in 0..taps {
                    let t = k as f64 / (taps - 1) as f64 - 0.5;
                    sum += bilinear(plane, w, h, x + sx * t, y + sy * t);
                }
                *out = (sum / taps as f64).round().clamp(0.0, 255.0) as u8;
            });
    }
}

/// A clamped-edge bilinear sample at pixel coordinates.
pub(super) fn bilinear(plane: &[u8], w: usize, h: usize, x: f64, y: f64) -> f64 {
    let x = x.clamp(0.0, (w - 1) as f64);
    let y = y.clamp(0.0, (h - 1) as f64);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (x - x0 as f64, y - y0 as f64);
    let at = |x: usize, y: usize| f64::from(plane[y * w + x]);
    let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
    let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
    top * (1.0 - ty) + bottom * ty
}

pub(super) fn validate_pins(pins: &[TiltPin]) -> Result<(), FilterError> {
    if pins.len() > MAX_FIELD_PINS {
        return Err(FilterError::InvalidParams(format!(
            "tilt-shift takes at most {MAX_FIELD_PINS} pins, got {}",
            pins.len()
        )));
    }
    for pin in pins {
        let values = [
            pin.x,
            pin.y,
            pin.angle,
            pin.focus_above,
            pin.focus_below,
            pin.fade_above,
            pin.fade_below,
            pin.distortion,
            pin.blur,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(FilterError::InvalidParams(
                "tilt-shift pin values must be finite".into(),
            ));
        }
        for (name, value) in [
            ("focus above", pin.focus_above),
            ("focus below", pin.focus_below),
            ("fade above", pin.fade_above),
            ("fade below", pin.fade_below),
        ] {
            if !(0.0..=MAX_TILT_EXTENT).contains(&value) {
                return Err(FilterError::InvalidParams(format!(
                    "tilt-shift {name} {value} is outside 0..={MAX_TILT_EXTENT}"
                )));
            }
        }
        if !(-100.0..=100.0).contains(&pin.distortion) {
            return Err(FilterError::InvalidParams(format!(
                "tilt-shift distortion {} is outside -100..=100",
                pin.distortion
            )));
        }
        if !(0.0..=MAX_FIELD_RADIUS as f64).contains(&pin.blur) {
            return Err(FilterError::InvalidParams(format!(
                "tilt-shift blur amount {} is outside 0..={MAX_FIELD_RADIUS}",
                pin.blur
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
