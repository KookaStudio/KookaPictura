//! The Fujifilm RAF reader (`codec/vendor-raw-containers`, #1473) against
//! synthetic RAFs written by `tests/camera_raw/raw/raf_builder.rs`, and,
//! opt-in, real camera files.

use pictura_codec::{
    demosaic, read_dng, read_raw, sniff_raw, sniff_raw_prefix, DemosaicMethod, DngImage,
    ImageBudget, PsdError, RawFormat,
};

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
use raf_builder::{raf, Coding, Exif, Raf, EXR, X100S_CFA};

const WIDTH: usize = 40;
const HEIGHT: usize = 24;

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn samples(width: usize, max: u16) -> Vec<u16> {
    noise_plane(width, HEIGHT, max, 33).samples
}

fn file(coding: Coding) -> Raf {
    let max = if matches!(coding, Coding::Lsb12) {
        4095
    } else {
        16383
    };
    Raf::new(WIDTH, HEIGHT, samples(WIDTH, max), coding)
}

/// The samples normalized by the black at each photosite's position.
fn normalized(r: &Raf) -> Vec<u16> {
    let side = match r.black.len() {
        1 => 1,
        4 => 2,
        _ => 6,
    };
    let white = ((1u32 << r.bits) - 1) as f64;
    r.samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let (x, y) = (i % r.width, i / r.width);
            let black = r.black[(y % side) * side + x % side] as f64;
            ((v as f64 - black) * 65535.0 / (white - black))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

fn unsupported(r: &Raf) -> String {
    match read(&raf(r)) {
        Err(PsdError::Unsupported(why)) => why,
        other => panic!("expected unsupported, got {:?}", other.map(|i| i.width)),
    }
}

#[test]
fn raf_is_named_by_its_header() {
    let bytes = raf(&file(Coding::Words));
    assert_eq!(sniff_raw(&bytes), Some(RawFormat::Raf));
    assert_eq!(sniff_raw(&bytes[..16]), Some(RawFormat::Raf));
    assert!(sniff_raw_prefix(&bytes[..16]));
    read(&bytes).expect("RAF through read_raw");
    let mut other = bytes.clone();
    other[15] = b'X';
    assert_eq!(sniff_raw(&other), None);
    assert!(!sniff_raw_prefix(&other[..16]));
}

#[test]
fn raf_words_decode_exactly() {
    for big_endian in [false, true] {
        let mut r = file(Coding::Words);
        r.big_endian = big_endian;
        let image = read(&raf(&r)).expect("words");
        assert_eq!((image.width, image.height), (WIDTH, HEIGHT));
        assert_eq!(image.samples, normalized(&r), "big endian {big_endian}");
    }
}

#[test]
fn packed_rafs_decode_exactly() {
    for coding in [Coding::Lsb12, Coding::Msb14] {
        for width in [WIDTH, WIDTH + 1] {
            for row_pad in [0, 2] {
                let max = if matches!(coding, Coding::Lsb12) {
                    4095
                } else {
                    16383
                };
                let mut r = Raf::new(width, HEIGHT, samples(width, max), coding.clone());
                r.row_pad = row_pad;
                let image = read(&raf(&r)).expect("packed");
                assert_eq!(
                    image.samples,
                    normalized(&r),
                    "{coding:?} width {width} pad {row_pad}"
                );
            }
        }
    }
}

#[test]
fn compressed_rafs_are_refused() {
    let r = file(Coding::Bytes(vec![0x5A; WIDTH * HEIGHT], None));
    assert_eq!(sniff_raw(&raf(&r)), Some(RawFormat::Raf));
    let why = unsupported(&r);
    assert!(why.contains("compressed sensor data"), "{why}");
}

#[test]
fn raf_variants_are_refused() {
    let mut second = file(Coding::Words);
    second.second_frame = true;
    assert!(unsupported(&second).contains("EXR"));

    let mut supercdd = file(Coding::Words);
    supercdd.sensor_tiff = false;
    assert!(unsupported(&supercdd).contains("SuperCCD"));

    let mut exr = file(Coding::Words).bayer();
    exr.layout = EXR;
    unsupported(&exr);

    // 10-bit samples in rows of 13 words.
    let mut ten = file(Coding::Bytes(vec![0; 52 * HEIGHT], Some(13)));
    ten.bits = 10;
    assert!(unsupported(&ten).contains("10-bit"));

    // Rows of 15 words that do not add up to the data.
    let mut short = file(Coding::Bytes(vec![0; 60 * HEIGHT + 4], Some(15)));
    short.bits = 12;
    unsupported(&short);
}

#[test]
fn xtrans_raf_takes_its_pattern_and_levels() {
    let mut r = file(Coding::Words);
    r.black = (0..36).map(|i| 1000 + 2 * i).collect();
    r.crop = Some([3, 5, 18, 30]);
    let image = read(&raf(&r)).expect("X-Trans");
    assert_eq!((image.cfa.width, image.cfa.height), (6, 6));
    assert_eq!(image.cfa.colors, X100S_CFA);
    assert_eq!(image.samples, normalized(&r));
    let crop = image.default_crop;
    assert_eq!(
        [crop.x, crop.y, crop.width, crop.height],
        [5.0, 3.0, 30.0, 18.0]
    );
    let neutral = image.color.as_shot_neutral.expect("neutral");
    let want = [302.0 / 577.0, 1.0, 302.0 / 454.0];
    assert!(neutral.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12));
    assert!((image.color.baseline_exposure - 0.72).abs() < 1e-12);
    for (bias, stops) in [(None, 0.0), (Some((-172, 100)), 1.72), (Some((5, 0)), 0.0)] {
        r.exposure_bias = bias;
        let image = read(&raf(&r)).expect("bias");
        assert!(
            (image.color.baseline_exposure - stops).abs() < 1e-12,
            "{bias:?}"
        );
    }
}

#[test]
fn bayer_raf_is_rggb() {
    let r = file(Coding::Lsb12).bayer();
    let image = read(&raf(&r)).expect("Bayer");
    assert_eq!((image.cfa.width, image.cfa.height), (2, 2));
    assert_eq!(image.cfa.colors, [0, 1, 1, 2]);
    assert_eq!(image.samples, normalized(&r));
    demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default()).expect("demosaic");
}

#[test]
fn raf_reports_camera_tags() {
    let mut r = file(Coding::Words);
    r.exif = Some(Exif {
        make: "FUJIFILM",
        model: "X-T3",
        orientation: 6,
    });
    let image = read(&raf(&r)).unwrap();
    assert_eq!(image.make.as_deref(), Some("FUJIFILM"));
    assert_eq!(image.model.as_deref(), Some("X-T3"));
    assert_eq!(image.orientation, 6);
    demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default()).expect("demosaic");

    r.exif = None;
    let image = read(&raf(&r)).unwrap();
    assert_eq!(image.make.as_deref(), Some("FUJIFILM"));
    assert_eq!(image.model.as_deref(), Some("X100S"));
    assert_eq!(image.orientation, 1);
}

#[test]
fn hostile_rafs_fail_cleanly() {
    let files = [
        file(Coding::Words),
        file(Coding::Lsb12).bayer(),
        file(Coding::Msb14),
        file(Coding::Bytes(vec![0x5A; WIDTH * HEIGHT], None)),
    ];
    for r in &files {
        let bytes = raf(r);
        for len in 0..bytes.len() {
            assert!(read(&bytes[..len]).is_err(), "{:?} cut at {len}", r.coding);
        }
        // The header, the directory and the raw directory, which precede the data.
        let data_at = bytes.len() - (bytes.len() - 0x94).min(2 * WIDTH * HEIGHT);
        for at in (0x10..data_at).step_by(3) {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                broken[at] ^= flip;
                // Any answer but a panic.
                let _ = read(&broken);
            }
        }
    }

    let mut crop = file(Coding::Words);
    crop.crop = Some([8, 0, HEIGHT as u16, WIDTH as u16]);
    assert!(read(&raf(&crop)).is_err(), "crop");
    let mut huge = file(Coding::Bytes(vec![0; 16], None));
    huge.width = 40_000;
    assert!(read(&raf(&huge)).is_err(), "width");
}

fn corpus() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("PICTURA_RAW_CORPUS");
    if dir.is_none() {
        eprintln!("SKIP raf corpus: PICTURA_RAW_CORPUS is not set");
    }
    dir.map(Into::into)
}

fn files_with(dir: &std::path::Path, ext: &str) -> Vec<std::path::PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.expect("corpus entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    out.sort();
    out
}

#[test]
fn raf_matches_its_dng_when_present() {
    let Some(dir) = corpus() else {
        return;
    };
    let stem = "X100S_fujifilm-x100s-daylight-DSCF9505";
    let (raf_path, dng_path) = (
        dir.join(format!("{stem}.RAF")),
        dir.join(format!("{stem}.dng")),
    );
    if !raf_path.exists() || !dng_path.exists() {
        eprintln!("SKIP raf dng pair: {stem}.RAF and .dng are not both in the corpus");
        return;
    }
    let budget = ImageBudget::default();
    let raw = read(&std::fs::read(raf_path).unwrap()).expect("RAF");
    let dng = read_dng(&std::fs::read(dng_path).unwrap(), &budget).expect("DNG");
    // The DNG drops the plane's first four columns.
    let (dx, dy) = (4usize, 0usize);
    let c = dng.default_crop;
    let (x0, y0, w, h) = (
        c.x as usize,
        c.y as usize,
        c.width as usize,
        c.height as usize,
    );
    let rc = raw.default_crop;
    assert_eq!([rc.x as usize, rc.y as usize], [x0 + dx, y0 + dy]);
    assert_eq!([rc.width as usize, rc.height as usize], [w, h]);
    let mut differ = 0usize;
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let (rx, ry) = (x + dx, y + dy);
            let colour = |img: &DngImage, x: usize, y: usize| {
                img.cfa.colors[(y % img.cfa.height) * img.cfa.width + x % img.cfa.width]
            };
            assert_eq!(colour(&dng, x, y), colour(&raw, rx, ry), "CFA at {x},{y}");
            let (a, b) = (
                dng.samples[y * dng.width + x],
                raw.samples[ry * raw.width + rx],
            );
            differ += usize::from(a.abs_diff(b) > 1);
        }
    }
    assert_eq!(differ, 0, "samples differing by more than rounding");
    let (a, b) = (
        raw.color.as_shot_neutral.unwrap(),
        dng.color.as_shot_neutral.unwrap(),
    );
    assert!(
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5),
        "{a:?} vs {b:?}"
    );
}

#[test]
fn raf_corpus_reads_when_present() {
    let Some(dir) = corpus() else {
        return;
    };
    let budget = ImageBudget::default();
    let (mut read_ok, mut refused) = (0, 0);
    let mut failures = Vec::new();
    for path in files_with(&dir, "raf") {
        let name = path.display();
        let bytes = std::fs::read(&path).expect("corpus file");
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Raf), "{name}");
        let image = match read(&bytes) {
            Ok(image) => image,
            Err(PsdError::Unsupported(why))
                if ["compressed sensor data", "SuperCCD", "EXR"]
                    .iter()
                    .any(|k| why.contains(k)) =>
            {
                eprintln!("{name}: refused, {why}");
                refused += 1;
                continue;
            }
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let first = image.samples[0];
        if image.samples.iter().all(|&v| v == first) {
            failures.push(format!("{name}: flat"));
        }
        match demosaic(&image, DemosaicMethod::Ahd, &budget) {
            Ok(rgb) => {
                let (w, cfa) = (image.width, &image.cfa);
                let kept = image.samples.iter().enumerate().all(|(i, &v)| {
                    let c = cfa.colors[(i / w % cfa.height) * cfa.width + i % w % cfa.width];
                    rgb.rgb[i * 3 + c as usize] == v
                });
                if !kept {
                    failures.push(format!("{name}: demosaic changed a sample"));
                }
            }
            Err(error) => failures.push(format!("{name}: demosaic {error}")),
        }
        eprintln!(
            "{name}: {} x {} {:?} {}x{} CFA crop {:?}",
            image.width,
            image.height,
            image.model,
            image.cfa.width,
            image.cfa.height,
            image.default_crop,
        );
        read_ok += 1;
    }
    eprintln!("{read_ok} read, {refused} refused");
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(
        read_ok + refused > 0,
        "PICTURA_RAW_CORPUS holds no .RAF file"
    );
}
