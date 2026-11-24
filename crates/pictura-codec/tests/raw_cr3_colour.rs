//! Colour for Canon CR3 bodies without published sensitivities
//! (`codec/camera-matrices`, #1574, #1579): the table's camera-JPEG rows and
//! proxies on synthetic CR3s.

use pictura_codec::{
    as_shot_transform, decode_raw_source, read_raw, DngColor, ImageBudget, PsdError,
};
use pictura_color::chroma::{self, Mat3};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "raw/cr3_builder.rs"]
mod cr3_builder;
#[allow(dead_code)]
#[path = "raw/crx_encode.rs"]
mod crx_encode;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use cr3_builder::{cmt1, cr3, ctmd, Cr3, Track};
use crx_encode::{frame, Frame, Grid};

const PAIR: [Option<u16>; 2] = [Some(20), Some(24)];

/// Every Canon body the table colours from its own camera JPEG, with the
/// EXIF `LightSource` codes of its matrices: D55 and ISO studio tungsten.
const BODIES: [(&str, [Option<u16>; 2]); 15] = [
    ("Canon EOS 850D", PAIR),
    ("Canon EOS 90D", PAIR),
    ("Canon EOS M200", PAIR),
    ("Canon EOS M50", PAIR),
    ("Canon EOS M50m2", PAIR),
    ("Canon EOS R100", PAIR),
    ("Canon EOS R50 V", PAIR),
    ("Canon EOS R1", PAIR),
    ("Canon EOS R3", PAIR),
    ("Canon EOS R6 Mark III", PAIR),
    ("Canon EOS R7", PAIR),
    ("Canon EOS-1D X Mark III", PAIR),
    ("Canon PowerShot G5 X Mark II", PAIR),
    ("Canon PowerShot SX70 HS", PAIR),
    ("Canon PowerShot V1", PAIR),
];

/// Regional names and same-sensor proxies, with the body whose row they share.
const SHARED: [(&str, &str); 7] = [
    ("Canon EOS Kiss M", "Canon EOS M50"),
    ("Canon EOS KISS M", "Canon EOS M50"),
    ("Canon EOS Kiss M2", "Canon EOS M50m2"),
    ("Canon EOS Rebel T8i", "Canon EOS 850D"),
    ("Canon EOS Kiss X10i", "Canon EOS 850D"),
    ("Canon EOS M6 Mark II", "Canon EOS 90D"),
    (
        "Canon PowerShot G7 X Mark III",
        "Canon PowerShot G5 X Mark II",
    ),
];

/// A small lossless CR3 naming `model`, with colour data `color` in its
/// CTMD track.
fn file(model: &'static str, color: Option<&[u16]>) -> Vec<u8> {
    let planes: [Grid; 4] = std::array::from_fn(|p| {
        let mut g = Grid::new(32, 24);
        for (i, v) in g.v.iter_mut().enumerate() {
            *v = ((i * 7 + p * 13) % 200) as i32 - 100;
        }
        g
    });
    let (cmp1, sample) = frame(&Frame::lossless((64, 48)), &planes);
    let mut tracks = vec![Track::Raw {
        cmp1,
        samples: vec![sample],
    }];
    tracks.extend(color.map(|c| Track::Ctmd(ctmd(c))));
    cr3(&Cr3 {
        cmt1: Some(cmt1("Canon", model, 1)),
        tracks,
        ..Cr3::default()
    })
}

fn color(model: &'static str) -> DngColor {
    read_raw(&file(model, None), &ImageBudget::default())
        .unwrap_or_else(|e| panic!("{model}: {e}"))
        .color
}

fn matrix(v: [f64; 9]) -> Mat3 {
    std::array::from_fn(|r| [v[3 * r], v[3 * r + 1], v[3 * r + 2]])
}

/// The nominal colour temperature the colour stage gives each code.
fn kelvin(code: u16) -> f64 {
    match code {
        20 => 5503.0,
        24 => 3200.0,
        other => panic!("unexpected illuminant {other}"),
    }
}

// cr3_jpeg_matrices
#[test]
fn canon_jpeg_bodies_take_their_matrices() {
    for (model, illuminants) in BODIES {
        let c = color(model);
        assert_eq!(c.calibration_illuminant, illuminants, "{model}");
        for (cm, code) in c.color_matrix.iter().zip(illuminants) {
            assert_eq!(cm.is_some(), code.is_some(), "{model}");
        }
    }
    for (name, body) in SHARED {
        assert_eq!(color(name).color_matrix, color(body).color_matrix, "{name}");
    }
    assert_ne!(
        color("Canon EOS 90D").color_matrix,
        color("Canon EOS M50").color_matrix
    );
    assert_ne!(
        color("Canon EOS R1").color_matrix,
        color("Canon EOS R3").color_matrix
    );
    for model in ["Canon PowerShot G1 X Mark III", "Canon EOS R8 Mark II"] {
        assert_eq!(color(model).color_matrix, [None, None], "{model}");
    }
}

// cr3_jpeg_matrices_white
#[test]
fn canon_jpeg_matrices_keep_white_neutral() {
    for (model, _) in BODIES {
        let c = color(model);
        for (cm, code) in c.color_matrix.into_iter().zip(c.calibration_illuminant) {
            let (Some(cm), Some(code)) = (cm, code) else {
                continue;
            };
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

// cr3_jpeg_camera_decodes
#[test]
fn jpeg_fitted_cr3_decodes() {
    let mut data = vec![7u16; 1500];
    data[0] = 16;
    data[71..75].copy_from_slice(&[1621, 1024, 1024, 1357]);
    let bytes = file("Canon EOS M50", Some(&data));
    let image = read_raw(&bytes, &ImageBudget::default()).expect("the EOS M50 CR3 reads");
    assert!(image.color.as_shot_neutral.is_some());
    let source = match decode_raw_source(&bytes, &ImageBudget::default()) {
        Ok(source) => source,
        Err(PsdError::Unsupported(why)) => panic!("refused: {why}"),
        Err(e) => panic!("{e}"),
    };
    let want = as_shot_transform(&image.color).unwrap().white_balance;
    assert_eq!(source.as_shot, want);
    assert!((2000.0..=12000.0).contains(&want.temperature), "{want:?}");
}
