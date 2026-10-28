//! Filter ▸ Lens Correction (#347, `FILT-101`). Adobe's lens models and
//! profile format are closed; this inverts a Brown–Conrady radial model,
//! scales each colour channel radially for lateral chromatic aberration,
//! applies a radial vignette gain, and projects through a virtual camera
//! tilt for the perspective sliders, all in one resample.
//! *(approximation: behavioural parity only)*
//!
//! Real profiles come from the system's lensfun database at runtime
//! (`lensfun.rs`, #932); the bundled set is one synthetic test profile.
//! ponytail: one sub-profile at the nearest calibration, no interpolation
//! between them; no GPU path.

mod catalog;
mod lensfun;
mod map;
mod profile;
#[cfg(test)]
mod profile_tests;
mod render;
#[cfg(test)]
mod tests;

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

pub use catalog::{
    camera_models, cameras, find_camera, find_lens, find_make, lenses, lenses_for, makes,
    parse_lens_name, CameraSpec, LensSpec,
};
pub use lensfun::parse_lensfun;
pub use map::LensMap;
pub use profile::{
    builtin_profiles, match_profiles, parse_profiles, pick_subprofile, LensProfile, SubProfile,
};
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

/// A profile's coefficients, already gated by the Auto tab's checkboxes,
/// on the radius `r` over the layer's half-diagonal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LensModel {
    /// An undistorted point at radius `r` shows source radius
    /// `r · (1 + c1 r + c2 r² + … + c6 r⁶)`, as `[c1 … c6]`.
    pub distortion: [f64; 6],
    /// The red, green, and blue samples' radial scale `v + c r + b r²`, as
    /// `[v, c, b]` (`[1, 0, 0]` = none).
    pub ca: [[f64; 3]; 3],
    /// The lens falloff `1 + v1 r² + v2 r⁴ + v3 r⁶`, which the correction
    /// divides out.
    pub vignette: [f64; 3],
}

impl LensModel {
    pub const NONE: LensModel = LensModel {
        distortion: [0.0; 6],
        ca: [[1.0, 0.0, 0.0]; 3],
        vignette: [0.0; 3],
    };

    /// The slot count of [`Self::to_slots`].
    pub const SLOTS: usize = 18;

    /// Brown–Conrady `k1 k2 k3` (on `r², r⁴, r⁶`) as distortion terms.
    pub const fn brown(k: [f64; 3]) -> [f64; 6] {
        [0.0, k[0], 0.0, k[1], 0.0, k[2]]
    }

    /// The model for an image whose half-diagonal is `t` times the one the
    /// coefficients were measured on: each `rⁱ` term scales by `tⁱ`.
    pub fn rescaled(&self, t: f64) -> LensModel {
        let mut out = *self;
        for (i, c) in out.distortion.iter_mut().enumerate() {
            *c *= t.powi(i as i32 + 1);
        }
        for ca in &mut out.ca {
            ca[1] *= t;
            ca[2] *= t * t;
        }
        for (i, v) in out.vignette.iter_mut().enumerate() {
            *v *= t.powi(2 * (i as i32 + 1));
        }
        out
    }

    /// Distortion, the three channels' CA, then the vignette.
    pub fn to_slots(&self) -> [f64; Self::SLOTS] {
        let mut out = [0.0; Self::SLOTS];
        out[..6].copy_from_slice(&self.distortion);
        for (c, ca) in self.ca.iter().enumerate() {
            out[6 + 3 * c..9 + 3 * c].copy_from_slice(ca);
        }
        out[15..].copy_from_slice(&self.vignette);
        out
    }

    /// The inverse of [`Self::to_slots`]; `None` for a short slice.
    pub fn from_slots(v: &[f64]) -> Option<Self> {
        let v: &[f64; Self::SLOTS] = v.get(..Self::SLOTS)?.try_into().ok()?;
        let three = |i: usize| [v[i], v[i + 1], v[i + 2]];
        Some(LensModel {
            distortion: [v[0], v[1], v[2], v[3], v[4], v[5]],
            ca: [three(6), three(9), three(12)],
            vignette: three(15),
        })
    }
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
    /// The dialog's slot count: a parameter set round-trips through this many
    /// values, in the order the `lens-correction` kind stores them.
    pub const SLOTS: usize = 36;

    /// The first profile slot after the dialog's 16: the sub-profile index,
    /// whether coefficients follow, then [`LensModel::to_slots`].
    pub const PROFILE_SLOTS: usize = 16;

    /// Build the parameters from the dialog's [`Self::SLOTS`] slots. `None`
    /// for a wrong count or a non-finite value. The profile is the stored
    /// coefficients when the slots carry them; otherwise id 1 is the bundled
    /// profile (a record from before the coefficients were stored) and any
    /// other non-zero id is refused.
    pub fn from_slots(values: &[f64]) -> Option<Self> {
        if values.len() != Self::SLOTS || values.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let on = |i: usize| values[i] >= 0.5;
        let id = values[5].round();
        let stored = if on(Self::PROFILE_SLOTS + 1) {
            LensModel::from_slots(&values[Self::PROFILE_SLOTS + 2..])
        } else if id <= 0.0 {
            None
        } else if id == 1.0 {
            Some(builtin_profiles().first()?.subs.first()?.coefficients)
        } else {
            return None;
        };
        let profile = stored.map(|p| LensModel {
            distortion: if on(0) { p.distortion } else { [0.0; 6] },
            ca: if on(1) { p.ca } else { LensModel::NONE.ca },
            vignette: if on(2) { p.vignette } else { [0.0; 3] },
        });
        let slider = |i: usize| values[i].clamp(-100.0, 100.0);
        Some(Self {
            profile,
            distortion: slider(6),
            fringe: [slider(7), slider(8), slider(9)],
            vignette_amount: slider(10),
            vignette_midpoint: values[11].clamp(0.0, 100.0),
            vertical: slider(12),
            horizontal: slider(13),
            angle: values[14].rem_euclid(360.0),
            scale: values[15].clamp(50.0, 150.0) / 100.0,
            auto_scale: on(3),
            edge: match values[4].round() as i64 {
                0 => LensEdge::Extension,
                2 => LensEdge::Black,
                3 => LensEdge::White,
                _ => LensEdge::Transparency,
            },
        })
    }

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
