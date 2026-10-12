//! Iris Blur: elliptical focus islands in a variable blur (`FILT-092`), on
//! the Blur Gallery's shared pyramid and Blur Effects / Noise passes.
//!
//! Each pin carries a superellipse: inside its feather core the picture stays
//! sharp, across the fade band the blur ramps up (smoothstep), and outside
//! the outline it is the pin's full amount. Several pins multiply their
//! falloffs, so every pin keeps its own sharp island; the exterior amount is
//! the inverse-square-distance blend of the pin amounts, as in Field Blur.
//!
//! ponytail: Adobe's iris falloff, roundness curve, and multi-pin combine are
//! closed; the smoothstep fade, the squareness-to-exponent map, and the
//! product of falloffs are inferred. The blur itself inherits Field Blur's
//! box-pyramid approximation of the disc/bokeh gather.

use pictura_core::PixelBuffer;

use super::field::{
    validate_bokeh, validate_noise, variable_blur, BlurNoise, BokehParams, MAX_FIELD_PINS,
    MAX_FIELD_RADIUS,
};
use crate::{validate, FilterError};

/// The distance floor of the inverse-square amount weights, in buffer widths.
const PIN_EPSILON: f64 = 1.0e-6;

/// The smallest iris radius, in buffer widths.
const MIN_IRIS_RADIUS: f64 = 1.0e-4;

/// The largest iris radius, in buffer widths: a pin may reach well past the
/// border, but not without bound.
pub const MAX_IRIS_RADIUS: f64 = 10.0;

/// One Iris Blur pin. `x`/`y` are fractions of the buffer (outside allowed);
/// `radius_x`/`radius_y` are the outline's semi-axes as fractions of the
/// buffer **width** (one unit for both, so rotation keeps the shape);
/// `angle` turns the `radius_x` axis clockwise from +x, in degrees;
/// `squareness` (0..=100) runs from an ellipse toward a rounded square;
/// `feather` (0..=1) is the sharp core as a fraction of the outline; `blur`
/// is the exterior radius in buffer pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IrisPin {
    pub x: f64,
    pub y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub angle: f64,
    pub squareness: f64,
    pub feather: f64,
    pub blur: f64,
}

impl Default for IrisPin {
    fn default() -> Self {
        // The owner's screenshot's opening shape (feather core ≈ 0.65, a
        // rounded outline); 15 px is CS6's Iris default (unverified).
        Self {
            x: 0.5,
            y: 0.5,
            radius_x: 0.35,
            radius_y: 0.35,
            angle: 0.0,
            squareness: 50.0,
            feather: 0.65,
            blur: 15.0,
        }
    }
}

/// The superellipse exponent for a squareness: 2 (an ellipse) at 0, 20 (a
/// near-square) at 100. The app's roundness handle inverts this.
pub fn iris_exponent(squareness: f64) -> f64 {
    2.0 / (1.0 - 0.009 * squareness)
}

/// Elliptical-focus variable blur with the Blur Effects and Noise passes. An
/// empty pin list is a no-op.
pub fn iris_blur(
    buf: &mut PixelBuffer,
    pins: &[IrisPin],
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
    let shapes: Vec<Shape> = pins.iter().map(|pin| Shape::new(pin, w, h)).collect();
    variable_blur(buf, max_pin, bokeh, noise, |x, y| {
        let mut falloff = 1.0;
        let mut weighted = 0.0;
        let mut weight_sum = 0.0;
        for shape in &shapes {
            falloff *= shape.falloff(x, y);
            let (dx, dy) = ((x - shape.cx) / w, (y - shape.cy) / w);
            let weight = 1.0 / (dx * dx + dy * dy + PIN_EPSILON);
            weighted += weight * shape.blur;
            weight_sum += weight;
        }
        falloff * weighted / weight_sum
    });
    Ok(())
}

/// One pin resolved to buffer pixels.
struct Shape {
    cx: f64,
    cy: f64,
    cos: f64,
    sin: f64,
    rx: f64,
    ry: f64,
    exponent: f64,
    feather: f64,
    blur: f64,
}

impl Shape {
    fn new(pin: &IrisPin, w: f64, h: f64) -> Self {
        let (sin, cos) = pin.angle.to_radians().sin_cos();
        Self {
            cx: pin.x * w,
            cy: pin.y * h,
            cos,
            sin,
            rx: pin.radius_x * w,
            ry: pin.radius_y * w,
            exponent: iris_exponent(pin.squareness),
            feather: pin.feather,
            blur: pin.blur,
        }
    }

    /// 0 in the sharp core, 1 outside the outline, a smoothstep across the
    /// fade band between them.
    fn falloff(&self, x: f64, y: f64) -> f64 {
        let (dx, dy) = (x - self.cx, y - self.cy);
        let u = ((dx * self.cos + dy * self.sin) / self.rx).abs();
        let v = ((dy * self.cos - dx * self.sin) / self.ry).abs();
        let p = self.exponent;
        let rho = (u.powf(p) + v.powf(p)).powf(1.0 / p);
        if rho <= self.feather {
            return 0.0;
        }
        if rho >= 1.0 {
            return 1.0;
        }
        let t = (rho - self.feather) / (1.0 - self.feather);
        t * t * (3.0 - 2.0 * t)
    }
}

fn validate_pins(pins: &[IrisPin]) -> Result<(), FilterError> {
    if pins.len() > MAX_FIELD_PINS {
        return Err(FilterError::InvalidParams(format!(
            "iris blur takes at most {MAX_FIELD_PINS} pins, got {}",
            pins.len()
        )));
    }
    for pin in pins {
        let values = [
            pin.x,
            pin.y,
            pin.radius_x,
            pin.radius_y,
            pin.angle,
            pin.squareness,
            pin.feather,
            pin.blur,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(FilterError::InvalidParams(
                "iris blur pin values must be finite".into(),
            ));
        }
        for (name, value) in [("x radius", pin.radius_x), ("y radius", pin.radius_y)] {
            if !(MIN_IRIS_RADIUS..=MAX_IRIS_RADIUS).contains(&value) {
                return Err(FilterError::InvalidParams(format!(
                    "iris {name} {value} is outside {MIN_IRIS_RADIUS}..={MAX_IRIS_RADIUS}"
                )));
            }
        }
        if !(0.0..=100.0).contains(&pin.squareness) {
            return Err(FilterError::InvalidParams(format!(
                "iris squareness {} is outside 0..=100",
                pin.squareness
            )));
        }
        if !(0.0..=1.0).contains(&pin.feather) {
            return Err(FilterError::InvalidParams(format!(
                "iris feather {} is outside 0..=1",
                pin.feather
            )));
        }
        if !(0.0..=MAX_FIELD_RADIUS as f64).contains(&pin.blur) {
            return Err(FilterError::InvalidParams(format!(
                "iris blur amount {} is outside 0..={MAX_FIELD_RADIUS}",
                pin.blur
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
