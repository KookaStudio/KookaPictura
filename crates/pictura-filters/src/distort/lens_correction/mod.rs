//! Filter ▸ Lens Correction (#347, `FILT-101`). Adobe's lens models and
//! profile format are closed; this inverts a Brown–Conrady radial model,
//! scales each colour channel radially for lateral chromatic aberration,
//! applies a radial vignette gain, and projects through a virtual camera
//! tilt for the perspective sliders, all in one resample.
//! *(approximation: behavioural parity only)*
//!
//! ponytail: profiles carry one model each (no focal-length, f-stop, or
//! focus-distance sub-profiles), and the bundled set is a synthetic test
//! profile; no GPU path.

mod map;
mod profile;
mod render;
#[cfg(test)]
mod tests;

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

pub use map::LensMap;
pub use profile::{builtin_profiles, match_profiles, parse_profiles, LensProfile};
pub use render::LensSource;

/// How blank areas left by the correction are filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LensEdge {
    /// Repeat the nearest edge pixel.
    Extension,
    /// Clear, or white on a layer without an alpha plane.
    Transparency,
    Black,
    White,
}

/// A profile's coefficients, already gated by the Auto tab's checkboxes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LensModel {
    /// Brown–Conrady `k1 k2 k3` on the radius over the half-diagonal:
    /// an undistorted point at radius `r` shows source radius
    /// `r · (1 + k1 r² + k2 r⁴ + k3 r⁶)`.
    pub distortion: [f64; 3],
    /// Radial scale of the red, green, and blue samples (1.0 = none).
    pub ca_scale: [f64; 3],
    /// The lens falloff `1 + v1 r² + v2 r⁴ + v3 r⁶`, which the correction
    /// divides out.
    pub vignette: [f64; 3],
}

impl LensModel {
    pub const NONE: LensModel = LensModel {
        distortion: [0.0; 3],
        ca_scale: [1.0; 3],
        vignette: [0.0; 3],
    };
}

/// The dialog's settings. Slider values use the dialog's own ranges.
#[derive(Debug, Clone, PartialEq)]
pub struct LensCorrectionParams {
    pub profile: Option<LensModel>,
    /// Remove Distortion, −100..+100; positive removes barrel distortion.
    pub distortion: f64,
    /// Fix Red/Cyan, Green/Magenta, and Blue/Yellow Fringe, −100..+100.
    pub fringe: [f64; 3],
    /// −100 darkens the corners, +100 lightens them.
    pub vignette_amount: f64,
    /// 0..100; lower widens the affected area.
    pub vignette_midpoint: f64,
    /// Vertical Perspective, −100..+100; positive narrows the top.
    pub vertical: f64,
    /// Horizontal Perspective, −100..+100; positive narrows the right side.
    pub horizontal: f64,
    /// Clockwise rotation in degrees.
    pub angle: f64,
    /// A fraction about the centre (1.0 = 100 %).
    pub scale: f64,
    /// Zoom so the profile's distortion leaves no blank area.
    pub auto_scale: bool,
    pub edge: LensEdge,
}

impl Default for LensCorrectionParams {
    fn default() -> Self {
        Self {
            profile: None,
            distortion: 0.0,
            fringe: [0.0; 3],
            vignette_amount: 0.0,
            vignette_midpoint: 50.0,
            vertical: 0.0,
            horizontal: 0.0,
            angle: 0.0,
            scale: 1.0,
            auto_scale: true,
            edge: LensEdge::Transparency,
        }
    }
}

impl LensCorrectionParams {
    /// Whether these settings leave every pixel as it is.
    pub fn is_identity(&self) -> bool {
        self.profile.is_none_or(|m| m == LensModel::NONE)
            && self.distortion == 0.0
            && self.fringe == [0.0; 3]
            && self.vignette_amount == 0.0
            && self.vertical == 0.0
            && self.horizontal == 0.0
            && self.angle.rem_euclid(360.0) == 0.0
            && self.scale == 1.0
    }
}

/// How far the line `a → b` (layer pixels, y down) leans from the nearest
/// horizontal or vertical, in degrees clockwise within (−45, 45]. Rotating
/// by its negative makes the line level or plumb.
pub fn straighten_angle(a: [f64; 2], b: [f64; 2]) -> f64 {
    let deg = (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees();
    let dev = deg.rem_euclid(90.0);
    if dev > 45.0 {
        dev - 90.0
    } else {
        dev
    }
}

/// Apply `params` to `buf` (three or four planes).
pub fn lens_correction(
    buf: &mut PixelBuffer,
    params: &LensCorrectionParams,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !params.scale.is_finite() || params.scale <= 0.0 {
        return Err(FilterError::InvalidParams(
            "Lens Correction scale must be positive".into(),
        ));
    }
    if params.is_identity() {
        return Ok(());
    }
    let map = LensMap::new(params, buf.width, buf.height);
    let source = LensSource::from_buffer(buf);
    render::commit(&map, &source, buf);
    Ok(())
}
