//! The Blur Gallery with several effects on at once (`FILT-092`): Field Blur,
//! Iris Blur, and Tilt-Shift pins combined in one variable-blur pass, then
//! Tilt-Shift's distortion smear, Path Blur's gather over that result, Spin
//! Blur's over that, and the shared Blur Effects / Noise passes.
//!
//! ponytail: Adobe's combine is closed. Each enabled effect builds its own
//! radius field exactly as it does alone, and a pixel takes the largest of
//! them (inferred), so every effect's sharp area stays sharp only where no
//! other effect blurs it. Paths blur what the radius effects left and spins
//! what the paths left (inferred order); Bokeh stays gated by the radius
//! field and Noise by the largest of the radius, path, and spin weights. A
//! single effect therefore matches its own kernel.

use pictura_core::PixelBuffer;

use super::field::{
    apply_effects, field_radius, pyramid_blur, validate_bokeh, validate_noise, BlurNoise, BlurPin,
    BokehParams,
};
use super::iris::{IrisField, IrisPin};
use super::path::{path_pass, validate_motion, validate_paths, BlurPath, PathMotion, BOKEH_OFF};
use super::spin::{spin_pass, validate_spins, validate_strobe, SpinPin, SpinStrobe};
use super::tilt::{smear, TiltField, TiltPin};
use crate::{validate, FilterError};

/// Field, Iris, Tilt-Shift, Path, and Spin Blur blurred together; an empty
/// list turns that effect off, and all five empty is a no-op.
#[allow(clippy::too_many_arguments)]
pub fn blur_gallery(
    buf: &mut PixelBuffer,
    field: &[BlurPin],
    iris: &[IrisPin],
    tilt: &[TiltPin],
    path: &[BlurPath],
    motion: &PathMotion,
    spin: &[SpinPin],
    spin_strobe: &SpinStrobe,
    bokeh: &BokehParams,
    noise: &BlurNoise,
) -> Result<(), FilterError> {
    validate(buf)?;
    super::field::validate_pins(field)?;
    super::iris::validate_pins(iris)?;
    super::tilt::validate_pins(tilt)?;
    validate_paths(path)?;
    validate_motion(motion)?;
    validate_spins(spin)?;
    validate_strobe(spin_strobe)?;
    validate_bokeh(bokeh)?;
    validate_noise(noise)?;
    let amounts = field
        .iter()
        .map(|pin| pin.blur)
        .chain(iris.iter().map(|pin| pin.blur))
        .chain(tilt.iter().map(|pin| pin.blur));
    let max_pin = amounts.fold(0.0f64, f64::max);
    let radius_on = !(field.is_empty() && iris.is_empty() && tilt.is_empty());
    let radius_weights = if radius_on {
        radius_pass(buf, field, iris, tilt, max_pin)
    } else {
        None
    };
    let path_weights = path_pass(buf, path, motion);
    let spin_weights = spin_pass(buf, spin, spin_strobe);
    let motion_weights = match (path_weights, spin_weights) {
        (Some(path), Some(spin)) => Some(larger(&path, &spin)),
        (path, spin) => path.or(spin),
    };
    match (radius_weights, motion_weights) {
        (None, None) => {}
        (Some(weights), None) => apply_effects(buf, &weights, bokeh, noise),
        (None, Some(weights)) => apply_effects(buf, &weights, &BOKEH_OFF, noise),
        (Some(radius), Some(motion)) => {
            let noise_off = BlurNoise {
                enabled: false,
                ..*noise
            };
            apply_effects(buf, &radius, bokeh, &noise_off);
            apply_effects(buf, &larger(&radius, &motion), &BOKEH_OFF, noise);
        }
    }
    Ok(())
}

fn larger(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(&a, &b)| a.max(b)).collect()
}

/// The radius effects' pyramid and Tilt-Shift's smear; their weights, or
/// `None` (buffer untouched) when no pin blurs.
fn radius_pass(
    buf: &mut PixelBuffer,
    field: &[BlurPin],
    iris: &[IrisPin],
    tilt: &[TiltPin],
    max_pin: f64,
) -> Option<Vec<u8>> {
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
    let weights = pyramid_blur(buf, max_pin, radius)?;
    if let Some(tilt) = tilt_field.as_ref().filter(|tilt| tilt.distorts()) {
        smear(buf, tilt);
    }
    Some(weights)
}

#[cfg(test)]
mod tests;
