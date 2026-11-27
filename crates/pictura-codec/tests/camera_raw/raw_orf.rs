//! The Olympus ORF reader (`codec/vendor-raw-containers`, #1471) against
//! synthetic ORFs written by `tests/camera_raw/raw/orf_builder.rs`, and, opt-in, real
//! camera files.

use pictura_codec::{
    as_shot_transform, decode_raw_source, demosaic, read_raw, sniff_raw, sniff_raw_prefix,
    DemosaicMethod, DngImage, ImageBudget, PsdError, RawFormat,
};
use pictura_color::chroma;

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[path = "raw/orf_builder.rs"]
mod orf_builder;

use builder::{noise_plane, write_tiff, IfdSpec, Val};
use orf_builder::{orf, Data, Note, Orf};

const WIDTH: usize = 40;
const HEIGHT: usize = 24;
const CODINGS: [Data; 3] = [Data::PaddedLsb, Data::MsbWords, Data::Msb];

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn samples(max: u16) -> Vec<u16> {
    noise_plane(WIDTH, HEIGHT, max, 21).samples
}

fn packed(data: Data) -> Orf {
    Orf::new(WIDTH, HEIGHT, samples(4095), data)
}

/// An ORF whose one strip is shorter than any uncompressed layout, without
/// the coding parameters a compressed stream needs.
fn compressed(bits: u16) -> Orf {
    let mut o = packed(Data::Strips(vec![vec![0x5A; WIDTH * HEIGHT]]));
    o.bits = bits;
    o
}

fn normalized(o: &Orf, white: f64) -> Vec<u16> {
    o.samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let black = o.black[(i / o.width % 2) * 2 + i % o.width % 2] as f64;
            ((v as f64 - black) * 65535.0 / (white - black))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

#[test]
fn orf_is_named_by_its_header() {
    for magic in [b"IIRO", b"IIRS", b"MMOR"] {
        let mut o = packed(Data::Words(4));
        o.magic = magic;
        let bytes = orf(&o);
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Orf));
        assert_eq!(sniff_raw(&bytes[..8]), Some(RawFormat::Orf));
        assert!(sniff_raw_prefix(&bytes[..8]));
        read(&bytes).expect("ORF through read_raw");
    }
    let tiff = IfdSpec {
        tags: vec![(271, Val::Ascii("OLYMPUS CORPORATION"))],
        ..IfdSpec::default()
    };
    let tiff = write_tiff(&tiff, false, false);
    assert_eq!(sniff_raw(&tiff), None);
    assert!(!sniff_raw_prefix(&tiff));
}

#[test]
fn sixteen_bit_orf_words_decode_exactly() {
    for (magic, bits) in [(b"IIRO", 12u16), (b"MMOR", 10)] {
        let ceiling = (1u16 << bits) - 1;
        let data = Data::Words(16 - u32::from(bits));
        let mut o = Orf::new(WIDTH, HEIGHT, samples(ceiling), data);
        o.magic = magic;
        o.bits = bits;
        o.rows_per_strip = 7;
        let image = read(&orf(&o)).unwrap();
        assert_eq!(image.samples, normalized(&o, ceiling as f64), "{bits} bits");
    }
}

#[test]
fn packed_orfs_decode_exactly() {
    for data in CODINGS {
        let mut o = packed(data.clone());
        if matches!(data, Data::Msb) {
            o.rows_per_strip = 5;
        }
        let image = read(&orf(&o)).unwrap();
        assert_eq!(image.samples, normalized(&o, 4095.0), "{data:?}");

        o.white = Some(3895);
        let image = read(&orf(&o)).unwrap();
        assert_eq!(
            image.samples,
            normalized(&o, 3895.0),
            "{data:?} with a white"
        );
    }
}

#[test]
fn interlaced_compact_orfs_put_their_fields_back() {
    for (height, rows_per_strip) in [(23, 4), (23, 16), (24, 5), (24, 24)] {
        let samples = noise_plane(WIDTH, height, 4095, 22).samples;
        let mut o = Orf::new(WIDTH, height, samples, Data::Msb);
        o.rows_per_strip = rows_per_strip;
        let image = read(&orf(&o)).unwrap();
        assert_eq!(
            image.samples,
            normalized(&o, 4095.0),
            "{height} rows by {rows_per_strip}"
        );
    }
}

#[test]
fn compressed_orfs_without_their_coding_are_refused() {
    for bits in [12, 14] {
        let bytes = orf(&compressed(bits));
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Orf));
        match decode_raw_source(&bytes, &ImageBudget::default()) {
            Err(PsdError::Unsupported(why)) => {
                assert!(why.contains("coding parameters"), "{why}")
            }
            other => panic!("{bits} bits: {:?}", other.err()),
        }
    }
}

#[test]
fn orf_variants_are_refused() {
    let n = WIDTH * HEIGHT;
    let wide = packed(Data::Strips(vec![vec![0; n * 7 / 4]]));
    let mut short = packed(Data::Strips(vec![vec![0; n / 4]; 2]));
    short.rows_per_strip = HEIGHT / 2;
    let mut bits = packed(Data::PaddedLsb);
    bits.bits = 14;
    let mut cfa = packed(Data::Msb);
    cfa.cfa = [0, 1, 2, 1];
    for (what, o) in [
        ("size", wide),
        ("strips", short),
        ("bits", bits),
        ("cfa", cfa),
    ] {
        assert!(
            matches!(read(&orf(&o)), Err(PsdError::Unsupported(_))),
            "{what}"
        );
    }
}

#[test]
fn every_maker_note_layout_reads_alike() {
    let notes = [
        Note::Olympus,
        Note::OmSystem,
        Note::OlympValue,
        Note::OlympRawInfo,
        Note::OlympMain,
    ];
    for (note, stray) in notes.into_iter().zip([true, false, true, false, false]) {
        let mut o = packed(Data::Msb);
        o.note = note;
        o.crop = Some([3, 2, 32, 20]);
        o.stray = stray;
        let image = read(&orf(&o)).unwrap();
        assert_eq!(image.samples, normalized(&o, 4095.0), "{note:?}");
        assert_eq!(
            image.color.as_shot_neutral,
            Some([0.5, 1.0, 256.0 / 384.0]),
            "{note:?}"
        );
        assert!(image.color.color_matrix[0].is_some(), "{note:?}");
        let c = image.default_crop;
        let expected = match note {
            Note::OlympMain => [0.0, 0.0, WIDTH as f64, HEIGHT as f64],
            _ => [3.0, 2.0, 32.0, 20.0],
        };
        assert_eq!([c.x, c.y, c.width, c.height], expected, "{note:?}");
    }
}

#[test]
fn orf_reports_camera_tags() {
    let mut o = packed(Data::PaddedLsb);
    o.note = Note::OmSystem;
    o.make = "OM Digital Solutions";
    o.model = "OM-5";
    o.orientation = 6;
    o.cfa = [1, 0, 2, 1];
    let image = read(&orf(&o)).unwrap();
    assert_eq!(image.make.as_deref(), Some("OM Digital Solutions"));
    assert_eq!(image.model.as_deref(), Some("OM-5"));
    assert_eq!(image.orientation, 6);
    assert_eq!(image.cfa.colors, [1, 0, 2, 1]);
    demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default()).expect("demosaic");
}

#[test]
fn hostile_orfs_fail_cleanly() {
    let mut files = vec![packed(Data::Words(4)), compressed(12)];
    files.extend(CODINGS.map(packed));
    for o in &files {
        let bytes = orf(o);
        for len in 0..bytes.len() {
            assert!(read(&bytes[..len]).is_err(), "{:?} cut at {len}", o.data);
        }
        // The header and every directory, which follow the strips.
        let mut tiny = o.clone();
        tiny.crop = Some([2, 2, 32, 20]);
        let bytes = orf(&tiny);
        let directories = bytes.len().saturating_sub(700);
        for at in (4..8).chain(directories..bytes.len()) {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                broken[at] ^= flip;
                // Any answer but a panic.
                let _ = read(&broken);
            }
        }
    }

    let mut crop = packed(Data::Msb);
    crop.crop = Some([8, 0, WIDTH as u16, 8]);
    assert!(read(&orf(&crop)).is_err(), "crop");
    let mut huge = packed(Data::Msb);
    huge.data = Data::Strips(vec![vec![0; 16]]);
    huge.width = 40_000;
    assert!(read(&orf(&huge)).is_err(), "width");
}

#[test]
fn orf_matrix_renders_as_the_camera_meant() {
    let o = packed(Data::Msb);
    let color = read(&orf(&o)).unwrap().color;
    let t = as_shot_transform(&color).unwrap();
    let [red, blue] = o.as_shot.unwrap().map(f64::from);
    let neutral = [256.0 / red, 1.0, 256.0 / blue];
    let render = |camera: [f64; 3]| {
        let balanced: [f64; 3] = std::array::from_fn(|i| camera[i] * t.gains[i]);
        chroma::mul_vec(&t.matrix, balanced)
    };
    let white = render(neutral.map(|v| v / neutral.iter().fold(0.0f64, |a, &b| a.max(b))));
    assert!(white.iter().all(|v| (v - 1.0).abs() < 0.002), "{white:?}");

    // A white-balanced colour, through the stored matrix, as sRGB in ProPhoto.
    let balanced = [0.5, 0.3, 0.2];
    let m: [f64; 3] = std::array::from_fn(|r| {
        let row = &o.matrix[3 * r..][..3];
        let sum: i32 = row.iter().map(|&v| v as i32).sum();
        (0..3).map(|c| row[c] as f64 * balanced[c]).sum::<f64>() / sum as f64
    });
    let srgb = chroma::rgb_to_xyz(
        [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]],
        [0.31271, 0.32902],
    );
    let d50 = chroma::bradford([0.31271, 0.32902], chroma::D50_XY);
    let xyz = chroma::mul_vec(&d50, chroma::mul_vec(&srgb, m));
    let expected = chroma::mul_vec(&chroma::xyz_d50_to_pro_photo(), xyz);
    let got = render(std::array::from_fn(|i| balanced[i] / t.gains[i]));
    for i in 0..3 {
        assert!(
            (got[i] - expected[i]).abs() < 0.002,
            "{got:?} != {expected:?}"
        );
    }
}

#[test]
fn orf_daylight_balance_sets_the_temperature() {
    let balance = [554, 464];
    let cases = [
        (Note::Olympus, Some(balance), 6504.0),
        (Note::OlympRawInfo, Some(balance), 5503.0),
        (Note::Olympus, None, 6504.0),
    ];
    for (note, daylight, kelvin) in cases {
        let mut o = packed(Data::Msb);
        o.note = note;
        o.as_shot = Some(balance);
        o.daylight = daylight;
        let color = read(&orf(&o)).unwrap().color;
        let got = as_shot_transform(&color).unwrap().white_balance.temperature;
        assert!(
            (got - kelvin).abs() < 100.0,
            "{note:?}: {got} K, not {kelvin}"
        );
    }

    let mut none = packed(Data::Msb);
    none.as_shot = None;
    none.daylight = None;
    let bytes = orf(&none);
    assert_eq!(read(&bytes).unwrap().color.color_matrix, [None, None]);
    assert!(matches!(
        decode_raw_source(&bytes, &ImageBudget::default()),
        Err(PsdError::Unsupported(_))
    ));
}

#[test]
fn orf_corpus_reads_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP orf corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let mut seen = 0;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let wanted = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("orf") || e.eq_ignore_ascii_case("ori"));
        if !wanted {
            continue;
        }
        seen += 1;
        let name = path.display();
        let bytes = std::fs::read(&path).expect("corpus file");
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Orf), "{name}");
        let image = match read(&bytes) {
            Ok(image) => image,
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let mut sum = [0f64; 4];
        for (i, &v) in image.samples.iter().enumerate() {
            sum[(i / image.width % 2) * 2 + i % image.width % 2] += v as f64;
        }
        let greens: Vec<f64> = (0..4)
            .filter(|&p| image.cfa.colors[p] == 1)
            .map(|p| sum[p])
            .collect();
        let first = image.samples[0];
        if image.samples.iter().all(|&v| v == first) {
            failures.push(format!("{name}: flat"));
        }
        if (greens[0] - greens[1]).abs() > 0.05 * greens[0].max(greens[1]) {
            failures.push(format!("{name}: greens disagree, {sum:?}"));
        }
        let colour = as_shot_transform(&image.color);
        if image.color.as_shot_neutral.is_some() && colour.is_err() {
            failures.push(format!("{name}: no colour"));
        }
        let clipped = image.samples.iter().filter(|&&v| v == 65535).count();
        eprintln!(
            "{name}: {} x {} {:?} crop {:?} clipped {clipped} as shot {:?}",
            image.width,
            image.height,
            image.model,
            image.default_crop,
            colour.map(|t| t.white_balance).ok(),
        );
    }
    eprintln!("{seen} files");
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .ORF or .ORI file");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
