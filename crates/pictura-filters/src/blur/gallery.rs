//! The Blur Gallery with several effects on at once (`FILT-092`): Field Blur,
//! Iris Blur, and Tilt-Shift pins combined in one variable-blur pass, then
//! Tilt-Shift's distortion smear and the shared Blur Effects / Noise passes.
//!
//! ponytail: Adobe's combine is closed. Each enabled effect builds its own
//! radius field exactly as it does alone, and a pixel takes the largest of
//! them (inferred), so every effect's sharp area stays sharp only where no
//! other effect blurs it. A single effect therefore matches its own kernel.

use pictura_core::PixelBuffer;

use super::field::{
    apply_effects, field_radius, pyramid_blur, validate_bokeh, validate_noise, BlurNoise, BlurPin,
    BokehParams,
};
use super::iris::{IrisField, IrisPin};
use super::tilt::{smear, TiltField, TiltPin};
use crate::{validate, FilterError};

/// Field, Iris, and Tilt-Shift pins blurred together; an empty list turns
/// that effect off, and all three empty is a no-op.
pub fn blur_gallery(
    buf: &mut PixelBuffer,
    field: &[BlurPin],
    iris: &[IrisPin],
    tilt: &[TiltPin],
    bokeh: &BokehParams,
    noise: &BlurNoise,
) -> Result<(), FilterError> {
    validate(buf)?;
    super::field::validate_pins(field)?;
    super::iris::validate_pins(iris)?;
    super::tilt::validate_pins(tilt)?;
    validate_bokeh(bokeh)?;
    validate_noise(noise)?;
    let amounts = field
        .iter()
        .map(|pin| pin.blur)
        .chain(iris.iter().map(|pin| pin.blur))
        .chain(tilt.iter().map(|pin| pin.blur));
    let max_pin = amounts.fold(0.0f64, f64::max);
    if field.is_empty() && iris.is_empty() && tilt.is_empty() {
        return Ok(());
    }
    let (w, h) = (f64::from(buf.width), f64::from(buf.height));
    let iris_field = (!iris.is_empty()).then(|| IrisField::new(iris, w, h));
    let tilt_field = (!tilt.is_empty()).then(|| TiltField::new(tilt, w, h));
    let radius = |x: f64, y: f64| {
        let mut radius = 0.0f64;
        if !field.is_empty() {
            radius = radius.max(field_radius(field, w, h, x, y));
        }
        if let Some(iris) = &iris_field {
            radius = radius.max(iris.radius(x, y));
        }
        if let Some(tilt) = &tilt_field {
            radius = radius.max(tilt.sample(x, y).0);
        }
        radius
    };
    let Some(weights) = pyramid_blur(buf, max_pin, radius) else {
        return Ok(());
    };
    if let Some(tilt) = tilt_field.as_ref().filter(|tilt| tilt.distorts()) {
        smear(buf, tilt);
    }
    apply_effects(buf, &weights, bokeh, noise);
    Ok(())
}

#[cfg(test)]
mod tests;
