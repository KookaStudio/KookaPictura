//! Linear DNGs (#402): `read_linear_dng` (`codec/dng-raw-reader`) and the
//! linear path of `decode_raw_source` (`codec/raw-develop`), on synthetic
//! DNGs written by `tests/camera_raw/dng/builder.rs` with three samples per pixel.

use pictura_codec::{
    as_shot_transform, decode_raw_source, read_dng, read_linear_dng, DngCrop, ImageBudget, PsdError,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use builder::{dng, noise_plane, Codec, Options, Plane, Val};

const LINEAR_RAW: u16 = 34892;

fn budget() -> ImageBudget {
    ImageBudget::default()
}

/// A linear DNG of `width` pixels whose plane holds `width × 3` samples a row.
fn linear(plane: &Plane, width: usize, mut opts: Options) -> Vec<u8> {
    let bits = match opts.codec {
        Codec::Raw { bits } => bits,
        Codec::Ljpeg { bits, .. } => bits as u16,
    };
    let mut tags = vec![
        (256, Val::Long(vec![width as u32])),
        (258, Val::Short(vec![bits; 3])),
        (262, Val::Short(vec![LINEAR_RAW])),
        (277, Val::Short(vec![3])),
    ];
    if let Some((tw, _)) = opts.tile {
        tags.push((322, Val::Long(vec![(tw / 3) as u32])));
    }
    tags.append(&mut opts.raw_tags);
    opts.raw_tags = tags;
    dng(plane, opts)
}

fn levelled(v: u16, black: f64, white: f64) -> u16 {
    ((v as f64 - black) * 65535.0 / (white - black))
        .round()
        .clamp(0.0, 65535.0) as u16
}

// dng_linear_read
#[test]
fn linear_dng_normalizes_per_sample() {
    let plane = noise_plane(12 * 3, 8, 4095, 7);
    let bytes = linear(
        &plane,
        12,
        Options {
            raw_tags: vec![
                (50714, Val::Short(vec![100, 200, 300])),
                (50717, Val::Short(vec![4095])),
                (50829, Val::Long(vec![1, 2, 7, 12])),
            ],
            ..Options::default()
        },
    );
    let read = read_linear_dng(&bytes, &budget()).expect("linear DNG reads");
    assert_eq!((read.camera.width, read.camera.height), (10, 6));
    let black = [100.0, 200.0, 300.0];
    for y in 0..6 {
        for x in 0..10 {
            for (c, &black) in black.iter().enumerate() {
                let v = plane.samples[(y + 1) * 36 + (x + 2) * 3 + c];
                assert_eq!(
                    read.camera.rgb[(y * 10 + x) * 3 + c],
                    levelled(v, black, 4095.0),
                    "pixel ({x}, {y}) sample {c}"
                );
            }
        }
    }
}

// dng_linear_read_ljpeg
#[test]
fn linear_dng_reads_ljpeg_tiles() {
    let plane = noise_plane(20 * 3, 10, 65535, 8);
    let bytes = linear(
        &plane,
        20,
        Options {
            codec: Codec::Ljpeg {
                bits: 16,
                predictor: 1,
                components: 3,
                restart: 0,
            },
            tile: Some((16 * 3, 16)),
            ..Options::default()
        },
    );
    let read = read_linear_dng(&bytes, &budget()).expect("tiled linear DNG reads");
    assert_eq!((read.camera.width, read.camera.height), (20, 10));
    assert_eq!(read.camera.rgb, plane.samples);
}

// dng_linear_refuses
#[test]
fn linear_dng_refuses_unsupported_forms() {
    let plane = noise_plane(12 * 3, 8, 4095, 9);
    let four = linear(
        &plane,
        9,
        Options {
            raw_tags: vec![(277, Val::Short(vec![4]))],
            ..Options::default()
        },
    );
    let deltas = linear(
        &plane,
        12,
        Options {
            raw_tags: vec![(50716, Val::Rational(vec![(1, 1); 8]))],
            ..Options::default()
        },
    );
    let cfa = dng(&noise_plane(12, 8, 4095, 10), Options::default());
    for bytes in [&four, &deltas, &cfa] {
        assert!(matches!(
            read_linear_dng(bytes, &budget()),
            Err(PsdError::Unsupported(_))
        ));
    }
    let plain = linear(&plane, 12, Options::default());
    assert!(matches!(
        read_dng(&plain, &budget()),
        Err(PsdError::Unsupported(_))
    ));
}

// raw_source_linear
#[test]
fn linear_dng_decodes_without_demosaic() {
    let plane = noise_plane(16 * 3, 12, 65535, 11);
    let matrix: Vec<(i32, i32)> = [
        (8, 10),
        (-2, 10),
        (0, 10),
        (-3, 10),
        (12, 10),
        (1, 10),
        (0, 10),
        (2, 10),
        (6, 10),
    ]
    .to_vec();
    let bytes = linear(
        &plane,
        16,
        Options {
            raw_tags: vec![
                (50719, Val::Rational(vec![(1, 1), (2, 1)])),
                (50720, Val::Rational(vec![(14, 1), (9, 1)])),
            ],
            ifd0_tags: vec![
                (50721, Val::SRational(matrix)),
                (50778, Val::Short(vec![21])),
                (50728, Val::Rational(vec![(1, 2), (1, 1), (3, 4)])),
            ],
            ..Options::default()
        },
    );
    let source = decode_raw_source(&bytes, &budget()).expect("linear DNG decodes");
    assert_eq!((source.camera.width, source.camera.height), (16, 12));
    assert_eq!(source.camera.rgb, plane.samples);
    assert_eq!(
        source.look.crop,
        DngCrop {
            x: 1.0,
            y: 2.0,
            width: 14.0,
            height: 9.0
        }
    );
    let read = read_linear_dng(&bytes, &budget()).unwrap();
    assert_eq!(
        source.as_shot,
        as_shot_transform(&read.color).unwrap().white_balance
    );
}
