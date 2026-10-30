//! Scalar colour-model conversions shared by every colour readout: sRGB to and
//! from CIELAB in the D50 ICC connection space
//! (`docs/07-color-painting/color-models.md`).

/// D50 reference white (ICC PCS), `Y = 1`.
const XN: f64 = 0.96422;
const ZN: f64 = 0.82521;

/// Linear sRGB → XYZ D50 (Bradford D65→D50).
const LINEAR_SRGB_TO_XYZ_D50: [[f64; 3]; 3] = [
    [0.4360747, 0.3850649, 0.1430804],
    [0.2225045, 0.7168786, 0.0606169],
    [0.0139322, 0.0971045, 0.7141733],
];

/// XYZ D50 → linear sRGB (Bradford D50→D65), the inverse of
/// [`LINEAR_SRGB_TO_XYZ_D50`].
const XYZ_D50_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
    [3.1338561, -1.6168667, -0.4906146],
    [-0.9787684, 1.9161415, 0.0334540],
    [0.0719453, -0.2289914, 1.4052427],
];

const EPSILON: f64 = 216.0 / 24389.0;
const KAPPA: f64 = 24389.0 / 27.0;

/// 8-bit sRGB → CIELAB D50 as `[L, a, b]` (L in 0–100, a/b unbounded before
/// the caller clamps to −128…127). ponytail: the standard profile-free
/// CIELAB(D50) ↔ sRGB(D65) pair; it matches lcms2's exact transform within one
/// unit, not Adobe's closed CMS.
pub fn srgb_to_lab(rgb: [u8; 3]) -> [f64; 3] {
    let linear = rgb.map(srgb_decode);
    let xyz = LINEAR_SRGB_TO_XYZ_D50.map(|row| dot(row, linear));
    let fx = lab_fwd(xyz[0] / XN);
    let fy = lab_fwd(xyz[1]);
    let fz = lab_fwd(xyz[2] / ZN);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIELAB D50 `[L, a, b]` → 8-bit sRGB, the inverse of [`srgb_to_lab`];
/// out-of-gamut channels clamp to 0–255.
pub fn lab_to_srgb(lab: [f64; 3]) -> [u8; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    xyz_d50_to_srgb([XN * lab_f(fx), lab_f(fy), ZN * lab_f(fz)])
}

/// CIE XYZ (D50) → 8-bit sRGB through the same Bradford matrix as
/// [`lab_to_srgb`]; out-of-gamut channels clamp.
pub fn xyz_d50_to_srgb(xyz: [f64; 3]) -> [u8; 3] {
    XYZ_D50_TO_LINEAR_SRGB.map(|row| srgb_encode(dot(row, xyz)))
}

fn dot(row: [f64; 3], v: [f64; 3]) -> f64 {
    row[0] * v[0] + row[1] * v[1] + row[2] * v[2]
}

fn lab_f(t: f64) -> f64 {
    let t3 = t * t * t;
    if t3 > EPSILON {
        t3
    } else {
        (116.0 * t - 16.0) / KAPPA
    }
}

/// The forward CIELAB `f` function: the exact inverse of [`lab_f`].
fn lab_fwd(t: f64) -> f64 {
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

fn srgb_encode(linear: f64) -> u8 {
    let value = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8
}

fn srgb_decode(value: u8) -> f64 {
    let value = f64::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Intent, Profile};
    use lcms2::{CIExyY, GlobalContext, PixelFormat, Profile as LcmsProfile, Transform};

    /// lcms2's own sRGB → Lab (D50) for `rgb`, the connection-space reference.
    fn lcms_lab(rgb: [u8; 3]) -> [f64; 3] {
        let d50 = CIExyY {
            x: 0.3457,
            y: 0.3585,
            Y: 1.0,
        };
        let lab = LcmsProfile::new_lab4_context(GlobalContext::new(), &d50).unwrap();
        let t: Transform<[f64; 3], [f64; 3]> = Transform::new(
            &Profile::srgb().0,
            PixelFormat::RGB_DBL,
            &lab,
            PixelFormat::Lab_DBL,
            Intent::RelativeColorimetric.to_lcms(),
        )
        .unwrap();
        let mut out = [[0.0; 3]];
        t.transform_pixels(&[rgb.map(|v| f64::from(v) / 255.0)], &mut out);
        out[0]
    }

    fn sweep() -> impl Iterator<Item = [u8; 3]> {
        (0..=255u16).step_by(15).flat_map(|r| {
            (0..=255u16).step_by(15).flat_map(move |g| {
                (0..=255u16)
                    .step_by(15)
                    .map(move |b| [r as u8, g as u8, b as u8])
            })
        })
    }

    #[test]
    fn lab_white_is_neutral() {
        let [l, a, b] = srgb_to_lab([255, 255, 255]);
        assert_eq!((l.round(), a.round(), b.round()), (100.0, 0.0, 0.0));
        assert_eq!(srgb_to_lab([0, 0, 0]).map(f64::round), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn lab_matches_lcms_connection_space() {
        let [l, a, b] = srgb_to_lab([255, 0, 0]);
        assert!((l - 54.3).abs() < 1.0 && (a - 80.8).abs() < 1.0 && (b - 69.9).abs() < 1.0);
        for rgb in sweep().chain([[255, 0, 0], [0, 255, 0], [0, 0, 255], [128, 128, 128]]) {
            let ours = srgb_to_lab(rgb);
            let theirs = lcms_lab(rgb);
            for c in 0..3 {
                assert!(
                    (ours[c] - theirs[c]).abs() <= 1.0,
                    "{rgb:?} channel {c}: ours {ours:?} lcms2 {theirs:?}"
                );
            }
        }
    }

    #[test]
    fn lab_round_trips_srgb_grid() {
        for rgb in sweep() {
            let back = lab_to_srgb(srgb_to_lab(rgb));
            for c in 0..3 {
                assert!(
                    back[c].abs_diff(rgb[c]) <= 1,
                    "{rgb:?} came back as {back:?}"
                );
            }
        }
    }

    #[test]
    fn lab_out_of_gamut_clamps() {
        assert_eq!(lab_to_srgb([50.0, 127.0, -128.0])[1], 0);
        assert_eq!(lab_to_srgb([100.0, 0.0, 0.0]), [255, 255, 255]);
        assert_eq!(lab_to_srgb([120.0, 0.0, 0.0]), [255, 255, 255]);
    }
}
