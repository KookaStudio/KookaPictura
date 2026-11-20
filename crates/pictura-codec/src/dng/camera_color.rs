//! The camera colour stage (#398): demosaiced camera RGB to white-balanced
//! linear ProPhoto through the DNG's own colour tags.
//!
//! Follows the DNG 1.6 specification's camera-to-XYZ mapping; no Adobe camera
//! profile (DCP) is read, so parity with Adobe Camera Raw is approximate.

use pictura_color::chroma::{self, Mat3, D50_XY, IDENTITY};
use rayon::prelude::*;

use crate::error::PsdError;

use super::{CameraRgb, DngColor};

/// A white balance as a colour temperature in kelvin and a tint (positive
/// toward magenta).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    pub temperature: f64,
    pub tint: f64,
}

/// The white balance and matrix that take camera RGB to linear ProPhoto.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraTransform {
    pub white_balance: WhiteBalance,
    /// Per-channel gains on camera RGB; the smallest is 1.
    pub gains: [f64; 3],
    /// White-balanced camera RGB → linear ProPhoto; `matrix · (1,1,1)` is
    /// `(1,1,1)`.
    pub matrix: Mat3,
}

/// Linear ProPhoto RGB (ROMM primaries, D50), 0–65535.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProPhotoRgb {
    pub width: usize,
    pub height: usize,
    /// `width × height × 3`, row-major, interleaved red, green, blue.
    pub rgb: Vec<u16>,
}

fn unsupported(what: &str) -> PsdError {
    PsdError::Unsupported(format!("DNG colour: {what}"))
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("DNG colour: {what}"))
}

/// Nominal colour temperature of an EXIF `LightSource` code.
fn illuminant_temperature(code: u16) -> Option<f64> {
    Some(match code {
        17 => 2856.0,
        18 => 4874.0,
        19 => 6774.0,
        20 => 5503.0,
        21 => 6504.0,
        22 => 7504.0,
        23 => 5003.0,
        24 => 3200.0,
        1 | 4 | 9 => 5500.0,
        2 | 14 => 4150.0,
        3 => 2850.0,
        10 => 6500.0,
        11 => 7500.0,
        12 => 6430.0,
        13 => 5000.0,
        15 => 3450.0,
        16 => 2940.0,
        _ => return None,
    })
}

/// The tags interpolated at one temperature.
struct Interpolator<'a> {
    color: &'a DngColor,
    /// `(low, high)` temperatures with the tag index of each, when both
    /// matrices exist under known, different illuminants.
    pair: Option<((f64, usize), (f64, usize))>,
    analog: Mat3,
}

impl<'a> Interpolator<'a> {
    fn new(color: &'a DngColor) -> Result<Self, PsdError> {
        if color.color_matrix.iter().all(Option::is_none) {
            return Err(unsupported("no ColorMatrix1 or ColorMatrix2"));
        }
        let temps = color
            .calibration_illuminant
            .map(|c| c.and_then(illuminant_temperature));
        let pair = match (temps, &color.color_matrix) {
            ([Some(a), Some(b)], [Some(_), Some(_)]) if a != b => Some(if a < b {
                ((a, 0), (b, 1))
            } else {
                ((b, 1), (a, 0))
            }),
            _ => None,
        };
        let analog = match color.analog_balance {
            Some(ab) if ab.iter().all(|&v| v > 0.0) => chroma::diag(ab),
            Some(_) => return Err(invalid("AnalogBalance")),
            None => IDENTITY,
        };
        Ok(Self {
            color,
            pair,
            analog,
        })
    }

    /// One of a tag pair at `t`: blended when both exist, else whichever does.
    fn at(&self, tags: &[Option<[f64; 9]>; 2], t: f64) -> Option<Mat3> {
        let m = |i: usize| {
            tags[i].map(|v| -> Mat3 {
                std::array::from_fn(|r| [v[3 * r], v[3 * r + 1], v[3 * r + 2]])
            })
        };
        match (self.pair, m(0), m(1)) {
            (Some(((lo, il), (hi, ih))), Some(_), Some(_)) => {
                let g = ((1.0 / t - 1.0 / hi) / (1.0 / lo - 1.0 / hi)).clamp(0.0, 1.0);
                Some(chroma::lerp(&m(il)?, &m(ih)?, g))
            }
            (_, a, b) => a.or(b),
        }
    }

    /// `AB · CC`, the reference-to-individual-camera matrix at `t`.
    fn ab_cc(&self, t: f64) -> Mat3 {
        let cc = self
            .at(&self.color.camera_calibration, t)
            .unwrap_or(IDENTITY);
        chroma::mul(&self.analog, &cc)
    }

    fn xyz_to_camera(&self, t: f64) -> Mat3 {
        let cm = self
            .at(&self.color.color_matrix, t)
            .expect("checked in new");
        chroma::mul(&self.ab_cc(t), &cm)
    }

    fn camera_to_xyz(&self, t: f64) -> Result<Mat3, PsdError> {
        chroma::invert(&self.xyz_to_camera(t)).ok_or_else(|| unsupported("singular colour matrix"))
    }

    /// The white balance whose camera response is `neutral`.
    fn white_of(&self, neutral: [f64; 3]) -> Result<WhiteBalance, PsdError> {
        let mut t = 5003.0;
        let mut tint = 0.0;
        for _ in 0..30 {
            let xyz = chroma::mul_vec(&self.camera_to_xyz(t)?, neutral);
            if xyz.iter().sum::<f64>() <= 0.0 || xyz[1] <= 0.0 {
                return Err(invalid("AsShotNeutral is outside the camera's gamut"));
            }
            let (next, next_tint) = chroma::xy_to_temperature_tint(chroma::xyz_to_xy(xyz));
            let settled = (1e6 / next - 1e6 / t).abs() < 0.01;
            (t, tint) = (next, next_tint);
            if settled || self.pair.is_none() {
                break;
            }
        }
        Ok(WhiteBalance {
            temperature: t,
            tint,
        })
    }

    fn transform(&self, wb: WhiteBalance, neutral: [f64; 3]) -> Result<CameraTransform, PsdError> {
        let max = neutral.iter().fold(0.0f64, |a, &v| a.max(v));
        let n = neutral.map(|v| v / max);
        let t = wb.temperature;
        let ab_cc_inv = chroma::invert(&self.ab_cc(t))
            .ok_or_else(|| unsupported("singular CameraCalibration"))?;
        let camera_to_xyz = match self.at(&self.color.forward_matrix, t) {
            Some(fm) => {
                let reference = chroma::mul_vec(&ab_cc_inv, n);
                if reference.iter().any(|&v| v <= 0.0) {
                    return Err(invalid("reference neutral is not positive"));
                }
                chroma::mul(
                    &fm,
                    &chroma::mul(&chroma::diag(reference.map(|v| 1.0 / v)), &ab_cc_inv),
                )
            }
            None => {
                let native = self.camera_to_xyz(t)?;
                let white = chroma::xyz_to_xy(chroma::mul_vec(&native, n));
                chroma::mul(&chroma::bradford(white, D50_XY), &native)
            }
        };
        let x = chroma::mul(&camera_to_xyz, &chroma::diag(n));
        let y = chroma::mul_vec(&x, [1.0; 3])[1];
        if !(y > 0.0 && y.is_finite()) {
            return Err(unsupported(
                "colour matrix maps the neutral to no luminance",
            ));
        }
        Ok(CameraTransform {
            white_balance: wb,
            gains: n.map(|v| 1.0 / v),
            matrix: chroma::scale(&chroma::mul(&chroma::xyz_d50_to_pro_photo(), &x), 1.0 / y),
        })
    }

    fn neutral_at(&self, wb: WhiteBalance) -> Result<[f64; 3], PsdError> {
        let xy = chroma::temperature_tint_to_xy(wb.temperature, wb.tint);
        let n = chroma::mul_vec(&self.xyz_to_camera(wb.temperature), chroma::xy_to_xyz(xy));
        if n.iter().any(|&v| v <= 0.0 || !v.is_finite()) {
            return Err(invalid("white balance outside the camera's gamut"));
        }
        Ok(n)
    }
}

/// The transform at the white balance the camera recorded (`AsShotNeutral`),
/// or D50 when it recorded none.
pub fn as_shot_transform(color: &DngColor) -> Result<CameraTransform, PsdError> {
    let interp = Interpolator::new(color)?;
    match color.as_shot_neutral {
        Some(n) if n.iter().all(|&v| v > 0.0 && v.is_finite()) => {
            interp.transform(interp.white_of(n)?, n)
        }
        Some(_) => Err(invalid("AsShotNeutral is not positive")),
        None => {
            let (temperature, tint) = chroma::xy_to_temperature_tint(D50_XY);
            white_balance_transform(color, WhiteBalance { temperature, tint })
        }
    }
}

/// The transform at a chosen colour temperature and tint.
pub fn white_balance_transform(
    color: &DngColor,
    white_balance: WhiteBalance,
) -> Result<CameraTransform, PsdError> {
    let interp = Interpolator::new(color)?;
    interp.transform(white_balance, interp.neutral_at(white_balance)?)
}

/// Applies `transform` to `image` in place: white balance, highlights clipped
/// to neutral, then the matrix.
pub fn to_pro_photo(image: CameraRgb, transform: &CameraTransform) -> ProPhotoRgb {
    let CameraRgb {
        width,
        height,
        mut rgb,
    } = image;
    let gains = transform.gains.map(|g| (g / 65535.0) as f32);
    let m = transform.matrix.map(|row| row.map(|v| v as f32));
    let row_len = (width * 3).max(1);
    rgb.par_chunks_mut(row_len).for_each(|row| {
        for px in row.as_chunks_mut::<3>().0 {
            // ponytail: clipped highlights go neutral; no highlight recovery.
            let w: [f32; 3] = std::array::from_fn(|i| (px[i] as f32 * gains[i]).min(1.0));
            for (out, r) in px.iter_mut().zip(&m) {
                // ponytail: colours outside ProPhoto clip per channel.
                let v = r[0] * w[0] + r[1] * w[1] + r[2] * w[2];
                *out = (v.clamp(0.0, 1.0) * 65535.0).round() as u16;
            }
        }
    });
    ProPhotoRgb { width, height, rgb }
}
