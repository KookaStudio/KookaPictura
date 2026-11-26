//! The in-house DNG reader (`codec/dng-raw-reader`, #396) against synthetic
//! DNGs written by `tests/camera_raw/dng/builder.rs`, and, opt-in, real camera files.

use pictura_codec::{read_dng, DngImage, ImageBudget, PsdError};

#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use builder::{dng, noise_plane, Codec, Options, Plane, Val};

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_dng(bytes, &ImageBudget::default())
}

fn scaled(v: u16, white: f64, black: f64) -> u16 {
    ((v as f64 - black) * 65535.0 / (white - black))
        .round()
        .clamp(0.0, 65535.0) as u16
}

#[test]
fn uncompressed_strip_decodes_exactly() {
    let plane = noise_plane(64, 48, 65535, 1);
    let bytes = dng(
        &plane,
        Options {
            rows_per_strip: Some(10),
            ..Options::default()
        },
    );
    let image = read(&bytes).expect("uncompressed DNG reads");
    assert_eq!((image.width, image.height), (64, 48));
    assert_eq!(image.samples, plane.samples);
    assert_eq!((image.cfa.width, image.cfa.height), (2, 2));
    assert_eq!(image.cfa.colors, vec![0, 1, 1, 2]);
    assert_eq!(image.orientation, 1);
}

#[test]
fn bit_packed_depths_decode() {
    for bits in [8u16, 10, 12, 14] {
        let white = (1u32 << bits) - 1;
        let plane = noise_plane(37, 9, white as u16, bits as u64);
        let bytes = dng(
            &plane,
            Options {
                codec: Codec::Raw { bits },
                rows_per_strip: Some(4),
                ..Options::default()
            },
        );
        let image = read(&bytes).unwrap_or_else(|e| panic!("{bits}-bit: {e}"));
        let expected: Vec<u16> = plane
            .samples
            .iter()
            .map(|&v| scaled(v, white as f64, 0.0))
            .collect();
        assert_eq!(image.samples, expected, "{bits}-bit");
    }
}

#[test]
fn lossless_jpeg_tiles_match_uncompressed() {
    let plane = noise_plane(70, 50, 16383, 7);
    let reference = read(&dng(
        &plane,
        Options {
            codec: Codec::Raw { bits: 16 },
            raw_tags: vec![(50717, Val::Short(vec![16383]))],
            ..Options::default()
        },
    ))
    .expect("reference reads")
    .samples;
    let mut cases = Vec::new();
    for predictor in 1..=7 {
        for components in [1, 2] {
            cases.push((predictor, components, 0));
        }
    }
    // A restart interval of two frame rows, one and two components.
    cases.push((1, 1, 64));
    cases.push((6, 2, 32));
    for (predictor, components, restart) in cases {
        let bytes = dng(
            &plane,
            Options {
                codec: Codec::Ljpeg {
                    bits: 14,
                    predictor,
                    components,
                    restart,
                },
                tile: Some((32, 32)),
                ..Options::default()
            },
        );
        let image = read(&bytes).unwrap_or_else(|e| {
            panic!("predictor {predictor}, {components} components, restart {restart}: {e}")
        });
        assert_eq!(
            image.samples, reference,
            "predictor {predictor}, {components} components, restart {restart}"
        );
    }

    // 16-bit samples with a 32768 step use difference category 16.
    let samples = (0..32 * 4)
        .map(|i| if (i / 3) % 2 == 0 { 0 } else { 32768 })
        .collect();
    let plane = Plane {
        width: 32,
        height: 4,
        samples,
    };
    let bytes = dng(
        &plane,
        Options {
            codec: Codec::Ljpeg {
                bits: 16,
                predictor: 1,
                components: 1,
                restart: 0,
            },
            ..Options::default()
        },
    );
    assert_eq!(
        read(&bytes).expect("category 16 reads").samples,
        plane.samples
    );
}

#[test]
fn byte_orders_and_bigtiff_agree() {
    let plane = noise_plane(40, 20, 65535, 3);
    for codec in [
        Codec::Raw { bits: 16 },
        Codec::Ljpeg {
            bits: 16,
            predictor: 4,
            components: 2,
            restart: 0,
        },
    ] {
        for (big_endian, bigtiff) in [(false, false), (true, false), (false, true), (true, true)] {
            let bytes = dng(
                &plane,
                Options {
                    codec,
                    tile: Some((16, 16)),
                    big_endian,
                    bigtiff,
                    in_subifd: true,
                    ..Options::default()
                },
            );
            let image = read(&bytes)
                .unwrap_or_else(|e| panic!("big-endian {big_endian}, BigTIFF {bigtiff}: {e}"));
            assert_eq!(
                image.samples, plane.samples,
                "big-endian {big_endian}, BigTIFF {bigtiff}"
            );
        }
    }
}

#[test]
fn black_pattern_and_deltas_normalize() {
    let (w, h) = (20usize, 16usize);
    let plane = noise_plane(w, h, 4095, 11);
    let black = [256.1, 260.0, 257.5, 258.5];
    let delta_v: Vec<(i32, i32)> = (0..h as i32).map(|y| (y - 8, 4)).collect();
    let delta_h: Vec<(i32, i32)> = (0..w as i32).map(|x| (3 - x % 7, 2)).collect();
    let bytes = dng(
        &plane,
        Options {
            codec: Codec::Raw { bits: 12 },
            raw_tags: vec![
                (50713, Val::Short(vec![2, 2])),
                (
                    50714,
                    Val::Rational(vec![(2561, 10), (260, 1), (2575, 10), (2585, 10)]),
                ),
                (50715, Val::SRational(delta_h.clone())),
                (50716, Val::SRational(delta_v.clone())),
                (50717, Val::Short(vec![4095])),
            ],
            ..Options::default()
        },
    );
    let image = read(&bytes).expect("levels DNG reads");
    for y in 0..h {
        for x in 0..w {
            let b = black[(y % 2) * 2 + x % 2]
                + delta_v[y].0 as f64 / delta_v[y].1 as f64
                + delta_h[x].0 as f64 / delta_h[x].1 as f64;
            let expected = scaled(plane.samples[y * w + x], 4095.0, b);
            assert_eq!(image.samples[y * w + x], expected, "({x}, {y})");
        }
    }
}

#[test]
fn linearization_table_applies() {
    let plane = Plane {
        width: 16,
        height: 16,
        samples: (0..256).collect(),
    };
    let table: Vec<u16> = (0..256u32).map(|i| (i * i / 4) as u16).collect();
    let bytes = dng(
        &plane,
        Options {
            codec: Codec::Raw { bits: 8 },
            raw_tags: vec![
                (50712, Val::Short(table.clone())),
                (50714, Val::Short(vec![44])),
                (50717, Val::Short(vec![16383])),
            ],
            ..Options::default()
        },
    );
    let image = read(&bytes).expect("linearized DNG reads");
    let expected: Vec<u16> = (0..256).map(|i| scaled(table[i], 16383.0, 44.0)).collect();
    assert_eq!(image.samples, expected);
}

#[test]
fn active_area_crops_the_plane() {
    let (w, h) = (40usize, 30usize);
    let plane = noise_plane(w, h, 65535, 5);
    let black = [10.0, 20.0, 30.0, 40.0];
    let bytes = dng(
        &plane,
        Options {
            raw_tags: vec![
                (33422, Val::Byte(vec![1, 0, 2, 1])),
                (50829, Val::Long(vec![3, 5, 27, 37])),
                (50713, Val::Short(vec![2, 2])),
                (50714, Val::Short(vec![10, 20, 30, 40])),
                (50719, Val::Long(vec![2, 2])),
                (50720, Val::Long(vec![28, 20])),
            ],
            ..Options::default()
        },
    );
    let image = read(&bytes).expect("active-area DNG reads");
    assert_eq!((image.width, image.height), (32, 24));
    assert_eq!(image.cfa.colors, vec![1, 0, 2, 1]);
    assert_eq!(
        (
            image.default_crop.x,
            image.default_crop.y,
            image.default_crop.width,
            image.default_crop.height
        ),
        (2.0, 2.0, 28.0, 20.0)
    );
    for y in 0..24 {
        for x in 0..32 {
            let v = plane.samples[(3 + y) * w + 5 + x];
            let expected = scaled(v, 65535.0, black[(y % 2) * 2 + x % 2]);
            assert_eq!(image.samples[y * 32 + x], expected, "({x}, {y})");
        }
    }
}

#[test]
fn color_tags_are_reported() {
    let color1: Vec<(i32, i32)> = (0..9).map(|i| (i * 1000 - 3000, 10000)).collect();
    let color2: Vec<(i32, i32)> = (0..9).map(|i| (5000 - i * 700, 10000)).collect();
    let forward1: Vec<(i32, i32)> = (0..9).map(|i| (i + 1, 16)).collect();
    let plane = noise_plane(8, 8, 65535, 2);
    let bytes = dng(
        &plane,
        Options {
            in_subifd: true,
            ifd0_tags: vec![
                (271, Val::Ascii("Kooka")),
                (274, Val::Short(vec![6])),
                (50708, Val::Ascii("Test Camera")),
                (50721, Val::SRational(color1.clone())),
                (50722, Val::SRational(color2.clone())),
                (50964, Val::SRational(forward1.clone())),
                (50728, Val::Rational(vec![(1, 2), (1, 1), (3, 4)])),
                (50730, Val::SRational(vec![(-1, 2)])),
                (50778, Val::Short(vec![17])),
                (50779, Val::Short(vec![21])),
            ],
            ..Options::default()
        },
    );
    let image = read(&bytes).expect("colour-tag DNG reads");
    let matrix =
        |v: &[(i32, i32)]| -> [f64; 9] { std::array::from_fn(|i| v[i].0 as f64 / v[i].1 as f64) };
    let c = &image.color;
    assert_eq!(
        c.color_matrix,
        [Some(matrix(&color1)), Some(matrix(&color2))]
    );
    assert_eq!(c.forward_matrix, [Some(matrix(&forward1)), None]);
    assert_eq!(c.camera_calibration, [None, None]);
    assert_eq!(c.calibration_illuminant, [Some(17), Some(21)]);
    assert_eq!(c.as_shot_neutral, Some([0.5, 1.0, 0.75]));
    assert_eq!(c.analog_balance, None);
    assert_eq!(c.baseline_exposure, -0.5);
    assert_eq!(image.orientation, 6);
    assert_eq!(image.make.as_deref(), Some("Kooka"));
    assert_eq!(image.model.as_deref(), Some("Test Camera"));
}

#[test]
fn deferred_features_are_refused() {
    let plane = noise_plane(8, 8, 65535, 4);
    let unsupported = [
        vec![(259, Val::Short(vec![8]))],
        vec![(259, Val::Short(vec![32773]))],
        vec![(259, Val::Short(vec![34892]))],
        vec![(262, Val::Short(vec![34892]))],
        vec![(339, Val::Short(vec![3]))],
        vec![
            (50710, Val::Byte(vec![3, 4, 5, 1])),
            (33422, Val::Byte(vec![0, 1, 2, 3])),
        ],
    ];
    for tags in unsupported {
        let bytes = dng(
            &plane,
            Options {
                raw_tags: tags,
                ..Options::default()
            },
        );
        assert!(
            matches!(read(&bytes), Err(PsdError::Unsupported(_))),
            "expected Unsupported, got {:?}",
            read(&bytes).map(|_| ())
        );
    }
    let bytes = dng(
        &plane,
        Options {
            dng_version: false,
            ..Options::default()
        },
    );
    assert!(matches!(read(&bytes), Err(PsdError::Invalid(_))));
}

#[test]
fn hostile_dngs_fail_cleanly() {
    let plane = noise_plane(16, 16, 16383, 9);
    let ljpeg = |pad_rows, raw_tags| {
        dng(
            &plane,
            Options {
                codec: Codec::Ljpeg {
                    bits: 14,
                    predictor: 1,
                    components: 1,
                    restart: 0,
                },
                tile: Some((16, 16)),
                ljpeg_pad_rows: pad_rows,
                raw_tags,
                ..Options::default()
            },
        )
    };
    let huge = dng(
        &plane,
        Options {
            raw_tags: vec![
                (256, Val::Long(vec![200_000])),
                (257, Val::Long(vec![200_000])),
            ],
            ..Options::default()
        },
    );
    let past_end = dng(
        &plane,
        Options {
            raw_tags: vec![(273, Val::Long(vec![10_000_000]))],
            ..Options::default()
        },
    );
    // Two bytes over the one-bit-per-sample floor, far short of the stream.
    let truncated = ljpeg(0, vec![(325, Val::Long(vec![34]))]);
    let oversize = ljpeg(4, Vec::new());
    for (name, bytes) in [
        ("huge", &huge),
        ("past end", &past_end),
        ("truncated tile", &truncated),
        ("oversize frame", &oversize),
    ] {
        assert!(read(bytes).is_err(), "{name} must fail");
    }

    for valid in [dng(&plane, Options::default()), ljpeg(0, Vec::new())] {
        read(&valid).expect("the uncut file reads");
        for len in 0..valid.len() {
            assert!(read(&valid[..len]).is_err(), "prefix of {len} bytes");
        }
    }
}

#[test]
fn leica_samples_decode_when_present() {
    let Some(root) = std::env::var_os("PICTURA_DNG_CORPUS") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    // Spot samples `(x, y, normalized)` from a separate Python parse of each
    // file: byte at the strip offset, LinearizationTable, black 44, white 16383.
    type Spot = (usize, usize, u16);
    let files: [(&str, [Spot; 7]); 2] = [
        (
            "L1.dng",
            [
                (0, 0, 1051),
                (1, 0, 2134),
                (0, 1, 2230),
                (1, 1, 1271),
                (2608, 1736, 666),
                (5215, 3471, 5022),
                (1000, 2000, 2856),
            ],
        ),
        (
            "L2.dng",
            [
                (0, 0, 7240),
                (1, 0, 17295),
                (0, 1, 17031),
                (1, 1, 14263),
                (2608, 1736, 3931),
                (5215, 3471, 1195),
                (1000, 2000, 3080),
            ],
        ),
    ];
    for (name, spots) in files {
        let path = root.join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let image = read(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((image.width, image.height), (5216, 3472), "{name}");
        assert_eq!(image.cfa.colors, vec![0, 1, 1, 2], "{name}");
        let crop = image.default_crop;
        assert_eq!(
            (crop.x, crop.y, crop.width, crop.height),
            (2.0, 2.0, 5212.0, 3468.0),
            "{name}"
        );
        assert_eq!(image.color.calibration_illuminant, [Some(17), Some(21)]);
        assert_eq!(image.make.as_deref(), Some("Leica Camera AG"));
        // The model string is the camera's, "M9 Digital Camera".
        assert!(image
            .model
            .as_deref()
            .is_some_and(|m| m.ends_with(" Digital Camera")));
        for (x, y, expected) in spots {
            assert_eq!(image.samples[y * 5216 + x], expected, "{name} ({x}, {y})");
        }
    }
}
