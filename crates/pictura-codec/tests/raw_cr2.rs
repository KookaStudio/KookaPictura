//! The vendor raw entry point and the Canon CR2 reader
//! (`codec/vendor-raw-containers`, #399) against synthetic CR2s written by
//! `tests/raw/cr2_builder.rs`, and, opt-in, real camera files.

use pictura_codec::{
    demosaic, read_dng, read_raw, sniff_raw, DemosaicMethod, DngImage, ImageBudget, PsdError,
    RawFormat,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "raw/cr2_builder.rs"]
mod cr2_builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use builder::{dng, noise_plane, Codec, Options, Plane, Val};
use cr2_builder::{cr2, subsample, Cr2};

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn scaled(v: u16, white: f64, black: f64) -> u16 {
    ((v as f64 - black) * 65535.0 / (white - black))
        .round()
        .clamp(0.0, 65535.0) as u16
}

fn plane_14bit() -> Plane {
    noise_plane(64, 20, 16383, 7)
}

#[test]
fn raw_containers_dispatch_by_signature() {
    let plane = noise_plane(32, 16, 4095, 3);
    let dng_bytes = dng(
        &plane,
        Options {
            codec: Codec::Ljpeg {
                bits: 12,
                predictor: 1,
                components: 2,
                restart: 0,
            },
            ..Options::default()
        },
    );
    assert_eq!(sniff_raw(&dng_bytes), Some(RawFormat::Dng));
    let via_raw = read(&dng_bytes).expect("DNG through read_raw");
    let direct = read_dng(&dng_bytes, &ImageBudget::default()).expect("DNG directly");
    assert_eq!(via_raw.samples, direct.samples);
    assert_eq!(via_raw.cfa, direct.cfa);

    let cr2_bytes = cr2(&Cr2::new(noise_plane(32, 16, 4095, 4), 12));
    assert_eq!(sniff_raw(&cr2_bytes), Some(RawFormat::Cr2));
    read(&cr2_bytes).expect("CR2 through read_raw");

    let tiff = dng(
        &plane,
        Options {
            dng_version: false,
            ..Options::default()
        },
    );
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    for other in [tiff, png] {
        assert_eq!(sniff_raw(&other), None);
        assert!(matches!(read(&other), Err(PsdError::Unsupported(_))));
    }
}

#[test]
fn sliced_cr2_decodes_exactly() {
    let plane = plane_14bit();
    let expected: Vec<u16> = plane
        .samples
        .iter()
        .map(|&v| scaled(v, 16383.0, 0.0))
        .collect();
    let mut spec = Cr2::new(plane, 14);
    spec.slices = Some([2, 24, 16]);
    let image = read(&cr2(&spec)).expect("sliced CR2 reads");
    assert_eq!((image.width, image.height), (64, 20));
    assert_eq!(image.samples, expected);
}

#[test]
fn frame_rows_holding_two_sensor_rows_decode() {
    let plane = plane_14bit();
    let expected: Vec<u16> = plane
        .samples
        .iter()
        .map(|&v| scaled(v, 16383.0, 0.0))
        .collect();
    let mut spec = Cr2::new(plane, 14);
    spec.components = 4;
    spec.rows_per_frame_row = 2;
    spec.slices = Some([1, 32, 32]);
    let image = read(&cr2(&spec)).expect("two sensor rows per frame row");
    assert_eq!((image.width, image.height), (64, 20));
    assert_eq!(image.samples, expected);
}

#[test]
fn unsliced_component_counts_agree() {
    let expected: Vec<u16> = plane_14bit()
        .samples
        .iter()
        .map(|&v| scaled(v, 16383.0, 0.0))
        .collect();
    for components in [1, 2, 4] {
        let mut spec = Cr2::new(plane_14bit(), 14);
        spec.components = components;
        let image = read(&cr2(&spec)).unwrap_or_else(|e| panic!("{components}: {e}"));
        assert_eq!(image.samples, expected, "{components} components");
    }
}

#[test]
fn cr2_variants_are_refused() {
    let mut small = cr2(&Cr2::new(plane_14bit(), 14));
    subsample(&mut small);
    assert!(
        matches!(read(&small), Err(PsdError::Unsupported(_))),
        "{:?}",
        read(&small).err()
    );

    let mut spec = Cr2::new(plane_14bit(), 14);
    spec.compression = 1;
    assert!(matches!(read(&cr2(&spec)), Err(PsdError::Unsupported(_))));
}

#[test]
fn masked_border_sets_the_black() {
    let (w, h, left, top) = (48usize, 24usize, 8usize, 4usize);
    let blacks = [100u16, 200, 300, 400];
    let active = noise_plane(w, h, 3595, 11);
    let mut samples = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            samples[y * w + x] = if x < left {
                let phase = ((y as isize - top as isize).rem_euclid(2) * 2
                    + (x as isize - left as isize).rem_euclid(2))
                    as usize;
                blacks[phase]
            } else {
                active.samples[y * w + x] + 500
            };
        }
    }
    let plane = Plane {
        width: w,
        height: h,
        samples: samples.clone(),
    };
    let mut spec = Cr2::new(plane, 12);
    spec.active = Some([left as u16, top as u16, (w - 1) as u16, (h - 1) as u16]);
    let image = read(&cr2(&spec)).expect("masked CR2 reads");
    assert_eq!((image.width, image.height), (w - left, h - top));
    for y in 0..image.height {
        for x in 0..image.width {
            let black = blacks[(y % 2) * 2 + x % 2] as f64;
            let v = samples[(y + top) * w + x + left];
            assert_eq!(
                image.samples[y * image.width + x],
                scaled(v, 4095.0, black),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn cr2_reports_camera_tags() {
    let mut spec = Cr2::new(noise_plane(40, 24, 4095, 5), 12);
    spec.active = Some([4, 2, 39, 23]);
    spec.ifd0_tags = vec![
        (271, Val::Ascii("Canon")),
        (272, Val::Ascii("Canon EOS Synthetic")),
        (274, Val::Short(vec![6])),
    ];
    let image = read(&cr2(&spec)).expect("tagged CR2 reads");
    assert_eq!(image.make.as_deref(), Some("Canon"));
    assert_eq!(image.model.as_deref(), Some("Canon EOS Synthetic"));
    assert_eq!(image.orientation, 6);
    assert_eq!((image.cfa.width, image.cfa.height), (2, 2));
    assert_eq!(image.cfa.colors, vec![0, 1, 1, 2]);
    let crop = image.default_crop;
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (0.0, 0.0, 36.0, 22.0)
    );
    assert_eq!(image.color, Default::default());
    let rgb = demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default())
        .expect("the demosaic takes a CR2");
    assert_eq!((rgb.width, rgb.height), (36, 22));

    // RGGB sits at the sensor corner: an odd top or left shifts the phase.
    for (left, top, colors) in [
        (5, 2, [1, 0, 2, 1]),
        (4, 3, [1, 2, 0, 1]),
        (5, 3, [2, 1, 1, 0]),
    ] {
        let mut spec = Cr2::new(noise_plane(40, 24, 4095, 5), 12);
        spec.active = Some([left, top, 39, 23]);
        let image = read(&cr2(&spec)).expect("offset CR2 reads");
        assert_eq!(image.cfa.colors, colors, "active corner {left}, {top}");
    }
}

#[test]
fn hostile_cr2s_fail_cleanly() {
    let valid = cr2(&Cr2::new(noise_plane(32, 8, 4095, 9), 12));
    read(&valid).expect("the uncut file reads");
    for len in 0..valid.len() {
        assert!(read(&valid[..len]).is_err(), "prefix of {len} bytes");
    }

    let mut spec = Cr2::new(plane_14bit(), 14);
    spec.slices = Some([2, 24, 16]);
    let mut bytes = cr2(&spec);
    // Rewrite the last slice width (16 → 8) in place: 2·24 + 8 ≠ 64.
    let tag = bytes
        .windows(4)
        .position(|w| w == [0x40, 0xC6, 3, 0])
        .expect("slice tag");
    let at = u32::from_le_bytes(bytes[tag + 8..tag + 12].try_into().unwrap()) as usize;
    bytes[at + 4..at + 6].copy_from_slice(&8u16.to_le_bytes());
    assert!(read(&bytes).is_err(), "slices that miss the width");

    let mut spec = Cr2::new(plane_14bit(), 14);
    spec.active = Some([0, 0, 70, 19]);
    assert!(read(&cr2(&spec)).is_err(), "active area past the plane");

    let small = ImageBudget {
        max_dimension: 32,
        ..ImageBudget::default()
    };
    assert!(
        read_raw(&cr2(&Cr2::new(plane_14bit(), 14)), &small).is_err(),
        "wider than the budget"
    );
}

#[test]
fn cr2_corpus_reads_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let is_cr2 = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cr2"));
        if !is_cr2 {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            sniff_raw(&bytes),
            Some(RawFormat::Cr2),
            "{}",
            path.display()
        );
        let image = read(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let first = image.samples[0];
        assert!(
            image.samples.iter().any(|&v| v != first),
            "{} is flat",
            path.display()
        );
        // Per 2×2 position: the two greens of a correct phase have nearly
        // equal means; clipped counts how many samples reach 65535.
        let mut sum = [0f64; 4];
        let mut n = [0f64; 4];
        for (i, &v) in image.samples.iter().enumerate() {
            let p = (i / image.width % 2) * 2 + i % image.width % 2;
            sum[p] += v as f64;
            n[p] += 1.0;
        }
        let means: Vec<u32> = (0..4).map(|p| (sum[p] / n[p]) as u32).collect();
        let greens: Vec<f64> = (0..4)
            .filter(|&p| image.cfa.colors[p] == 1)
            .map(|p| sum[p] / n[p])
            .collect();
        assert!(
            (greens[0] - greens[1]).abs() <= 0.05 * greens[0].max(greens[1]),
            "{}: the reported green positions disagree, {means:?}",
            path.display()
        );
        let max = image.samples.iter().copied().max().unwrap_or(0);
        let clipped = image.samples.iter().filter(|&&v| v == 65535).count();
        eprintln!(
            "{}: {} × {} {:?} phase means {means:?} max {max} clipped {clipped}",
            path.display(),
            image.width,
            image.height,
            image.model
        );
        seen += 1;
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .CR2 file");
}
