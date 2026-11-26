//! The Sony ARW reader (`codec/vendor-raw-containers`, #1470) against
//! synthetic ARWs written by `tests/camera_raw/raw/arw_builder.rs`, and, opt-in, real
//! camera files.

use pictura_codec::{
    demosaic, read_raw, sniff_raw, sniff_raw_prefix, DemosaicMethod, DngImage, ImageBudget,
    PsdError, RawFormat,
};

#[path = "raw/arw_builder.rs"]
mod arw_builder;
#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;

use arw_builder::{arw, curve, encode_arw2, Arw, Coding, Levels, Place};
use builder::{dng, noise_plane, Options, Plane, Val};

const KNEES: [u16; 4] = [8000, 10400, 12900, 14100];

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

/// `samples` (`width` wide) normalized by a black per 2 × 2 position and a
/// white.
fn normalized(samples: &[u16], width: usize, black: [u16; 4], white: u16) -> Vec<u16> {
    samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let b = black[(i / width % 2) * 2 + i % width % 2] as f64;
            ((v as f64 - b) * 65535.0 / (white as f64 - b))
                .round()
                .clamp(0.0, 65535.0) as u16
        })
        .collect()
}

fn plane_14bit() -> Plane {
    noise_plane(64, 20, 16383, 11)
}

fn codes() -> Plane {
    noise_plane(64, 12, 2047, 12)
}

fn levels() -> Levels {
    Levels {
        black: [500, 510, 520, 530],
        white: 16000,
        wb_rggb: [2000, 1000, 1024, 1500],
    }
}

#[test]
fn arw_is_named_by_its_make() {
    let bytes = arw(&Arw::new(plane_14bit(), Coding::Uncompressed));
    assert_eq!(sniff_raw(&bytes), Some(RawFormat::Arw));
    assert!(sniff_raw_prefix(&bytes));
    read(&bytes).expect("ARW through read_raw");

    let mut other = Arw::new(plane_14bit(), Coding::Uncompressed);
    other.make = "FUJIFILM";
    let other = arw(&other);
    assert_eq!(sniff_raw(&other), None);
    assert!(!sniff_raw_prefix(&other));

    let sony_dng = dng(
        &plane_14bit(),
        Options {
            ifd0_tags: vec![(271, Val::Ascii("SONY"))],
            ..Options::default()
        },
    );
    assert_eq!(sniff_raw(&sony_dng), Some(RawFormat::Dng));
}

#[test]
fn uncompressed_arw_decodes_exactly() {
    let mut spec = Arw::new(plane_14bit(), Coding::Uncompressed);
    spec.levels = levels();
    let image = read(&arw(&spec)).expect("uncompressed ARW");
    assert_eq!((image.width, image.height), (64, 20));
    let l = levels();
    assert_eq!(
        image.samples,
        normalized(&spec.plane.samples, 64, l.black, l.white)
    );
}

#[test]
fn arw2_blocks_decode_through_the_tone_curve() {
    let plane = codes();
    let (_, decoded) = encode_arw2(&plane);
    for (d, v) in decoded.iter().zip(&plane.samples) {
        assert!(d.abs_diff(*v) < 16, "the encoder loses at most the shift");
    }
    let table = curve(KNEES);
    let linear: Vec<u16> = decoded.iter().map(|&c| table[c as usize]).collect();
    let mut spec = Arw::new(plane, Coding::Arw2 { knees: KNEES });
    spec.levels.black = [512; 4];
    let image = read(&arw(&spec)).expect("ARW2");
    assert_eq!((image.width, image.height), (64, 12));
    assert_eq!(image.samples, normalized(&linear, 64, [512; 4], 16383));
}

#[test]
fn lossless_tiles_decode_to_the_sensor_size() {
    let plane = noise_plane(80, 40, 16383, 13);
    let mut spec = Arw::new(
        plane,
        Coding::Lossless {
            tile_w: 32,
            tile_h: 16,
        },
    );
    spec.levels = levels();
    let image = read(&arw(&spec)).expect("lossless ARW");
    assert_eq!((image.width, image.height), (80, 40));
    let l = levels();
    assert_eq!(
        image.samples,
        normalized(&spec.plane.samples, 80, l.black, l.white)
    );
}

#[test]
fn sr2_block_supplies_the_levels() {
    let l = levels();
    let want_plane = normalized(&plane_14bit().samples, 64, l.black, l.white);
    let want_neutral = [1012.0 / 2000.0, 1.0, 1012.0 / 1500.0];
    for place in [Place::Sr2, Place::Sr2Old, Place::RawIfd] {
        let mut spec = Arw::new(plane_14bit(), Coding::Uncompressed);
        spec.levels = l;
        spec.place = place;
        let image = read(&arw(&spec)).expect("ARW with levels");
        assert_eq!(image.samples, want_plane, "levels in the {}", name(place));
        let neutral = image.color.as_shot_neutral.expect("as-shot neutral");
        for (g, w) in neutral.iter().zip(want_neutral) {
            assert!((g - w).abs() <= 1e-9, "{}: {neutral:?}", name(place));
        }
    }

    // The raw IFD's levels win over the SR2 block's.
    let mut spec = Arw::new(plane_14bit(), Coding::Uncompressed);
    spec.levels = l;
    spec.sr2_too = Some(Levels {
        black: [0; 4],
        white: 4095,
        wb_rggb: [1, 1, 1, 1],
    });
    let image = read(&arw(&spec)).expect("ARW with both");
    assert_eq!(image.samples, want_plane);

    // Without levels anywhere: black 0 and the precision's white.
    let mut spec = Arw::new(plane_14bit(), Coding::Uncompressed);
    spec.place = Place::Nowhere;
    let image = read(&arw(&spec)).expect("ARW without levels");
    assert_eq!(
        image.samples,
        normalized(&spec.plane.samples, 64, [0; 4], 16383)
    );
    assert_eq!(image.color.as_shot_neutral, None);
}

fn name(place: Place) -> &'static str {
    match place {
        Place::Nowhere => "nowhere",
        Place::RawIfd => "raw IFD",
        Place::Sr2 => "SR2 block",
        Place::Sr2Old => "SR2 block, older tags",
    }
}

#[test]
fn arw_variants_are_refused() {
    let unsupported = |spec: Arw, what: &str| {
        let got = read(&arw(&spec));
        assert!(
            matches!(&got, Err(PsdError::Unsupported(_))),
            "{what}: {:?}",
            got.err()
        );
    };
    let mut small = Arw::new(plane_14bit(), Coding::Uncompressed);
    small.photometric = 6;
    unsupported(small, "YCbCr small raw");

    let mut arw1 = Arw::new(codes(), Coding::Arw2 { knees: KNEES });
    arw1.raw_tags = vec![(279, Val::Long(vec![64 * 12 / 2]))];
    unsupported(arw1, "ARW 1.0 strip");

    let mut packed = Arw::new(plane_14bit(), Coding::Uncompressed);
    packed.raw_tags = vec![(279, Val::Long(vec![64 * 20 * 3 / 2]))];
    unsupported(packed, "packed uncompressed");

    let mut other = Arw::new(plane_14bit(), Coding::Uncompressed);
    other.compression = Some(8);
    unsupported(other, "deflate");
}

#[test]
fn arw_reports_camera_tags() {
    let mut spec = Arw::new(plane_14bit(), Coding::Uncompressed);
    spec.model = "ILCE-7M3";
    spec.orientation = 6;
    spec.cfa = [1, 2, 0, 1];
    spec.crop = Some(([4, 2], [56, 16]));
    let image = read(&arw(&spec)).expect("tagged ARW");
    assert_eq!(image.make.as_deref(), Some("SONY"));
    assert_eq!(image.model.as_deref(), Some("ILCE-7M3"));
    assert_eq!(image.orientation, 6);
    assert_eq!(image.cfa.colors, vec![1, 2, 0, 1]);
    let crop = image.default_crop;
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (4.0, 2.0, 56.0, 16.0)
    );
    assert!(image.color.color_matrix.iter().all(Option::is_some));
    assert!(image.color.as_shot_neutral.is_some());
    demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default()).expect("demosaic accepts ARW");

    let mut unknown = Arw::new(plane_14bit(), Coding::Uncompressed);
    unknown.model = "NEX-5";
    let image = read(&arw(&unknown)).expect("unknown-model ARW");
    assert_eq!(image.color.color_matrix, [None, None]);
    let crop = image.default_crop;
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (0.0, 0.0, 64.0, 20.0)
    );
}

#[test]
fn hostile_arws_fail_cleanly() {
    let mut files = vec![
        arw(&Arw::new(plane_14bit(), Coding::Uncompressed)),
        arw(&Arw::new(codes(), Coding::Arw2 { knees: KNEES })),
        arw(&Arw::new(
            noise_plane(40, 20, 16383, 5),
            Coding::Lossless {
                tile_w: 16,
                tile_h: 16,
            },
        )),
    ];
    let mut sr2 = Arw::new(plane_14bit(), Coding::Uncompressed);
    sr2.place = Place::Sr2;
    files.push(arw(&sr2));
    for bytes in &files {
        for cut in 0..bytes.len() {
            let _ = read(&bytes[..cut]);
        }
    }

    let mut cases = Vec::new();
    let mut narrow = Arw::new(codes(), Coding::Arw2 { knees: KNEES });
    narrow.raw_tags = vec![(256, Val::Long(vec![48]))];
    cases.push(("ARW2 width 48", narrow));
    let mut no_curve = Arw::new(codes(), Coding::Arw2 { knees: KNEES });
    no_curve.raw_tags = vec![(0x7010, Val::Short(vec![1, 2]))];
    cases.push(("tone curve of two values", no_curve));
    let mut wide = Arw::new(
        noise_plane(40, 20, 16383, 5),
        Coding::Lossless {
            tile_w: 16,
            tile_h: 16,
        },
    );
    wide.raw_tags = vec![(0x7038, Val::Long(vec![60, 20]))];
    cases.push(("sensor size beyond the plane", wide));
    let mut huge = Arw::new(plane_14bit(), Coding::Uncompressed);
    huge.raw_tags = vec![(256, Val::Long(vec![1 << 20]))];
    cases.push(("width beyond the budget", huge));
    for (what, spec) in cases {
        assert!(read(&arw(&spec)).is_err(), "{what} was read");
    }

    // An SR2 block pointing past the file start's limit.
    let mut far = arw(&sr2);
    // Tag 0x7200, type LONG, count 1.
    let entry = [0x00, 0x72, 4, 0, 1, 0, 0, 0];
    let at = far
        .windows(8)
        .position(|w| w == entry)
        .expect("SR2 offset entry");
    far[at + 8..at + 12].copy_from_slice(&(64u32 << 20).to_le_bytes());
    assert!(read(&far).is_err());
}

#[test]
fn arw_corpus_reads_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let is_arw = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("arw"));
        if !is_arw {
            continue;
        }
        seen += 1;
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            sniff_raw(&bytes),
            Some(RawFormat::Arw),
            "{}",
            path.display()
        );
        let image = match read(&bytes) {
            Ok(image) => image,
            Err(PsdError::Unsupported(why)) => {
                eprintln!("{}: refused, {why}", path.display());
                continue;
            }
            Err(e) => panic!("{}: {e}", path.display()),
        };
        let first = image.samples[0];
        assert!(
            image.samples.iter().any(|&v| v != first),
            "{} is flat",
            path.display()
        );
        // Per 2 × 2 position: the two reported greens have nearly equal
        // means, and same-colour neighbours two columns apart differ little
        // compared with the frame's spread.
        let mut sum = [0f64; 4];
        let mut n = [0f64; 4];
        let mut step = 0f64;
        for (i, &v) in image.samples.iter().enumerate() {
            let (x, y) = (i % image.width, i / image.width);
            let p = (y % 2) * 2 + x % 2;
            sum[p] += v as f64;
            n[p] += 1.0;
            if x >= 2 {
                step += (v as f64 - image.samples[i - 2] as f64).abs();
            }
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
        let step = step / image.samples.len() as f64;
        let clipped = image.samples.iter().filter(|&&v| v == 65535).count();
        eprintln!(
            "{}: {} × {} {:?} crop {:?} phase means {means:?} neighbour step {step:.0} clipped {clipped} neutral {:?}",
            path.display(),
            image.width,
            image.height,
            image.model,
            image.default_crop,
            image.color.as_shot_neutral,
        );
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .ARW file");
}
