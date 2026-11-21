//! `codec/dng-writer` (#402): written DNGs read back through the reader and
//! inspected with the reader's own TIFF parser.

use super::super::tiff::{Ifd, Tiff};
use super::super::{read_dng, read_linear_dng, CameraRgb, CfaPattern, DngColor, DngCrop};
use super::*;
use crate::{raw_xmp, ImageBudget};

fn noise(len: usize, seed: u64) -> Vec<u16> {
    let mut state = seed;
    (0..len)
        .map(|i| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((i as u64 * 97 % 50000) + (state >> 49)) as u16
        })
        .collect()
}

fn grbg() -> CfaPattern {
    CfaPattern {
        width: 2,
        height: 2,
        colors: vec![1, 0, 2, 1],
    }
}

fn color() -> DngColor {
    let m =
        |k: f64| std::array::from_fn(|i| (i as f64 - 4.0) * k + if i % 4 == 0 { 1.0 } else { 0.0 });
    DngColor {
        color_matrix: [Some(m(0.0625)), Some(m(0.125))],
        forward_matrix: [Some(m(0.03125)), Some(m(0.25))],
        camera_calibration: [Some(m(0.0)), None],
        calibration_illuminant: [Some(17), Some(21)],
        as_shot_neutral: Some([0.5, 1.0, 0.75]),
        analog_balance: Some([1.0, 1.0, 1.0]),
        baseline_exposure: 0.5,
    }
}

fn crop() -> DngCrop {
    DngCrop {
        x: 2.0,
        y: 3.0,
        width: 36.0,
        height: 24.0,
    }
}

fn look() -> Vec<u8> {
    (0..400 * 300 * 3).map(|i| (i % 251) as u8).collect()
}

fn request<'a>(
    width: usize,
    height: usize,
    pixels: DngPixels<'a>,
    color: &'a DngColor,
    look: &'a [u8],
) -> DngWrite<'a> {
    DngWrite {
        width,
        height,
        pixels,
        color,
        crop: crop(),
        orientation: 6,
        make: Some("Kooka"),
        model: Some("Kooka Test"),
        look: DngRgb8 {
            width: 400,
            height: 300,
            rgb: look,
        },
        preview: None,
        xmp: None,
        original: None,
        compatibility: DngCompatibility::default(),
    }
}

fn write(request: &DngWrite) -> Vec<u8> {
    write_dng(request, &|_| {}, &|| false).unwrap().unwrap()
}

fn budget() -> ImageBudget {
    ImageBudget::default()
}

/// IFD0 and its SubIFDs.
fn ifds(bytes: &[u8]) -> (Tiff<'_>, Ifd, Vec<Ifd>) {
    let (tiff, first) = Tiff::parse(bytes).unwrap();
    let ifd0 = tiff.ifd(first).unwrap();
    let subs = tiff
        .uints(ifd0.get(SUB_IFDS).unwrap())
        .unwrap()
        .into_iter()
        .map(|at| tiff.ifd(at).unwrap())
        .collect();
    (tiff, ifd0, subs)
}

fn uints(tiff: &Tiff, ifd: &Ifd, tag: u16) -> Option<Vec<u64>> {
    ifd.get(tag).map(|e| tiff.uints(e).unwrap())
}

// dng_write_mosaic_round_trip
#[test]
fn mosaic_round_trips() {
    let samples = noise(40 * 30, 1);
    let cfa = grbg();
    let color = color();
    let look = look();
    let bytes = write(&request(
        40,
        30,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        &color,
        &look,
    ));
    let image = read_dng(&bytes, &budget()).unwrap();
    assert_eq!((image.width, image.height), (40, 30));
    assert_eq!(image.samples, samples);
    assert_eq!(image.cfa, cfa);
    assert_eq!(image.default_crop, crop());
    assert_eq!(image.orientation, 6);
    assert_eq!(image.make.as_deref(), Some("Kooka"));
    assert_eq!(image.model.as_deref(), Some("Kooka Test"));
    assert_eq!(image.color, color);
}

// dng_write_mosaic_tiles
#[test]
fn mosaic_spans_tiles() {
    let samples = noise(600 * 300, 2);
    let cfa = grbg();
    let color = color();
    let look = look();
    let bytes = write(&request(
        600,
        300,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        &color,
        &look,
    ));
    let (tiff, _, subs) = ifds(&bytes);
    let raw = &subs[0];
    assert_eq!(uints(&tiff, raw, COMPRESSION), Some(vec![7]));
    assert_eq!(uints(&tiff, raw, TILE_WIDTH), Some(vec![256]));
    assert_eq!(uints(&tiff, raw, TILE_LENGTH), Some(vec![256]));
    assert_eq!(uints(&tiff, raw, TILE_OFFSETS).unwrap().len(), 6);
    assert_eq!(read_dng(&bytes, &budget()).unwrap().samples, samples);
}

#[test]
fn constant_and_extreme_tiles_round_trip() {
    let flat = vec![1234u16; 300 * 20];
    let jumps: Vec<u16> = (0..300 * 20)
        .map(|i| [0, 32768, 65535, 0, 1][i % 5])
        .collect();
    let cfa = grbg();
    let color = color();
    let look = look();
    for samples in [flat, jumps] {
        let bytes = write(&request(
            300,
            20,
            DngPixels::Mosaic {
                samples: &samples,
                cfa: &cfa,
            },
            &color,
            &look,
        ));
        assert_eq!(read_dng(&bytes, &budget()).unwrap().samples, samples);
    }
}

// dng_write_linear_round_trip
#[test]
fn linear_round_trips() {
    let camera = CameraRgb {
        width: 37,
        height: 21,
        rgb: noise(37 * 21 * 3, 3),
    };
    let color = DngColor {
        color_matrix: [Some(std::array::from_fn(|i| i as f64 / 8.0)), None],
        calibration_illuminant: [Some(21), None],
        as_shot_neutral: Some([0.25, 1.0, 0.5]),
        ..DngColor::default()
    };
    let look = look();
    let mut req = request(
        37,
        21,
        DngPixels::Linear { rgb: &camera.rgb },
        &color,
        &look,
    );
    req.orientation = 3;
    let read = read_linear_dng(&write(&req), &budget()).unwrap();
    assert_eq!(read.camera, camera);
    assert_eq!(read.color, color);
    assert_eq!(read.default_crop, crop());
    assert_eq!(read.orientation, 3);
    assert_eq!(read.make.as_deref(), Some("Kooka"));
    assert_eq!(read.model.as_deref(), Some("Kooka Test"));
}

/// What a test sets on top of the 40 × 30 mosaic request.
#[derive(Default)]
struct Extras<'a> {
    preview: Option<DngRgb8<'a>>,
    xmp: Option<&'a [u8]>,
    original: Option<(&'a str, &'a [u8])>,
    compatibility: DngCompatibility,
}

fn small_mosaic(extras: Extras) -> Vec<u8> {
    let samples = noise(40 * 30, 4);
    let cfa = grbg();
    let color = color();
    let look = look();
    let mut req = request(
        40,
        30,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        &color,
        &look,
    );
    req.preview = extras.preview;
    req.xmp = extras.xmp;
    req.original = extras.original;
    req.compatibility = extras.compatibility;
    write(&req)
}

// dng_write_thumbnail
#[test]
fn ifd0_is_thumbnail() {
    let bytes = small_mosaic(Extras::default());
    let (tiff, ifd0, subs) = ifds(&bytes);
    assert_eq!(subs.len(), 1);
    assert_eq!(uints(&tiff, &ifd0, NEW_SUBFILE_TYPE), Some(vec![1]));
    assert_eq!(uints(&tiff, &ifd0, PHOTOMETRIC), Some(vec![2]));
    assert_eq!(uints(&tiff, &ifd0, BITS_PER_SAMPLE), Some(vec![8, 8, 8]));
    assert_eq!(uints(&tiff, &ifd0, IMAGE_WIDTH), Some(vec![256]));
    assert_eq!(uints(&tiff, &ifd0, IMAGE_LENGTH), Some(vec![192]));
    assert_eq!(
        uints(&tiff, &ifd0, STRIP_BYTE_COUNTS),
        Some(vec![256 * 192 * 3])
    );
}

// dng_write_jpeg_preview
#[test]
fn jpeg_preview_ifd() {
    let preview: Vec<u8> = (0..120 * 80 * 3).map(|i| (i % 200) as u8).collect();
    let bytes = small_mosaic(Extras {
        preview: Some(DngRgb8 {
            width: 120,
            height: 80,
            rgb: &preview,
        }),
        ..Extras::default()
    });
    let (tiff, _, subs) = ifds(&bytes);
    assert_eq!(subs.len(), 2);
    let ifd = &subs[1];
    assert_eq!(uints(&tiff, ifd, NEW_SUBFILE_TYPE), Some(vec![1]));
    assert_eq!(uints(&tiff, ifd, COMPRESSION), Some(vec![7]));
    assert_eq!(uints(&tiff, ifd, PHOTOMETRIC), Some(vec![6]));
    assert_eq!(uints(&tiff, ifd, PREVIEW_COLOR_SPACE), Some(vec![2]));
    let at = uints(&tiff, ifd, STRIP_OFFSETS).unwrap()[0] as usize;
    let len = uints(&tiff, ifd, STRIP_BYTE_COUNTS).unwrap()[0] as usize;
    let strip = &bytes[at..at + len];
    assert_eq!(&strip[..2], &[0xFF, 0xD8]);
    let decoded = image::load_from_memory_with_format(strip, image::ImageFormat::Jpeg).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (120, 80));
}

// dng_write_xmp
#[test]
fn xmp_packet_round_trips() {
    let packet = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF><rdf:Description crs:Exposure2012="+0.50"/></rdf:RDF></x:xmpmeta>"#;
    let bytes = small_mosaic(Extras {
        xmp: Some(packet),
        ..Extras::default()
    });
    assert_eq!(raw_xmp(&bytes).as_deref(), Some(&packet[..]));
}

// dng_write_original_raw
#[test]
fn original_raw_inflates() {
    let original: Vec<u8> = (0..200_000u32).map(|i| (i * 7 % 256) as u8).collect();
    let bytes = small_mosaic(Extras {
        original: Some(("shot.cr2", &original)),
        ..Extras::default()
    });
    let (tiff, ifd0, _) = ifds(&bytes);
    assert_eq!(
        tiff.ascii(ifd0.get(ORIGINAL_RAW_FILE_NAME).unwrap())
            .as_deref(),
        Some("shot.cr2")
    );
    let data = tiff.raw(ifd0.get(ORIGINAL_RAW_FILE_DATA).unwrap());
    let fork = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
    assert_eq!(fork.div_ceil(original::BLOCK), 4);
    assert_eq!(read_original_raw(&bytes), Some(original));
}

// dng_write_compatibility
#[test]
fn compatibility_sets_versions() {
    let preview = vec![9u8; 8 * 8 * 3];
    let at = |compatibility| {
        small_mosaic(Extras {
            preview: Some(DngRgb8 {
                width: 8,
                height: 8,
                rgb: &preview,
            }),
            compatibility,
            ..Extras::default()
        })
    };
    let old = at(DngCompatibility::CameraRaw2_4);
    let (tiff, ifd0, subs) = ifds(&old);
    assert_eq!(uints(&tiff, &ifd0, DNG_VERSION), Some(vec![1, 1, 0, 0]));
    assert_eq!(
        uints(&tiff, &ifd0, DNG_BACKWARD_VERSION),
        Some(vec![1, 1, 0, 0])
    );
    assert!(ifd0.get(FORWARD_MATRIX[0]).is_none());
    assert!(subs[1].get(PREVIEW_COLOR_SPACE).is_none());

    let new = at(DngCompatibility::CameraRaw6_6);
    let (tiff, ifd0, subs) = ifds(&new);
    assert_eq!(uints(&tiff, &ifd0, DNG_VERSION), Some(vec![1, 4, 0, 0]));
    assert_eq!(
        uints(&tiff, &ifd0, DNG_BACKWARD_VERSION),
        Some(vec![1, 4, 0, 0])
    );
    assert!(ifd0.get(FORWARD_MATRIX[0]).is_some());
    assert_eq!(uints(&tiff, &subs[1], PREVIEW_COLOR_SPACE), Some(vec![2]));
}

// dng_write_refuses
#[test]
fn inconsistent_input_is_refused() {
    let cfa = grbg();
    let color = color();
    let look = look();
    let short_preview = vec![0u8; 10 * 10 * 3 - 1];
    let samples = noise(40 * 30, 5);
    let empty = request(
        0,
        0,
        DngPixels::Mosaic {
            samples: &[],
            cfa: &cfa,
        },
        &color,
        &look,
    );
    let missing = request(
        40,
        30,
        DngPixels::Mosaic {
            samples: &samples[1..],
            cfa: &cfa,
        },
        &color,
        &look,
    );
    let mut preview = request(
        40,
        30,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        &color,
        &look,
    );
    preview.preview = Some(DngRgb8 {
        width: 10,
        height: 10,
        rgb: &short_preview,
    });
    for req in [empty, missing, preview] {
        assert!(matches!(
            write_dng(&req, &|_| {}, &|| false),
            Err(PsdError::Invalid(_))
        ));
    }
}

// dng_write_cancel
#[test]
fn cancelled_write_returns_none() {
    let samples = noise(40 * 30, 6);
    let cfa = grbg();
    let color = color();
    let look = look();
    let req = request(
        40,
        30,
        DngPixels::Mosaic {
            samples: &samples,
            cfa: &cfa,
        },
        &color,
        &look,
    );
    assert!(write_dng(&req, &|_| {}, &|| true).unwrap().is_none());
}

#[test]
fn rationals_keep_short_decimals() {
    for v in [0.5, -1.25, 0.1234, 3.0, 0.0, -0.000125] {
        let (n, d) = tiff_out::srational(v);
        assert_eq!(n as f64 / d as f64, v);
    }
    let (n, d) = tiff_out::rational(4321.5);
    assert_eq!(n as f64 / d as f64, 4321.5);
}
