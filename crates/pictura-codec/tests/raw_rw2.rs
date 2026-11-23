//! The Panasonic RW2 reader (`codec/vendor-raw-containers`, #1472) against
//! synthetic RW2s written by `tests/raw/rw2_builder.rs`, and, opt-in, real
//! camera files.

use pictura_codec::{
    decode_raw_source, demosaic, read_raw, sniff_raw, sniff_raw_prefix, DemosaicMethod, DngImage,
    ImageBudget, PsdError, RawFormat,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[path = "raw/rw2_builder.rs"]
mod rw2_builder;

use builder::{noise_plane, write_tiff, IfdSpec, Val};
use rw2_builder::{data_start, rw2, Data, Rw2};

/// A whole number of both packings' groups.
const WIDTH: usize = 90;
const HEIGHT: usize = 12;
const WORDS_12: Data = Data::Words {
    shift: 4,
    compression: 34828,
};
const WORDS_14: Data = Data::Words {
    shift: 2,
    compression: 34830,
};
const CODINGS: [Data; 4] = [WORDS_12, WORDS_14, Data::Ten, Data::Nine];

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn sample(data: Data) -> Rw2 {
    let max = match data {
        Data::Ten => 4095,
        Data::Words { shift, .. } => u16::MAX >> shift,
        Data::Nine => 16383,
    };
    let samples = noise_plane(WIDTH, HEIGHT, max, 23).samples;
    Rw2::new(WIDTH, HEIGHT, samples, data)
}

/// The colour code `cfa` gives the photosite at `x`, `y` of the sensor.
fn colour(cfa: u16, x: usize, y: usize) -> u8 {
    let pattern = [[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]][cfa as usize - 1];
    pattern[y % 2 * 2 + x % 2]
}

/// What the reader owes for `r`: the area inside the borders, normalized.
fn normalized(r: &Rw2) -> Vec<u16> {
    let (shift, scale) = match r.data {
        Data::Words { shift, .. } => (shift, 16.0),
        _ => (0, 1.0),
    };
    let [top, left, bottom, right] = r.borders.map(usize::from);
    let white = f64::from(r.white) * scale;
    let mut out = Vec::new();
    for y in top..bottom {
        for x in left..right {
            let stored = f64::from(r.samples[y * r.width + x] << shift);
            let black = f64::from(r.black[colour(r.cfa, x, y) as usize]) * scale;
            let v = ((stored - black) * 65535.0 / (white - black)).round();
            out.push(v.clamp(0.0, 65535.0) as u16);
        }
    }
    out
}

#[test]
fn rw2_is_named_by_its_header() {
    for make in ["Panasonic", "LEICA"] {
        let mut r = sample(Data::Ten);
        r.make = make;
        let bytes = rw2(&r);
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Rw2));
        assert_eq!(sniff_raw(&bytes[..8]), Some(RawFormat::Rw2));
        assert!(sniff_raw_prefix(&bytes[..8]));
        read(&bytes).expect("RW2 through read_raw");
    }
    let tiff = IfdSpec {
        tags: vec![(271, Val::Ascii("Panasonic"))],
        ..IfdSpec::default()
    };
    let tiff = write_tiff(&tiff, false, false);
    assert_eq!(sniff_raw(&tiff), None);
    assert!(!sniff_raw_prefix(&tiff));
}

#[test]
fn rw2_words_decode_exactly() {
    for data in [WORDS_12, WORDS_14] {
        let mut r = sample(data);
        r.black = [16, 16, 16];
        r.white = 3967;
        let image = read(&rw2(&r)).unwrap();
        assert_eq!(image.samples, normalized(&r), "{data:?}");
    }
}

#[test]
fn packed_rw2s_decode_exactly() {
    for data in [Data::Ten, Data::Nine] {
        for raw_data_offset in [false, true] {
            let mut r = sample(data);
            r.black = [128, 128, 128];
            r.raw_data_offset = raw_data_offset;
            let image = read(&rw2(&r)).unwrap();
            assert_eq!(image.samples, normalized(&r), "{data:?} {raw_data_offset}");
        }
    }
}

#[test]
fn other_rw2_codings_are_refused_by_name() {
    for format in [3, 4, 5, 6, 8] {
        for compression in [Some(34316), None] {
            let mut r = sample(Data::Ten);
            r.raw_format = Some(format);
            r.compression = compression;
            let bytes = rw2(&r);
            assert_eq!(sniff_raw(&bytes), Some(RawFormat::Rw2));
            match decode_raw_source(&bytes, &ImageBudget::default()) {
                Err(PsdError::Unsupported(why)) => {
                    assert!(why.contains(&format!("RawFormat {format} ")), "{why}")
                }
                other => panic!("RawFormat {format}: {:?}", other.err()),
            }
        }
    }
}

#[test]
fn rw2_variants_are_refused() {
    let mut ten = sample(Data::Ten);
    ten.bits = 14;
    let mut nine = sample(Data::Nine);
    nine.bits = 12;
    let mut words = sample(WORDS_12);
    words.bits = 10;
    let mut compression = sample(WORDS_12);
    compression.compression = Some(1);
    let narrow = Rw2::new(85, HEIGHT, vec![0; 85 * HEIGHT], Data::Ten);
    let mut cfa = sample(Data::Ten);
    cfa.cfa = 5;
    let mut long = sample(Data::Nine);
    long.extra = 2;
    for (what, r) in [
        ("RawFormat 2 at 14 bits", ten),
        ("RawFormat 7 at 12 bits", nine),
        ("10-bit words", words),
        ("compression 1", compression),
        ("width", narrow),
        ("cfa", cfa),
        ("length", long),
    ] {
        assert!(
            matches!(read(&rw2(&r)), Err(PsdError::Unsupported(_))),
            "{what}"
        );
    }
}

#[test]
fn rw2_borders_shift_the_colour_pattern() {
    for cfa in 1..=4 {
        for (top, left) in [(2u16, 4u16), (2, 5), (3, 4), (3, 5)] {
            let mut r = sample(Data::Ten);
            r.cfa = cfa;
            r.black = [40, 90, 140];
            r.borders = [top, left, 11, 88];
            let image = read(&rw2(&r)).unwrap();
            let case = format!("code {cfa} at {top}, {left}");
            assert_eq!(
                (image.width, image.height),
                (88 - left as usize, 11 - top as usize),
                "{case}"
            );
            let (y, x) = (top as usize, left as usize);
            let expected =
                [(0, 0), (0, 1), (1, 0), (1, 1)].map(|(dy, dx)| colour(cfa, x + dx, y + dy));
            assert_eq!(image.cfa.colors, expected, "{case}");
            assert_eq!(image.samples, normalized(&r), "{case}");
        }
    }
}

#[test]
fn rw2_crop_counts_from_the_sensor() {
    let whole = [0.0, 0.0, 83.0, 8.0];
    for (crop, expected) in [
        (Some([4, 9, 10, 69]), [4.0, 1.0, 60.0, 6.0]),
        (Some([2, 5, 10, 69]), whole),
        (Some([4, 9, 12, 69]), whole),
        (None, whole),
    ] {
        let mut r = sample(Data::Nine);
        r.borders = [3, 5, 11, 88];
        r.crop = crop;
        let c = read(&rw2(&r)).unwrap().default_crop;
        assert_eq!([c.x, c.y, c.width, c.height], expected, "{crop:?}");
    }
}

#[test]
fn rw2_reports_camera_tags() {
    let mut r = sample(WORDS_14);
    r.model = "DMC-L1";
    r.orientation = 6;
    let image = read(&rw2(&r)).unwrap();
    assert_eq!(image.make.as_deref(), Some("Panasonic"));
    assert_eq!(image.model.as_deref(), Some("DMC-L1"));
    assert_eq!(image.orientation, 6);
    assert_eq!(image.color.as_shot_neutral, Some([0.5, 1.0, 256.0 / 384.0]));
    demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default()).expect("demosaic");

    r.wb[0] = 0;
    assert_eq!(read(&rw2(&r)).unwrap().color.as_shot_neutral, None);
}

#[test]
fn rw2_without_colour_names_the_camera() {
    let bytes = rw2(&sample(WORDS_12));
    assert_eq!(read(&bytes).unwrap().color.color_matrix, [None, None]);
    match decode_raw_source(&bytes, &ImageBudget::default()) {
        Err(PsdError::Unsupported(why)) => assert!(
            why.contains("camera colour for Panasonic DMC-FZ50 is not supported yet"),
            "{why}"
        ),
        other => panic!("{:?}", other.err()),
    }
}

#[test]
fn hostile_rw2s_fail_cleanly() {
    for data in CODINGS {
        let mut r = sample(data);
        r.crop = Some([2, 2, 10, 80]);
        r.raw_data_offset = data == Data::Nine;
        let bytes = rw2(&r);
        for len in 0..bytes.len() {
            assert!(read(&bytes[..len]).is_err(), "{data:?} cut at {len}");
        }
        for at in 4..data_start(&r) {
            for flip in [0x01, 0x80, 0xFF] {
                let mut broken = bytes.clone();
                broken[at] ^= flip;
                // Any answer but a panic.
                let _ = read(&broken);
            }
        }
    }

    for borders in [
        [0, 0, 13, 90],
        [0, 0, 12, 91],
        [8, 0, 8, 90],
        [0, 50, 12, 40],
    ] {
        let mut r = sample(Data::Ten);
        r.borders = borders;
        assert!(
            matches!(read(&rw2(&r)), Err(PsdError::Invalid(_))),
            "{borders:?}"
        );
    }

    let budget = ImageBudget {
        max_dimension: WIDTH as u32 - 1,
        ..ImageBudget::default()
    };
    assert!(
        matches!(
            read_raw(&rw2(&sample(Data::Ten)), &budget),
            Err(PsdError::Invalid(_))
        ),
        "width over the budget"
    );
}

#[test]
fn rw2_corpus_reads_when_present() {
    let Some(dir) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        eprintln!("SKIP rw2 corpus: PICTURA_RAW_CORPUS is not set");
        return;
    };
    let (mut seen, mut refused) = (0, 0);
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        let wanted = path.extension().is_some_and(|e| {
            ["rw2", "rwl", "raw"]
                .iter()
                .any(|known| e.eq_ignore_ascii_case(known))
        });
        if !wanted {
            continue;
        }
        let bytes = std::fs::read(&path).expect("corpus file");
        // `.raw` names other formats too.
        if !bytes.starts_with(b"IIU\0") {
            continue;
        }
        seen += 1;
        let name = path.display();
        assert_eq!(sniff_raw(&bytes), Some(RawFormat::Rw2), "{name}");
        let image = match read(&bytes) {
            Ok(image) => image,
            Err(PsdError::Unsupported(why)) if why.contains("sensor data is not supported yet") => {
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
        let mut sum = [0f64; 4];
        for (i, &v) in image.samples.iter().enumerate() {
            sum[(i / image.width % 2) * 2 + i % image.width % 2] += f64::from(v);
        }
        let greens: Vec<f64> = (0..4)
            .filter(|&p| image.cfa.colors[p] == 1)
            .map(|p| sum[p])
            .collect();
        if (greens[0] - greens[1]).abs() > 0.05 * greens[0].max(greens[1]) {
            failures.push(format!("{name}: greens disagree, {sum:?}"));
        }
        let clipped = image.samples.iter().filter(|&&v| v == 65535).count();
        eprintln!(
            "{name}: {} x {} {:?} crop {:?} clipped {clipped} neutral {:?}",
            image.width, image.height, image.model, image.default_crop, image.color.as_shot_neutral,
        );
    }
    eprintln!("{seen} files, {refused} refused");
    assert!(
        seen > 0,
        "PICTURA_RAW_CORPUS holds no RW2, RWL or Panasonic RAW file"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
