//! The Canon CR3 reader and its CRX decoder (`codec/vendor-raw-containers`,
//! `codec/camera-matrices`, #1474) against synthetic CR3s written by
//! `tests/raw/cr3_builder.rs` and `tests/raw/crx_encode.rs`, and, opt-in, real
//! camera files.

use pictura_codec::{
    as_shot_transform, decode_raw_source, demosaic, read_raw, sniff_raw, sniff_raw_prefix,
    DemosaicMethod, DngImage, ImageBudget, PsdError, RawFormat,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "raw/cr3_builder.rs"]
mod cr3_builder;
#[path = "raw/crx_encode.rs"]
mod crx_encode;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use cr3_builder::{boxed, cmt1, cr3, ctmd, note, record, sensor_info, Cr3, Track};
use crx_encode::{forward, frame, inverse, Frame, Grid};

const MEDIAN: i32 = 8192;
const WHITE: f64 = 16383.0;
/// Each colour plane's cell in the 2 × 2 pattern, row-major, per layout.
const CELL: [[usize; 4]; 4] = [[0, 1, 2, 3], [1, 0, 3, 2], [2, 3, 0, 1], [3, 2, 1, 0]];
/// The colours at the frame's corner, per layout.
const CORNER: [[u8; 4]; 4] = [[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]];

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn scaled(v: i32, black: f64) -> u16 {
    ((v as f64 - black) * 65535.0 / (WHITE - black))
        .round()
        .clamp(0.0, 65535.0) as u16
}

/// A 14-bit sensor frame of noise with flat patches at the median (zero
/// to the coder) and at another level, so both runs and codes occur.
fn sensor(w: usize, h: usize, seed: u64) -> Grid {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut g = Grid::new(w, h);
    for y in 0..h {
        for x in 0..w {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            g.v[y * w + x] = match (x / 6 + y / 3) % 4 {
                0 => MEDIAN,
                1 => 5000,
                _ => 3000 + (x * 40 + y * 25) as i32 % 4000 + ((state >> 33) % 600) as i32,
            };
        }
    }
    g
}

/// The four colour planes of `s` under `cfa`, relative to the median.
fn planes(s: &Grid, cfa: u8) -> [Grid; 4] {
    std::array::from_fn(|p| {
        let cell = CELL[cfa as usize][p];
        let mut g = Grid::new(s.w / 2, s.h / 2);
        for y in 0..g.h {
            for x in 0..g.w {
                g.v[y * g.w + x] = s.v[(2 * y + cell / 2) * s.w + 2 * x + cell % 2] - MEDIAN;
            }
        }
        g
    })
}

/// The sensor frame four planes (relative to the median) lay out.
fn unplanes(p: &[Grid; 4], cfa: u8) -> Grid {
    let mut s = Grid::new(2 * p[0].w, 2 * p[0].h);
    for (i, g) in p.iter().enumerate() {
        let cell = CELL[cfa as usize][i];
        for y in 0..g.h {
            for x in 0..g.w {
                s.v[(2 * y + cell / 2) * s.w + 2 * x + cell % 2] =
                    (g.v[y * g.w + x] + MEDIAN).clamp(0, 16383);
            }
        }
    }
    s
}

fn raw_track(f: &Frame, planes: &[Grid; 4]) -> Track {
    let (cmp1, sample) = frame(f, planes);
    Track::Raw {
        cmp1,
        samples: vec![sample],
    }
}

fn file(tracks: Vec<Track>) -> Vec<u8> {
    cr3(&Cr3 {
        tracks,
        ..Cr3::default()
    })
}

/// The whole-frame plane a CR3 without a maker note normalizes to.
fn expect_frame(image: &DngImage, s: &Grid, what: &str) {
    assert_eq!((image.width, image.height), (s.w, s.h), "{what}");
    let want: Vec<u16> = s.v.iter().map(|&v| scaled(v, 0.0)).collect();
    let bad = image.samples.iter().zip(&want).position(|(a, b)| a != b);
    assert!(bad.is_none(), "{what}: first difference at {bad:?}");
}

#[test]
fn cr3_is_named_by_its_brand() {
    let s = sensor(48, 48, 1);
    let bytes = file(vec![raw_track(&Frame::lossless((48, 48)), &planes(&s, 0))]);
    assert_eq!(sniff_raw(&bytes), Some(RawFormat::Cr3));
    assert!(sniff_raw_prefix(&bytes[..12]));
    assert_eq!(sniff_raw(&bytes[..12]), Some(RawFormat::Cr3));
    expect_frame(&read(&bytes).expect("CR3 through read_raw"), &s, "sniffed");

    let mut movie = boxed(b"ftyp", b"isom\0\0\x02\0isomiso2");
    movie.extend(boxed(b"mdat", &[0; 16]));
    assert_eq!(sniff_raw(&movie), None);
    assert!(!sniff_raw_prefix(&movie));
    assert!(matches!(read(&movie), Err(PsdError::Unsupported(_))));
}

#[test]
fn cr3_reads_the_largest_tracks_first_frame() {
    let f = Frame::lossless((64, 48));
    let (small, a0, a1, b0) = (
        sensor(48, 48, 2),
        sensor(64, 48, 3),
        sensor(64, 48, 4),
        sensor(64, 48, 5),
    );
    let sample = |s: &Grid| frame(&f, &planes(s, 0));
    let (cmp1, first) = sample(&a0);
    let bytes = file(vec![
        raw_track(&Frame::lossless((48, 48)), &planes(&small, 0)),
        Track::Raw {
            cmp1,
            samples: vec![first, sample(&a1).1],
        },
        Track::Raw {
            cmp1: sample(&b0).0,
            samples: vec![sample(&b0).1, sample(&a1).1],
        },
        Track::Ctmd(ctmd(&[0; 8])),
    ]);
    expect_frame(
        &read(&bytes).expect("multi-track CR3 reads"),
        &a0,
        "first frame",
    );
}

/// An edit of a `CMP1` box.
type Edit = dyn Fn(&mut Vec<u8>);

#[test]
fn cr3_headers_are_refused() {
    let s = sensor(48, 48, 6);
    let (cmp1, sample) = frame(&Frame::lossless((48, 48)), &planes(&s, 0));
    let with = |edit: &dyn Fn(&mut Vec<u8>)| {
        let mut c = cmp1.clone();
        edit(&mut c);
        read(&file(vec![Track::Raw {
            cmp1: c,
            samples: vec![sample.clone()],
        }]))
    };
    let unsupported: [(&Edit, &str); 4] = [
        (&|c| c[25] = 1 << 4, "1-plane"),
        (&|c| c[26] = 1 << 4, "coding 1"),
        (&|c| c[26] = 2 << 4, "coding 2"),
        (&|c| c[24] = 15, "at 15 bits"),
    ];
    for (edit, name) in unsupported {
        let got = with(edit);
        assert!(
            matches!(&got, Err(PsdError::Unsupported(m)) if m.contains(name)),
            "{name}: {got:?}"
        );
    }
    let invalid: [(&Edit, &str); 3] = [
        (
            &|c| c[4..6].copy_from_slice(&0x300u16.to_be_bytes()),
            "version",
        ),
        (&|c| c[26] = 4, "levels"),
        (&|c| c.truncate(39), "short"),
    ];
    for (edit, name) in invalid {
        let got = with(edit);
        assert!(matches!(&got, Err(PsdError::Invalid(_))), "{name}: {got:?}");
    }
    let no_movie = cr3(&Cr3 {
        tracks: vec![raw_track(&Frame::lossless((48, 48)), &planes(&s, 0))],
        no_movie: true,
        ..Cr3::default()
    });
    assert!(matches!(read(&no_movie), Err(PsdError::Invalid(_))));
    let no_raw = file(vec![Track::Ctmd(ctmd(&[0; 8]))]);
    assert!(matches!(read(&no_raw), Err(PsdError::Invalid(_))));
}

#[test]
fn lossless_crx_decodes_exactly() {
    for cfa in 0..4u8 {
        for (w, h, tile) in [(96, 64, (96, 64)), (96, 96, (48, 48))] {
            let s = sensor(w, h, 10 + cfa as u64);
            let f = Frame {
                cfa,
                ..Frame::lossless(tile)
            };
            let image = read(&file(vec![raw_track(&f, &planes(&s, cfa))])).expect("lossless CR3");
            expect_frame(&image, &s, &format!("layout {cfa}, tile {tile:?}"));
            assert_eq!(image.cfa.colors, CORNER[cfa as usize], "layout {cfa}");
        }
    }
}

#[test]
fn unreferenced_crx_lines_decode_exactly() {
    for cfa in 0..4u8 {
        for (w, h, tile) in [(96, 64, (96, 64)), (96, 96, (48, 48))] {
            let s = sensor(w, h, 20 + cfa as u64);
            let f = Frame {
                cfa,
                partial: false,
                ..Frame::lossless(tile)
            };
            let image =
                read(&file(vec![raw_track(&f, &planes(&s, cfa))])).expect("no-reference CR3");
            expect_frame(&image, &s, &format!("layout {cfa}, tile {tile:?}"));
        }
    }
}

#[test]
fn wavelet_crx_decodes_exactly() {
    // 45 × 27 planes: an odd tile at every level.
    let s = sensor(90, 54, 30);
    for levels in 1..=3 {
        let f = Frame {
            levels,
            ..Frame::lossless((90, 54))
        };
        let image = read(&file(vec![raw_track(&f, &planes(&s, 0))])).expect("wavelet CR3");
        expect_frame(&image, &s, &format!("{levels} levels"));
    }
}

/// The plane a three-level frame decodes to when every coefficient is
/// doubled by its quantizer: the textbook inverse of the doubled bands.
fn doubled(p: &[Grid; 4]) -> Grid {
    let p = std::array::from_fn(|i| {
        let bands: Vec<Grid> = forward(&p[i], 3).iter().map(|b| b.map(|v| 2 * v)).collect();
        inverse(&bands)
    });
    unplanes(&p, 0)
}

/// A small-amplitude frame, so doubled coefficients stay in range.
fn quiet(w: usize, h: usize) -> Grid {
    sensor(w, h, 40).map(|v| MEDIAN + (v - 5000) / 5)
}

#[test]
fn quantized_bands_scale_back() {
    let s = quiet(90, 54);
    let p = planes(&s, 0);
    let f = Frame {
        levels: 3,
        q: 10,
        ..Frame::lossless((90, 54))
    };
    let image = read(&file(vec![raw_track(&f, &p)])).expect("quantized CR3");
    expect_frame(&image, &doubled(&p), "quantizer 10");
}

#[test]
fn qp_tables_scale_back() {
    let s = quiet(90, 54);
    let p = planes(&s, 0);
    let f = Frame {
        version: 0x200,
        levels: 3,
        qp: Some((0, 16)),
        ..Frame::lossless((90, 54))
    };
    let image = read(&file(vec![raw_track(&f, &p)])).expect("version 2 CR3");
    expect_frame(&image, &doubled(&p), "constant table");
}

#[test]
fn encoding_3_combines_the_planes() {
    // LibRaw's formula at 14-bit median precision, computed by hand.
    for (values, want) in [
        ([100, 0, 0, 0], [8292, 8292, 8292, 8292]),
        ([100, 10, 20, -30], [8248, 8317, 8297, 8311]),
    ] {
        let p: [Grid; 4] = std::array::from_fn(|i| Grid::new(24, 24).map(|_| values[i]));
        let f = Frame {
            encoding: 3,
            median_bits: Some(14),
            ..Frame::lossless((48, 48))
        };
        let image = read(&file(vec![raw_track(&f, &p)])).expect("encoding 3 CR3");
        for (cell, &v) in want.iter().enumerate() {
            assert_eq!(
                image.samples[(cell / 2) * 48 + cell % 2],
                scaled(v, 0.0),
                "{values:?} cell {cell}"
            );
        }
    }
}

/// A CR3 of `s` whose maker note names the active area `[left, top, right, bottom]`.
fn with_area(s: &Grid, cfa: u8, area: [u16; 4]) -> Vec<u8> {
    let f = Frame {
        cfa,
        ..Frame::lossless((s.w, s.h))
    };
    cr3(&Cr3 {
        cmt3: Some(note(vec![sensor_info(s.w, s.h, area)])),
        tracks: vec![raw_track(&f, &planes(s, cfa))],
        ..Cr3::default()
    })
}

#[test]
fn cr3_masked_border_sets_the_black() {
    let (w, h, left, top) = (96usize, 64usize, 8usize, 4usize);
    let blacks = [500, 510, 520, 530];
    let mut s = sensor(w, h, 50);
    for y in 0..h {
        for x in 0..left {
            s.v[y * w + x] = blacks[(y % 2) * 2 + x % 2];
        }
    }
    let image = read(&with_area(
        &s,
        0,
        [left as u16, top as u16, (w - 1) as u16, (h - 1) as u16],
    ))
    .expect("masked CR3");
    assert_eq!((image.width, image.height), (w - left, h - top));
    for y in 0..image.height {
        for x in 0..image.width {
            let black = blacks[(y % 2) * 2 + x % 2] as f64;
            let v = s.v[(y + top) * w + x + left];
            assert_eq!(
                image.samples[y * image.width + x],
                scaled(v, black),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn cr3_borders_shift_the_colour_pattern() {
    let s = sensor(96, 64, 60);
    for cfa in 0..4u8 {
        for (left, top) in [(8, 4), (9, 4), (8, 5), (9, 5)] {
            let image = read(&with_area(&s, cfa, [left, top, 95, 63])).expect("bordered CR3");
            let corner = CORNER[cfa as usize];
            let want: Vec<u8> = (0..4)
                .map(|i| corner[((i / 2 + top as usize) % 2) * 2 + (i % 2 + left as usize) % 2])
                .collect();
            assert_eq!(image.cfa.colors, want, "layout {cfa}, corner {left}, {top}");
        }
    }
}

#[test]
fn cr3_reports_camera_tags() {
    let s = sensor(64, 48, 70);
    let track = || raw_track(&Frame::lossless((64, 48)), &planes(&s, 0));
    let image = read(&cr3(&Cr3 {
        cmt1: Some(cmt1("Canon", "Canon EOS R3", 6)),
        tracks: vec![track()],
        ..Cr3::default()
    }))
    .expect("tagged CR3");
    assert_eq!(image.make.as_deref(), Some("Canon"));
    assert_eq!(image.model.as_deref(), Some("Canon EOS R3"));
    assert_eq!(image.orientation, 6);
    let crop = image.default_crop;
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (0.0, 0.0, 64.0, 48.0)
    );
    assert_eq!(image.color, Default::default());
    let rgb = demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default())
        .expect("the demosaic takes a CR3");
    assert_eq!((rgb.width, rgb.height), (64, 48));

    let bare = read(&file(vec![track()])).expect("untagged CR3");
    assert_eq!((bare.make, bare.model, bare.orientation), (None, None, 1));
}

#[test]
fn hostile_cr3s_fail_cleanly() {
    let (small, tiled) = (sensor(48, 48, 80), sensor(96, 96, 81));
    let p = planes(&small, 0);
    let lossless = Frame::lossless((48, 48));
    let codings = [
        (lossless.clone(), &tiled),
        (
            Frame {
                partial: false,
                ..lossless.clone()
            },
            &tiled,
        ),
        (
            Frame {
                levels: 3,
                ..lossless.clone()
            },
            &small,
        ),
        (
            Frame {
                version: 0x200,
                levels: 3,
                qp: Some((0, 8)),
                ..lossless.clone()
            },
            &small,
        ),
    ];
    for (f, s) in &codings {
        let valid = file(vec![raw_track(f, &planes(s, 0))]);
        read(&valid).expect("the valid file reads");
        for cut in 0..valid.len() {
            assert!(
                read(&valid[..cut]).is_err(),
                "cut at {cut} of {}",
                valid.len()
            );
        }
        for at in 0..valid.len() {
            let mut bad = valid.clone();
            bad[at] ^= 0xA5;
            let _ = read(&bad);
        }
    }
    // A box running past its parent.
    let mut overrun = file(vec![raw_track(&lossless, &p)]);
    let moov = overrun.windows(4).position(|w| w == b"moov").expect("moov") - 4;
    let size = u32::from_be_bytes(overrun[moov..moov + 4].try_into().unwrap());
    overrun[moov..moov + 4].copy_from_slice(&(size + 1000).to_be_bytes());
    assert!(read(&overrun).is_err());
    // A frame beyond the image budget.
    let (mut cmp1, sample) = frame(&lossless, &p);
    cmp1[8..12].copy_from_slice(&40_000u32.to_be_bytes());
    let huge = file(vec![Track::Raw {
        cmp1,
        samples: vec![sample],
    }]);
    assert!(matches!(read(&huge), Err(PsdError::Invalid(_))));
}

#[test]
fn cr3_corpus_reads_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cr3"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            sniff_raw(&bytes),
            Some(RawFormat::Cr3),
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
        eprintln!(
            "{}: {} × {} {:?} phase means {means:?}",
            path.display(),
            image.width,
            image.height,
            image.model
        );
        seen += 1;
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .CR3 file");
}

/// A small CR3 naming `model`, with `ctmd_sample` as its CTMD track.
fn colour_file(model: &'static str, ctmd_sample: Option<Vec<u8>>) -> Vec<u8> {
    let s = sensor(64, 48, 90);
    let mut tracks = vec![raw_track(&Frame::lossless((64, 48)), &planes(&s, 0))];
    tracks.extend(ctmd_sample.map(Track::Ctmd));
    cr3(&Cr3 {
        cmt1: Some(cmt1("Canon", model, 1)),
        tracks,
        ..Cr3::default()
    })
}

fn canon(model: &'static str) -> pictura_codec::DngColor {
    read(&colour_file(model, None))
        .expect("synthetic CR3 reads")
        .color
}

#[test]
fn cr3_models_get_both_matrices() {
    for model in [
        "Canon EOS R",
        "Canon EOS RP",
        "Canon EOS R5",
        "Canon EOS R5m2",
        "Canon EOS R6",
        "Canon EOS R6m2",
        "Canon EOS R10",
        "Canon EOS 250D",
        "Canon EOS R50",
        "Canon EOS R8",
    ] {
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
        canon("Canon EOS R50").color_matrix,
        canon("Canon EOS R10").color_matrix
    );
    assert_eq!(
        canon("Canon EOS R8").color_matrix,
        canon("Canon EOS R6m2").color_matrix
    );
    assert_ne!(
        canon("Canon EOS R5").color_matrix,
        canon("Canon EOS R6").color_matrix
    );
    assert_eq!(canon("Canon EOS R3").color_matrix, [None, None]);
}

/// Colour data of `version` with the multipliers at `at`.
fn color_data(version: u16, at: usize, levels: [u16; 4]) -> Vec<u16> {
    let mut data = vec![7u16; 1500];
    data[0] = version;
    data[at..at + 4].copy_from_slice(&levels);
    data
}

#[test]
fn cr3_as_shot_neutral_from_ctmd() {
    let want = [1012.0 / 2000.0, 1.0, 1012.0 / 1500.0];
    for (version, at) in [(19, 71), (32, 85), (48, 105), (64, 105)] {
        let data = color_data(version, at, [2000, 1000, 1024, 1500]);
        let image = read(&colour_file("Canon EOS R3", Some(ctmd(&data)))).expect("CR3 with CTMD");
        let got = image
            .color
            .as_shot_neutral
            .unwrap_or_else(|| panic!("version {version}: no neutral"));
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-9, "version {version}: {got:?}");
        }
    }
    let none = read(&colour_file("Canon EOS R3", None)).expect("CR3 without CTMD");
    assert_eq!(none.color.as_shot_neutral, None);
    let mut overrun = record(8, &[0; 20]);
    overrun[0..4].copy_from_slice(&500u32.to_le_bytes());
    let bad = read(&colour_file("Canon EOS R3", Some(overrun))).expect("CR3 with a bad CTMD");
    assert_eq!(bad.color.as_shot_neutral, None);
}

#[test]
fn known_camera_cr3_decodes() {
    let data = color_data(34, 105, [2208, 1024, 1024, 1967]);
    let bytes = colour_file("Canon EOS R5", Some(ctmd(&data)));
    let image = read(&bytes).expect("R5 CR3 reads");
    let source = decode_raw_source(&bytes, &ImageBudget::default()).expect("R5 CR3 decodes");
    let expected = as_shot_transform(&image.color).unwrap().white_balance;
    assert_eq!(source.as_shot, expected);
    assert!(
        (2000.0..=12000.0).contains(&expected.temperature),
        "{expected:?}"
    );
}

#[test]
fn cr3_colour_corpus_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cr3"))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let image = read(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let decoded = decode_raw_source(&bytes, &ImageBudget::default());
        if image.color.color_matrix.iter().all(Option::is_some) {
            let source = decoded.unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let t = source.as_shot.temperature;
            assert!((2000.0..=12000.0).contains(&t), "{}: {t} K", path.display());
            eprintln!(
                "{}: {:?} neutral {:?} as shot {t:.0} K tint {:.1}",
                path.display(),
                image.model,
                image.color.as_shot_neutral,
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
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .CR3 file");
}
