//! Colour for Fujifilm RAF files (`codec/camera-matrices`, #1473, #1621):
//! the table's Fujifilm rows on synthetic RAFs, and, opt-in, real camera
//! files.

use pictura_codec::{as_shot_transform, decode_raw_source, read_raw, DngColor, ImageBudget};
use pictura_color::chroma::{self, Mat3};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[allow(dead_code)]
#[path = "raw/raf_builder.rs"]
mod raf_builder;

use builder::noise_plane;
use raf_builder::{raf, Coding, Exif, Raf};

/// Every Fujifilm body the table colours, with the EXIF `LightSource` codes
/// of its matrices: the D65 preset, then the Standard A one when paired, or,
/// for word-section bodies (#1621), Fine Weather then Tungsten. The X-E1,
/// X100S, X-S1, XF1 and S2 Pro samples failed the fit and have no row.
const BODIES: [(&str, &[u16]); 24] = [
    ("FinePix X100", &[21, 17]),
    ("GFX 50S", &[21, 17]),
    ("X-A1", &[21, 17]),
    ("X-A5", &[21, 17]),
    ("X-E2", &[21, 17]),
    ("X-H1", &[21, 17]),
    ("X-Pro1", &[21, 17]),
    ("X-T1", &[21, 17]),
    ("X-T2", &[21, 17]),
    ("X-T3", &[21, 17]),
    ("X10", &[21, 17]),
    ("FinePix F550EXR", &[21, 17]),
    ("FinePix HS20EXR", &[21, 17]),
    ("FinePix HS50EXR", &[21, 17]),
    ("FinePix E550", &[9, 3]),
    ("FinePix E900", &[9, 3]),
    ("FinePix F700", &[9, 3]),
    ("FinePix S100FS", &[9, 3]),
    ("FinePix S200EXR", &[9]),
    ("FinePix S3Pro", &[9, 3]),
    ("FinePix S5600", &[9, 3]),
    ("FinePix S5Pro", &[9, 3]),
    ("FinePix S7000", &[9]),
    ("FinePix S9500", &[9, 3]),
];

fn file(model: &'static str, wb: [u32; 3]) -> Vec<u8> {
    let samples = noise_plane(36, 24, 16383, 29).samples;
    let mut r = Raf::new(36, 24, samples, Coding::Words);
    r.exif = Some(Exif {
        make: "FUJIFILM",
        model,
        orientation: 1,
    });
    r.wb = Some(wb);
    raf(&r)
}

fn color(model: &'static str) -> DngColor {
    read_raw(&file(model, [302, 577, 454]), &ImageBudget::default())
        .unwrap_or_else(|e| panic!("{model}: {e}"))
        .color
}

fn matrix(v: [f64; 9]) -> Mat3 {
    std::array::from_fn(|r| [v[3 * r], v[3 * r + 1], v[3 * r + 2]])
}

fn kelvin(code: u16) -> f64 {
    match code {
        3 => 2850.0,
        9 => 5500.0,
        17 => 2856.0,
        21 => 6504.0,
        other => panic!("unexpected illuminant {other}"),
    }
}

// raf_jpeg_matrices
#[test]
fn fujifilm_bodies_take_their_matrices() {
    for (model, codes) in BODIES {
        let c = color(model);
        let lights: Vec<u16> = c.calibration_illuminant.iter().flatten().copied().collect();
        assert_eq!(lights, codes, "{model}");
        let matrices = c.color_matrix.iter().flatten().count();
        assert_eq!(matrices, codes.len(), "{model}");
    }
    assert_ne!(color("X-T3").color_matrix, color("X-T2").color_matrix);
    assert_eq!(color("X-T99").color_matrix, [None, None]);
}

// raf_jpeg_matrices_white
#[test]
fn fujifilm_matrices_keep_white_neutral() {
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

// raf_known_camera_decodes
#[test]
fn known_fujifilm_raf_decodes() {
    let bytes = file("X-T3", [302, 583, 584]);
    let image = read_raw(&bytes, &ImageBudget::default()).unwrap();
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("X-T3 decodes");
    let want = as_shot_transform(&image.color).unwrap().white_balance;
    assert_eq!(source.as_shot, want);
    assert!((2000.0..=12000.0).contains(&want.temperature), "{want:?}");
}

// raf_superccd_decodes
#[test]
fn fitted_superccd_raf_decodes_upright() {
    let (width, height) = (16, 24);
    let mut r = Raf::words(width, height, noise_plane(width, height, 15872, 41).samples);
    r.crop = None;
    r.layout = [0x80, 0x81, 0x82, 0x83];
    r.exif = Some(Exif {
        make: "FUJIFILM",
        model: "FinePix S7000",
        orientation: 1,
    });
    r.record(0x0121, &[12, 32]);
    r.record(0x2FF0, &[80, 167, 80, 119]);
    let source = decode_raw_source(&raf(&r), &ImageBudget::default()).expect("S7000 decodes");
    // fw 16 on a 28 × 27 grid: ⌊15 / √½⌋ × ⌊12 / √½⌋.
    assert_eq!((source.camera.width, source.camera.height), (21, 16));
    let t = source.as_shot.temperature;
    assert!((2000.0..=12000.0).contains(&t), "{t} K");
}

// raf_colour_corpus
#[test]
fn raf_corpus_colour_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP raf colour corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let bytes = std::fs::read(&path).expect("corpus file");
        if !bytes.starts_with(b"FUJIFILMCCD-RAW ") {
            continue;
        }
        let name = path.display();
        let Ok(image) = read_raw(&bytes, &ImageBudget::default()) else {
            continue;
        };
        if !image.color.color_matrix.iter().any(Option::is_some) {
            eprintln!("{name}: {:?} has no table row", image.model);
            continue;
        }
        match decode_raw_source(&bytes, &ImageBudget::default()) {
            Ok(source) => {
                let t = source.as_shot.temperature;
                eprintln!("{name}: {:?} as shot {t:.0} K", image.model);
                if !(2000.0..=12000.0).contains(&t) {
                    failures.push(format!("{name}: {t} K"));
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
