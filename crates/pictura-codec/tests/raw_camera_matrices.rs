//! Per-camera colour matrices for vendor raw files (`codec/camera-matrices`,
//! #1465, #1469, #1470) on synthetic CR2s, NEFs and ARWs, and, opt-in, real
//! camera files.

use pictura_codec::{
    as_shot_transform, decode_raw_source, read_raw, DngColor, DngImage, ImageBudget, PsdError,
};
use pictura_color::chroma::{self, Mat3};

#[allow(dead_code)]
#[path = "raw/arw_builder.rs"]
mod arw_builder;
#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "raw/cr2_builder.rs"]
mod cr2_builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/nef_builder.rs"]
mod nef_builder;

use arw_builder::{arw, Arw, Coding};
use builder::{noise_plane, Val};
use cr2_builder::{cr2, Cr2};
use nef_builder::{nef, Data, Nef};

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

/// The Exif model names of every Nikon camera the table seeds.
const NIKON_MODELS: [&str; 13] = [
    "NIKON D70",
    "NIKON D200",
    "NIKON D700",
    "NIKON D800",
    "NIKON D800E",
    "NIKON D810",
    "NIKON D850",
    "NIKON D3300",
    "NIKON D5100",
    "NIKON D5300",
    "NIKON D7000",
    "NIKON Z f",
    "NIKON Z 6_2",
];

fn nef_spec(model: &'static str) -> Nef {
    let mut spec = Nef::new(noise_plane(32, 16, 16383, 8), 14, Data::Containers);
    spec.ifd0_tags = vec![
        (271, Val::Ascii("NIKON CORPORATION")),
        (272, Val::Ascii(model)),
    ];
    spec
}

fn nikon(model: &'static str) -> DngColor {
    read_raw(&nef(&nef_spec(model)), &ImageBudget::default())
        .expect("synthetic NEF reads")
        .color
}

/// The Exif model names of every Sony camera the table seeds.
const SONY_MODELS: [&str; 11] = [
    "DSC-RX100M4",
    "ILCE-6400",
    "ILCE-7CM2",
    "ILCE-7M3",
    "ILCE-7M4",
    "ILCE-7RM2",
    "ILCE-7RM3",
    "ILCE-7RM4",
    "ILCE-7SM2",
    "ILCE-7SM3",
    "ILCE-9",
];

fn arw_spec(model: &'static str) -> Arw {
    let mut spec = Arw::new(noise_plane(32, 16, 16383, 9), Coding::Uncompressed);
    spec.model = model;
    spec
}

fn sony(model: &'static str) -> DngColor {
    read_raw(&arw(&arw_spec(model)), &ImageBudget::default())
        .expect("synthetic ARW reads")
        .color
}

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
fn known_nikon_models_take_fitted_matrices() {
    for model in NIKON_MODELS {
        let color = nikon(model);
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
        nikon("NIKON D800").color_matrix,
        nikon("NIKON D800E").color_matrix
    );
    assert_eq!(
        nikon("NIKON Z f").color_matrix,
        nikon("NIKON Z 6_2").color_matrix
    );
    assert_ne!(
        nikon("NIKON D850").color_matrix,
        nikon("NIKON Z f").color_matrix
    );
    assert_eq!(nikon("NIKON D4").color_matrix, [None, None]);
}

#[test]
fn proxy_bodies_take_their_siblings_matrices() {
    for (proxy, sibling) in [
        ("NIKON D100", nikon("NIKON D70")),
        ("NIKON D7100", nikon("NIKON D5300")),
        ("COOLPIX P7100", canon("Canon PowerShot S90")),
    ] {
        let color = nikon(proxy);
        assert!(color.color_matrix.iter().all(Option::is_some), "{proxy}");
        assert_eq!(color.color_matrix, sibling.color_matrix, "{proxy}");
    }
}

#[test]
fn nef_as_shot_neutral_from_wb_levels() {
    let as_shot = |wb: Option<Vec<(u32, u32)>>| {
        let mut spec = nef_spec("NIKON D850");
        spec.wb = wb;
        let bytes = nef(&spec);
        let image = read_raw(&bytes, &ImageBudget::default()).expect("D850 NEF reads");
        (bytes, image.color.as_shot_neutral)
    };
    let (bytes, neutral) = as_shot(Some(vec![(19, 10), (11, 8), (1, 1), (1, 1)]));
    let neutral = neutral.expect("WB_RBLevels give a neutral");
    for (got, want) in neutral.iter().zip([1.0 / 1.9, 1.0, 1.0 / 1.375]) {
        assert!((got - want).abs() <= 1e-9, "{neutral:?}");
    }
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("D850 NEF decodes");
    let t = source.as_shot.temperature;
    assert!((2000.0..=12000.0).contains(&t), "{t} K");
    assert_eq!(as_shot(Some(vec![(0, 1), (11, 8), (1, 1), (1, 1)])).1, None);
    assert_eq!(as_shot(None).1, None);

    // The D100 keeps its levels in the version-0100 colour balance.
    for be in [true, false] {
        let mut spec = nef_spec("NIKON D100");
        spec.be = be;
        spec.color_balance_0100 = Some([581, 436, 256, 256]);
        let image = read_raw(&nef(&spec), &ImageBudget::default()).expect("D100 NEF reads");
        let neutral = image.color.as_shot_neutral.expect("colour balance neutral");
        for (got, want) in neutral.iter().zip([256.0 / 581.0, 1.0, 256.0 / 436.0]) {
            assert!((got - want).abs() <= 1e-9, "{neutral:?}");
        }
    }
}

#[test]
fn known_sony_models_take_fitted_matrices() {
    for model in SONY_MODELS {
        let color = sony(model);
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
    for (alias, body) in [
        ("ILCE-7RM3A", "ILCE-7RM3"),
        ("ILCE-7RM4A", "ILCE-7RM4"),
        ("ILCE-7RM5", "ILCE-7RM4"),
    ] {
        assert_eq!(sony(alias).color_matrix, sony(body).color_matrix, "{alias}");
    }
    assert_ne!(
        sony("ILCE-7M3").color_matrix,
        sony("ILCE-7RM3").color_matrix
    );
    assert_eq!(sony("ILCE-1").color_matrix, [None, None]);
}

#[test]
fn known_camera_arw_decodes() {
    let bytes = arw(&arw_spec("ILCE-7M3"));
    let image = read_raw(&bytes, &ImageBudget::default()).expect("ILCE-7M3 ARW reads");
    let neutral = image
        .color
        .as_shot_neutral
        .expect("WB_RGGBLevels give a neutral");
    for (got, want) in neutral.iter().zip([1024.0 / 2272.0, 1.0, 1024.0 / 1800.0]) {
        assert!((got - want).abs() <= 1e-9, "{neutral:?}");
    }
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("ILCE-7M3 decodes");
    let want = as_shot_transform(&image.color)
        .expect("as-shot transform")
        .white_balance;
    assert_eq!(source.as_shot, want);
    let t = source.as_shot.temperature;
    assert!((2000.0..=12000.0).contains(&t), "{t} K");
}

#[test]
fn fitted_matrices_keep_white_neutral() {
    let colors = TABLE_MODELS
        .into_iter()
        .map(|m| (m, canon(m)))
        .chain(NIKON_MODELS.into_iter().map(|m| (m, nikon(m))))
        .chain(SONY_MODELS.into_iter().map(|m| (m, sony(m))));
    for (model, color) in colors {
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

fn corpus_colour(ext: &str) {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
        {
            continue;
        }
        seen += 1;
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let Ok(image) = read_raw(&bytes, &ImageBudget::default()) else {
            eprintln!("{}: not read", path.display());
            continue;
        };
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
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .{ext} file");
}

#[test]
fn nef_corpus_colour_when_present() {
    corpus_colour("NEF");
}

#[test]
fn arw_corpus_colour_when_present() {
    corpus_colour("ARW");
}
