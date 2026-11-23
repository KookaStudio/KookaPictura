//! Colour for Panasonic RW2 files (`codec/camera-matrices`, #1542): the table's
//! Panasonic rows on synthetic RW2s, and, opt-in, real camera files.

use pictura_codec::{
    as_shot_transform, decode_raw_source, read_raw, DngColor, ImageBudget, PsdError,
};
use pictura_color::chroma::{self, Mat3};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/rw2_builder.rs"]
mod rw2_builder;
#[allow(dead_code)]
#[path = "raw/rw2_encode.rs"]
mod rw2_encode;

use builder::noise_plane;
use rw2_builder::{rw2, Data, Rw2};

/// Every Panasonic body the table colours, with the EXIF `LightSource` codes of
/// its two matrices: CIE A and D65 for the spectral fit, the tungsten and
/// daylight presets for a camera-JPEG fit.
const BODIES: [(&str, [u16; 2]); 10] = [
    ("DC-GX9", [17, 21]),
    ("DC-GH5S", [20, 3]),
    ("DC-GH6", [20, 3]),
    ("DC-S1", [20, 3]),
    ("DC-S5", [20, 3]),
    ("DMC-FZ28", [20, 3]),
    ("DMC-G1", [20, 3]),
    ("DMC-GM1S", [20, 3]),
    ("DMC-ZS100", [20, 3]),
    ("DMC-FZ18", [20, 3]),
];

/// Market names of one camera, with the body whose row they share.
const ALIASES: [(&str, &str); 4] = [
    ("DMC-GM1", "DMC-GM1S"),
    ("DMC-TZ100", "DMC-ZS100"),
    ("DMC-TZ101", "DMC-ZS100"),
    ("DMC-TZ110", "DMC-ZS100"),
];

fn file(model: &'static str) -> Vec<u8> {
    let samples = noise_plane(90, 12, 4095, 23).samples;
    let mut r = Rw2::new(90, 12, samples, Data::Ten);
    r.model = model;
    rw2(&r)
}

fn color(model: &'static str) -> DngColor {
    read_raw(&file(model), &ImageBudget::default())
        .unwrap_or_else(|e| panic!("{model}: {e}"))
        .color
}

fn matrix(v: [f64; 9]) -> Mat3 {
    std::array::from_fn(|r| [v[3 * r], v[3 * r + 1], v[3 * r + 2]])
}

/// The nominal colour temperature the colour stage gives each code.
fn kelvin(code: u16) -> f64 {
    match code {
        1 => 5500.0,
        3 => 2850.0,
        17 => 2856.0,
        20 => 5503.0,
        21 => 6504.0,
        other => panic!("unexpected illuminant {other}"),
    }
}

// rw2_camera_matrices
#[test]
fn panasonic_bodies_take_their_matrices() {
    for (model, illuminants) in BODIES {
        let c = color(model);
        assert!(c.color_matrix.iter().all(Option::is_some), "{model}");
        assert_eq!(c.calibration_illuminant, illuminants.map(Some), "{model}");
    }
    for (alias, body) in ALIASES {
        assert_eq!(
            color(alias).color_matrix,
            color(body).color_matrix,
            "{alias}"
        );
    }
    assert_ne!(color("DMC-GM1S").color_matrix, color("DMC-G1").color_matrix);
    assert_eq!(color("DMC-FZ1").color_matrix, [None, None]);
}

// rw2_matrices_white
#[test]
fn panasonic_matrices_keep_white_neutral() {
    for (model, _) in BODIES {
        let c = color(model);
        for (cm, code) in c.color_matrix.into_iter().zip(c.calibration_illuminant) {
            let (cm, code) = (cm.unwrap(), code.unwrap());
            let want = kelvin(code);
            let white = chroma::temperature_tint_to_xy(want, 0.0);
            let neutral = chroma::mul_vec(&matrix(cm), chroma::xy_to_xyz(white));
            assert!(neutral.iter().all(|&v| v > 0.0), "{model}: {neutral:?}");
            let shot = DngColor {
                as_shot_neutral: Some(neutral),
                ..c.clone()
            };
            let t = as_shot_transform(&shot)
                .unwrap_or_else(|e| panic!("{model}: {e}"))
                .white_balance
                .temperature;
            assert!(
                (t - want).abs() <= 0.02 * want,
                "{model}: {t} K for {want} K"
            );
        }
    }
}

// rw2_known_camera_decodes
#[test]
fn known_panasonic_rw2_decodes() {
    let bytes = file("DMC-GM1S");
    let image = read_raw(&bytes, &ImageBudget::default()).unwrap();
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("GM1S decodes");
    let want = as_shot_transform(&image.color).unwrap().white_balance;
    assert_eq!(source.as_shot, want);
    assert!((2000.0..=12000.0).contains(&want.temperature), "{want:?}");
}

// rw2_no_colour
#[test]
fn unknown_panasonic_body_is_refused_by_name() {
    match decode_raw_source(&file("DMC-FZ1"), &ImageBudget::default()) {
        Err(PsdError::Unsupported(why)) => assert!(
            why.contains("camera colour for Panasonic DMC-FZ1 is not supported yet"),
            "{why}"
        ),
        other => panic!("{:?}", other.err()),
    }
}

// rw2_colour_corpus
#[test]
fn rw2_corpus_colour_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP rw2 colour corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let bytes = std::fs::read(&path).expect("corpus file");
        if !bytes.starts_with(b"IIU\0") {
            continue;
        }
        let name = path.display();
        let Ok(image) = read_raw(&bytes, &ImageBudget::default()) else {
            continue;
        };
        let known = image.color.color_matrix.iter().any(Option::is_some);
        match decode_raw_source(&bytes, &ImageBudget::default()) {
            Ok(source) if known => {
                let t = source.as_shot.temperature;
                eprintln!("{name}: {:?} as shot {t:.0} K", image.model);
                if !(2000.0..=12000.0).contains(&t) {
                    failures.push(format!("{name}: {t} K"));
                }
            }
            Err(PsdError::Unsupported(why)) if !known && why.contains("camera colour") => {}
            other => failures.push(format!("{name}: known {known}, {:?}", other.err())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
