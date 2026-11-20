//! Per-camera colour matrices for vendor raw files (`codec/camera-matrices`,
//! #1465) on synthetic CR2s, and, opt-in, real camera files.

use pictura_codec::{
    as_shot_transform, decode_raw_source, read_raw, DngColor, DngImage, ImageBudget, PsdError,
};
use pictura_color::chroma::{self, Mat3};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "raw/cr2_builder.rs"]
mod cr2_builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use builder::{noise_plane, Val};
use cr2_builder::{cr2, Cr2};

const A_XY: [f64; 2] = [0.44757, 0.40745];
const D65_XY: [f64; 2] = [0.31271, 0.32902];

/// The Exif model names of every camera the table seeds.
const TABLE_MODELS: [&str; 14] = [
    "Canon EOS 400D DIGITAL",
    "Canon EOS-1D X Mark II",
    "Canon EOS-1Ds Mark II",
    "Canon EOS-1Ds Mark III",
    "Canon EOS 100D",
    "Canon EOS 200D",
    "Canon EOS 5D",
    "Canon EOS 5DS",
    "Canon EOS 5D Mark II",
    "Canon EOS 5D Mark III",
    "Canon EOS 5D Mark IV",
    "Canon EOS M",
    "Canon PowerShot S90",
    "Canon EOS 6D Mark II",
];

fn spec(make: Option<&'static str>, model: Option<&'static str>) -> Cr2 {
    let mut spec = Cr2::new(noise_plane(32, 16, 4095, 4), 12);
    spec.ifd0_tags = make
        .map(|m| (271, Val::Ascii(m)))
        .into_iter()
        .chain(model.map(|m| (272, Val::Ascii(m))))
        .collect();
    spec
}

fn read(spec: &Cr2) -> DngImage {
    read_raw(&cr2(spec), &ImageBudget::default()).expect("synthetic CR2 reads")
}

fn canon(model: &'static str) -> DngColor {
    read(&spec(Some("Canon"), Some(model))).color
}

fn matrix(v: [f64; 9]) -> Mat3 {
    std::array::from_fn(|r| [v[3 * r], v[3 * r + 1], v[3 * r + 2]])
}

#[test]
fn known_canon_models_take_fitted_matrices() {
    for model in TABLE_MODELS
        .into_iter()
        .chain(["Canon EOS 5DS R", "Canon EOS DIGITAL REBEL XTi"])
    {
        let color = canon(model);
        assert!(
            color.color_matrix.iter().all(Option::is_some),
            "{model}: {color:?}"
        );
        assert_eq!(
            color.calibration_illuminant,
            [Some(17), Some(21)],
            "{model}"
        );
    }
    assert_eq!(
        canon("Canon EOS 5DS").color_matrix,
        canon("Canon EOS 5DS R").color_matrix
    );
    let xti = canon("Canon EOS 400D DIGITAL").color_matrix;
    assert_eq!(canon("Canon EOS DIGITAL REBEL XTi").color_matrix, xti);
    assert_eq!(canon("Canon EOS Kiss Digital X").color_matrix, xti);
    assert_ne!(canon("Canon EOS 5DS").color_matrix, xti);
}

#[test]
fn unknown_cameras_keep_no_matrices() {
    for (make, model) in [
        (Some("Canon"), Some("Canon EOS 450D")),
        (Some("Nikon"), Some("EOS 5DS")),
        (None, None),
    ] {
        let color = read(&spec(make, model)).color;
        assert_eq!(color.color_matrix, [None, None], "{make:?} {model:?}");
    }
    let bytes = cr2(&spec(Some("Canon"), Some("Canon EOS 450D")));
    match decode_raw_source(&bytes, &ImageBudget::default()) {
        Err(PsdError::Unsupported(message)) => {
            assert!(message.contains("camera colour"), "{message}");
            assert!(message.contains("Canon EOS 450D"), "{message}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn fitted_matrices_keep_white_neutral() {
    for model in TABLE_MODELS {
        let color = canon(model);
        let [Some(a), Some(d65)] = color.color_matrix else {
            panic!("{model} has no matrices");
        };
        for (cm, xy, kelvin) in [(d65, D65_XY, 6504.0), (a, A_XY, 2856.0)] {
            let neutral = chroma::mul_vec(&matrix(cm), chroma::xy_to_xyz(xy));
            assert!(neutral.iter().all(|&v| v > 0.0), "{model}: {neutral:?}");
            let shot = DngColor {
                as_shot_neutral: Some(neutral),
                ..color.clone()
            };
            let t = as_shot_transform(&shot)
                .unwrap_or_else(|e| panic!("{model}: {e}"))
                .white_balance
                .temperature;
            assert!(
                (t - kelvin).abs() <= 0.02 * kelvin,
                "{model}: {t} K for {kelvin} K"
            );
        }
    }
}

#[test]
fn known_camera_cr2_decodes() {
    let mut spec = spec(Some("Canon"), Some("Canon EOS 5DS"));
    let mut data = vec![7u16; 1560];
    data[0] = 12;
    data[63..67].copy_from_slice(&[2208, 1024, 1024, 1967]);
    spec.color_data = Some(data);
    let bytes = cr2(&spec);
    let image = read_raw(&bytes, &ImageBudget::default()).expect("5DS CR2 reads");
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("5DS CR2 decodes");
    let expected = as_shot_transform(&image.color).unwrap().white_balance;
    assert_eq!(source.as_shot, expected);
    assert!(
        (2000.0..=12000.0).contains(&expected.temperature),
        "{expected:?}"
    );
}

#[test]
fn cr2_corpus_colour_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cr2"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let image = read_raw(&bytes, &ImageBudget::default())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let known = image.color.color_matrix.iter().all(Option::is_some);
        let decoded = decode_raw_source(&bytes, &ImageBudget::default());
        if known {
            let source = decoded.unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let t = source.as_shot.temperature;
            assert!((2000.0..=12000.0).contains(&t), "{}: {t} K", path.display());
            eprintln!(
                "{}: {:?} neutral {:?} as shot {:.0} K tint {:.1}",
                path.display(),
                image.model,
                image.color.as_shot_neutral,
                t,
                source.as_shot.tint
            );
        } else {
            assert!(
                matches!(&decoded, Err(PsdError::Unsupported(m)) if m.contains("camera colour")),
                "{}: {:?}",
                path.display(),
                decoded.err()
            );
            eprintln!("{}: {:?} refused", path.display(), image.model);
        }
        seen += 1;
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .CR2 file");
}
