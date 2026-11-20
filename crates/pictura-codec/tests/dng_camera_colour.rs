//! The camera colour stage (`codec/camera-colour`, #398) on synthetic colour
//! tags, and, opt-in, on real camera files.

use pictura_codec::{
    as_shot_transform, demosaic, read_dng, to_pro_photo, white_balance_transform, CameraRgb,
    CameraTransform, DemosaicMethod, DngColor, ImageBudget, PsdError, WhiteBalance,
};
use pictura_color::chroma::{self, Mat3, D50_XY, IDENTITY};

const A: u16 = 17;
const D65: u16 = 21;
const D50: u16 = 23;
const A_XY: [f64; 2] = [0.44757, 0.40745];
const D65_XY: [f64; 2] = [0.31271, 0.32902];

fn flat(m: &Mat3) -> [f64; 9] {
    std::array::from_fn(|i| m[i / 3][i % 3])
}

fn color(matrix: &Mat3, illuminant: u16, neutral: Option<[f64; 3]>) -> DngColor {
    DngColor {
        color_matrix: [Some(flat(matrix)), None],
        calibration_illuminant: [Some(illuminant), None],
        as_shot_neutral: neutral,
        ..DngColor::default()
    }
}

fn assert_mat(a: &Mat3, b: &Mat3, tol: f64, what: &str) {
    for (x, y) in a.iter().flatten().zip(b.iter().flatten()) {
        assert!((x - y).abs() <= tol, "{what}: {a:?} vs {b:?}");
    }
}

fn image(pixels: &[[u16; 3]]) -> CameraRgb {
    CameraRgb {
        width: pixels.len(),
        height: 1,
        rgb: pixels.iter().flatten().copied().collect(),
    }
}

/// Two dissimilar, invertible XYZ → camera matrices.
const CM_A: Mat3 = [[0.9, -0.2, -0.1], [-0.4, 1.3, 0.1], [-0.05, 0.15, 0.55]];
const CM_D65: Mat3 = [[0.7, -0.1, -0.05], [-0.35, 1.2, 0.15], [-0.1, 0.25, 0.7]];

fn two_illuminants(neutral: Option<[f64; 3]>) -> DngColor {
    DngColor {
        color_matrix: [Some(flat(&CM_A)), Some(flat(&CM_D65))],
        calibration_illuminant: [Some(A), Some(D65)],
        as_shot_neutral: neutral,
        ..DngColor::default()
    }
}

fn response(cm: &Mat3, xy: [f64; 2]) -> [f64; 3] {
    let n = chroma::mul_vec(cm, chroma::xy_to_xyz(xy));
    let max = n.iter().fold(0.0f64, |a, &v| a.max(v));
    n.map(|v| v / max)
}

#[test]
fn pro_photo_camera_passes_through() {
    let dng = color(&chroma::xyz_d50_to_pro_photo(), D50, Some([1.0; 3]));
    let t = as_shot_transform(&dng).unwrap();
    assert_mat(&t.matrix, &IDENTITY, 1e-6, "matrix");
    let mut s = 0x1234_5678u64;
    let input: Vec<u16> = (0..16 * 12 * 3)
        .map(|_| {
            s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (s >> 48) as u16
        })
        .collect();
    let out = to_pro_photo(
        CameraRgb {
            width: 16,
            height: 12,
            rgb: input.clone(),
        },
        &t,
    );
    assert_eq!((out.width, out.height), (16, 12));
    for (a, b) in input.iter().zip(&out.rgb) {
        assert!(a.abs_diff(*b) <= 1, "{a} vs {b}");
    }
}

#[test]
fn srgb_camera_lands_on_pro_photo() {
    let d65 = [0.3127, 0.3290];
    let srgb_to_xyz = chroma::rgb_to_xyz([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]], d65);
    let dng = color(&chroma::invert(&srgb_to_xyz).unwrap(), D65, Some([1.0; 3]));
    let t = as_shot_transform(&dng).unwrap();
    // Lindbloom's published Bradford-adapted sRGB → XYZ (D50).
    let published = [
        [0.4360747, 0.3850649, 0.1430804],
        [0.2225045, 0.7168786, 0.0606169],
        [0.0139322, 0.0971045, 0.7141733],
    ];
    let oracle = chroma::mul(&chroma::xyz_d50_to_pro_photo(), &published);
    let colours = [
        [0.8, 0.2, 0.1],
        [0.1, 0.7, 0.3],
        [0.2, 0.3, 0.9],
        [0.5, 0.5, 0.5],
        [0.9, 0.9, 0.2],
    ];
    let pixels: Vec<[u16; 3]> = colours
        .iter()
        .map(|c| c.map(|v| (v * 65535.0) as u16))
        .collect();
    let out = to_pro_photo(image(&pixels), &t);
    for (i, c) in colours.iter().enumerate() {
        let want = chroma::mul_vec(&oracle, *c);
        for (ch, want) in want.iter().enumerate() {
            let got = out.rgb[i * 3 + ch] as f64 / 65535.0;
            assert!(
                (got - want).abs() <= 0.002,
                "{c:?} ch {ch}: {got} vs {want}"
            );
        }
    }
}

#[test]
fn camera_neutral_becomes_grey() {
    let n = [0.45, 1.0, 0.7];
    let t = as_shot_transform(&two_illuminants(Some(n))).unwrap();
    let scales = [0.1, 0.5, 1.0];
    let mut pixels: Vec<[u16; 3]> = scales
        .iter()
        .map(|k| n.map(|v| (v * k * 65535.0).round() as u16))
        .collect();
    pixels.push([65535; 3]);
    let out = to_pro_photo(image(&pixels), &t);
    for (i, k) in scales.iter().enumerate() {
        let want = (k * 65535.0) as i32;
        for ch in 0..3 {
            let got = out.rgb[i * 3 + ch] as i32;
            assert!(
                (got - want).abs() <= 2,
                "scale {k} ch {ch}: {got} vs {want}"
            );
        }
    }
    assert_eq!(&out.rgb[9..], &[65535; 3]);
}

#[test]
fn calibration_and_analog_balance_apply() {
    let cm = chroma::xyz_d50_to_pro_photo();
    let cc = chroma::diag([1.1, 1.0, 0.9]);
    let ab = [0.95, 1.0, 1.05];
    let full = chroma::mul(&chroma::mul(&chroma::diag(ab), &cc), &cm);
    let dng = DngColor {
        camera_calibration: [Some(flat(&cc)), None],
        analog_balance: Some(ab),
        ..color(&cm, D50, Some(response(&full, D50_XY)))
    };
    let t = as_shot_transform(&dng).unwrap();
    assert_mat(&t.matrix, &IDENTITY, 1e-6, "matrix");
}

#[test]
fn forward_matrix_is_preferred() {
    let fm = {
        // Rows chosen so FM · (1,1,1) is the D50 white, as DNG requires.
        let d50 = chroma::xy_to_xyz(D50_XY);
        let raw = [[0.6, 0.3, 0.1], [0.25, 0.7, 0.05], [0.02, 0.1, 0.88]];
        let sums = raw.map(|r: [f64; 3]| r.iter().sum::<f64>());
        std::array::from_fn::<[f64; 3], 3, _>(|r| raw[r].map(|v| v * d50[r] / sums[r]))
    };
    let n = [0.5, 1.0, 0.8];
    let dng = DngColor {
        forward_matrix: [Some(flat(&fm)), None],
        ..color(&CM_D65, D65, Some(n))
    };
    let t = as_shot_transform(&dng).unwrap();
    let want = chroma::mul(&chroma::xyz_d50_to_pro_photo(), &fm);
    let y = chroma::mul_vec(&fm, [1.0; 3])[1];
    assert_mat(
        &t.matrix,
        &chroma::scale(&want, 1.0 / y),
        1e-6,
        "forward matrix",
    );
    let without = as_shot_transform(&color(&CM_D65, D65, Some(n))).unwrap();
    assert!(
        (without.matrix[0][0] - t.matrix[0][0]).abs() > 1e-3,
        "paths differ"
    );
}

#[test]
fn illuminants_interpolate_in_inverse_temperature() {
    let t = as_shot_transform(&two_illuminants(Some(response(&CM_D65, D65_XY)))).unwrap();
    assert!(
        (t.white_balance.temperature - 6504.0).abs() <= 15.0,
        "{:?}",
        t.white_balance
    );
    let t = as_shot_transform(&two_illuminants(Some(response(&CM_A, A_XY)))).unwrap();
    assert!(
        (t.white_balance.temperature - 2856.0).abs() <= 10.0,
        "{:?}",
        t.white_balance
    );

    let wb = WhiteBalance {
        temperature: 4000.0,
        tint: 0.0,
    };
    let blended = white_balance_transform(&two_illuminants(None), wb).unwrap();
    let g = (1.0 / 4000.0 - 1.0 / 6504.0) / (1.0 / 2856.0 - 1.0 / 6504.0);
    let single = color(&chroma::lerp(&CM_A, &CM_D65, g), D50, None);
    let reference = white_balance_transform(&single, wb).unwrap();
    assert_mat(&blended.matrix, &reference.matrix, 1e-9, "matrix");
    for (a, b) in blended.gains.iter().zip(reference.gains) {
        assert!((a - b).abs() <= 1e-9 * b, "gains {a} vs {b}");
    }
}

fn assert_transforms_close(a: &CameraTransform, b: &CameraTransform) {
    for (x, y) in a.gains.iter().zip(b.gains) {
        assert!(
            (x - y).abs() <= 1e-4 * y.abs(),
            "gains {:?} vs {:?}",
            a.gains,
            b.gains
        );
    }
    for (x, y) in a.matrix.iter().flatten().zip(b.matrix.iter().flatten()) {
        assert!(
            (x - y).abs() <= 1e-4 * y.abs().max(1.0),
            "matrix {:?} vs {:?}",
            a.matrix,
            b.matrix
        );
    }
}

#[test]
fn white_balance_round_trips() {
    for dng in [
        two_illuminants(Some([0.45, 1.0, 0.7])),
        two_illuminants(Some([0.7, 1.0, 0.45])),
        color(&CM_D65, D65, Some([0.5, 1.0, 0.8])),
    ] {
        let shot = as_shot_transform(&dng).unwrap();
        let again = white_balance_transform(&dng, shot.white_balance).unwrap();
        assert_transforms_close(&again, &shot);
    }
}

#[test]
fn missing_neutral_defaults_to_d50() {
    let t = as_shot_transform(&color(&CM_D65, D65, None)).unwrap();
    assert!(
        (t.white_balance.temperature - 5003.0).abs() <= 15.0,
        "{:?}",
        t.white_balance
    );
}

#[test]
fn unusable_colour_tags_are_refused() {
    let none = DngColor {
        as_shot_neutral: Some([1.0; 3]),
        ..DngColor::default()
    };
    assert!(matches!(
        as_shot_transform(&none),
        Err(PsdError::Unsupported(_))
    ));
    let zero = color(&[[0.0; 3]; 3], D65, Some([1.0; 3]));
    assert!(matches!(
        as_shot_transform(&zero),
        Err(PsdError::Unsupported(_))
    ));
    let bad = color(&CM_D65, D65, Some([0.0, 1.0, 1.0]));
    assert!(matches!(as_shot_transform(&bad), Err(PsdError::Invalid(_))));
}

#[test]
fn leica_samples_convert_when_present() {
    let Some(root) = std::env::var_os("PICTURA_DNG_CORPUS") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    for name in ["L1.dng", "L2.dng"] {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let budget = ImageBudget::default();
        let img = read_dng(&bytes, &budget).unwrap();
        let t = as_shot_transform(&img.color).unwrap();
        let k = t.white_balance.temperature;
        assert!(
            (2000.0..=15000.0).contains(&k),
            "{name}: {:?}",
            t.white_balance
        );
        let rgb = demosaic(&img, DemosaicMethod::Bilinear, &budget).unwrap();
        let out = to_pro_photo(rgb, &t);
        assert_eq!((out.width, out.height), (5216, 3472), "{name}");
        assert!(out.rgb.iter().any(|&v| v > 0), "{name} is all black");
        eprintln!("{name}: {:?}", t.white_balance);
    }
}
