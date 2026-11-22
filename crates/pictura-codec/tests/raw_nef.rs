//! The Nikon NEF reader (`codec/vendor-raw-containers`, #1469) against
//! synthetic NEFs written by `tests/raw/nef_builder.rs`, and, opt-in, real
//! camera files.

use pictura_codec::{
    demosaic, read_raw, sniff_raw, DemosaicMethod, DngImage, ImageBudget, PsdError, RawFormat,
};

#[allow(dead_code)]
#[path = "dng/builder.rs"]
mod builder;
#[allow(dead_code)]
#[path = "dng/ljpeg_encode.rs"]
mod ljpeg_encode;
#[path = "raw/nef_builder.rs"]
mod nef_builder;

use builder::{noise_plane, Val};
use nef_builder::{encode_huffman, nef, Data, Lin, Nef};

fn read(bytes: &[u8]) -> Result<DngImage, PsdError> {
    read_raw(bytes, &ImageBudget::default())
}

fn scaled(v: u16, white: f64, black: f64) -> u16 {
    ((v as f64 - black) * 65535.0 / (white - black))
        .round()
        .clamp(0.0, 65535.0) as u16
}

fn white(bits: u8) -> f64 {
    ((1u32 << bits) - 1) as f64
}

fn expect_plane(spec: &Nef, what: &str) {
    let image = read(&nef(spec)).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_eq!(
        (image.width, image.height),
        (spec.plane.width, spec.plane.height),
        "{what}"
    );
    let expected: Vec<u16> = spec
        .plane
        .samples
        .iter()
        .map(|&v| scaled(v, white(spec.bits), 0.0))
        .collect();
    assert_eq!(image.samples, expected, "{what}");
}

#[test]
fn nef_takes_the_cfa_subifd() {
    let mut spec = Nef::new(noise_plane(40, 16, 16383, 1), 14, Data::Containers);
    spec.cfa = [2, 1, 1, 0];
    let bytes = nef(&spec);
    assert_eq!(sniff_raw(&bytes), Some(RawFormat::Nef));
    let image = read(&bytes).expect("NEF reads");
    assert_eq!((image.cfa.width, image.cfa.height), (2, 2));
    assert_eq!(image.cfa.colors, vec![2, 1, 1, 0]);
    expect_plane(&spec, "BGGR NEF");
}

#[test]
fn uncompressed_nef_layouts_decode() {
    let spec = Nef::new(
        noise_plane(37, 17, 4095, 2),
        12,
        Data::Msb { rows_per_strip: 5 },
    );
    expect_plane(&spec, "12-bit MSB-first strips");
    // Little-endian: one strip of rows on whole 32-bit words (Coolpix NRW).
    let mut spec = Nef::new(
        noise_plane(40, 9, 4095, 2),
        12,
        Data::Msb { rows_per_strip: 9 },
    );
    spec.be = false;
    expect_plane(&spec, "12-bit MSB-first in little-endian words");
    for be in [true, false] {
        let mut spec = Nef::new(noise_plane(40, 16, 16383, 3), 14, Data::Containers);
        spec.be = be;
        expect_plane(&spec, &format!("14-bit containers, big-endian {be}"));
    }
    let mut spec = Nef::new(noise_plane(40, 16, 4095, 4), 12, Data::Lsb);
    spec.be = false;
    expect_plane(&spec, "12-bit LSB-first under 34713");
    let spec = Nef::new(noise_plane(37, 9, 4095, 5), 12, Data::Padded);
    expect_plane(&spec, "12-bit padded groups under 34713 (D100)");
}

#[test]
fn nef_variants_are_refused() {
    let unsupported = |spec: &Nef, what: &str| {
        let got = read(&nef(spec));
        assert!(
            matches!(got, Err(PsdError::Unsupported(_))),
            "{what}: {:?}",
            got.err()
        );
    };
    for c in [8, 13, 14] {
        let mut spec = Nef::new(
            noise_plane(32, 8, 4095, 5),
            12,
            Data::Huffman(Lin::lossless(12)),
        );
        spec.nef_compression = Some(c);
        unsupported(&spec, &format!("NEFCompression {c}"));
    }
    let mut spec = Nef::new(noise_plane(32, 8, 4095, 5), 12, Data::Containers);
    spec.compression = Some(7);
    unsupported(&spec, "compression 7");
    let spec = Nef::new(
        noise_plane(32, 8, 1023, 5),
        10,
        Data::Msb { rows_per_strip: 8 },
    );
    unsupported(&spec, "10-bit");
    // A strip larger than any known packing of the rows it claims.
    let mut spec = Nef::new(noise_plane(32, 8, 4095, 5), 12, Data::Lsb);
    spec.height_tag = Some(7);
    unsupported(&spec, "an unknown uncompressed layout");
}

#[test]
fn lossless_nef_decodes_exactly() {
    for bits in [12u8, 14] {
        for be in [true, false] {
            let plane = noise_plane(64, 20, white(bits) as u16, bits as u64);
            let mut spec = Nef::new(plane, bits, Data::Huffman(Lin::lossless(bits)));
            spec.be = be;
            expect_plane(&spec, &format!("{bits}-bit lossless, big-endian {be}"));
        }
    }
}

fn expect_curve(spec: &Nef, curve: &[u16], what: &str) {
    let image = read(&nef(spec)).unwrap_or_else(|e| panic!("{what}: {e}"));
    let expected: Vec<u16> = spec
        .plane
        .samples
        .iter()
        .map(|&v| scaled(curve[v as usize], white(spec.bits), 0.0))
        .collect();
    assert_eq!(image.samples, expected, "{what}");
}

#[test]
fn lossy_nef_maps_through_the_curve() {
    for bits in [12u8, 14] {
        let lin = Lin::stepped(bits);
        let curve = lin.full_curve(bits);
        assert_eq!(curve.len(), (1 << bits) + 1);
        let plane = noise_plane(64, 20, white(bits) as u16, 20 + bits as u64);
        let spec = Nef::new(plane, bits, Data::Huffman(lin));
        expect_curve(&spec, &curve, &format!("{bits}-bit stepped curve"));
    }
    let listed: Vec<u16> = (0..683u32)
        .map(|i| (i * i * 4095 / (682 * 682)) as u16)
        .collect();
    let lin = Lin {
        version: [0x44, 0x10],
        vpred: [0x148; 4],
        curve: listed.clone(),
        split: None,
    };
    let spec = Nef::new(noise_plane(64, 20, 682, 30), 12, Data::Huffman(lin));
    expect_curve(&spec, &listed, "12-bit listed curve");
}

#[test]
fn lossy_nef_switches_tables_at_the_split() {
    for bits in [12u8, 14] {
        let mut lin = Lin::stepped(bits);
        lin.split = Some(6);
        let curve = lin.full_curve(bits);
        let plane = noise_plane(48, 14, white(bits) as u16, 40 + bits as u64);
        let (_, recon) = encode_huffman(&plane, &lin, bits);
        let w = plane.width;
        for (i, (&r, &v)) in recon.iter().zip(&plane.samples).enumerate().take(6 * w) {
            assert_eq!(r, v as i32, "{bits}-bit sample {i} above the split");
        }
        assert_ne!(
            recon[6 * w..],
            plane.samples[6 * w..]
                .iter()
                .map(|&v| v as i32)
                .collect::<Vec<_>>(),
            "the after-split table drops bits"
        );
        let top = curve.len() as i32 - 1;
        let expected: Vec<u16> = recon
            .iter()
            .map(|&r| scaled(curve[r.clamp(0, top) as usize], white(bits), 0.0))
            .collect();
        let spec = Nef::new(plane, bits, Data::Huffman(lin));
        let image = read(&nef(&spec)).unwrap_or_else(|e| panic!("{bits}-bit split: {e}"));
        assert_eq!(image.samples, expected, "{bits}-bit split");
    }
}

#[test]
fn nef_levels_and_camera_tags() {
    let (w, h) = (40usize, 16usize);
    let blacks = [600u16, 610, 620, 630];
    let mut plane = noise_plane(w, h, 15000, 50);
    plane.samples.iter_mut().for_each(|v| *v += 700);
    let mut spec = Nef::new(plane, 14, Data::Containers);
    spec.black = Some(blacks.to_vec());
    spec.ifd0_tags = vec![
        (271, Val::Ascii("NIKON CORPORATION")),
        (272, Val::Ascii("NIKON TESTCAM")),
        (274, Val::Short(vec![8])),
    ];
    let image = read(&nef(&spec)).expect("14-bit NEF with black reads");
    for (i, &v) in spec.plane.samples.iter().enumerate() {
        let black = blacks[(i / w % 2) * 2 + i % w % 2] as f64;
        assert_eq!(image.samples[i], scaled(v, 16383.0, black), "sample {i}");
    }
    assert_eq!(image.make.as_deref(), Some("NIKON CORPORATION"));
    assert_eq!(image.model.as_deref(), Some("NIKON TESTCAM"));
    assert_eq!(image.orientation, 8);
    let crop = image.default_crop;
    assert_eq!(
        (crop.x, crop.y, crop.width, crop.height),
        (0.0, 0.0, w as f64, h as f64)
    );
    assert_eq!(image.color.color_matrix, [None, None]);
    let rgb = demosaic(&image, DemosaicMethod::Ahd, &ImageBudget::default())
        .expect("the demosaic takes a NEF");
    assert_eq!((rgb.width, rgb.height), (w, h));

    let mut plane = noise_plane(w, h, 3900, 51);
    plane.samples.iter_mut().for_each(|v| *v += 150);
    let mut spec = Nef::new(plane, 12, Data::Msb { rows_per_strip: 4 });
    spec.black = Some(vec![400; 4]);
    let image = read(&nef(&spec)).expect("12-bit NEF with black reads");
    let expected: Vec<u16> = spec
        .plane
        .samples
        .iter()
        .map(|&v| scaled(v, 4095.0, 100.0))
        .collect();
    assert_eq!(
        image.samples, expected,
        "the 14-bit black scaled to 12 bits"
    );
}

#[test]
fn hostile_nefs_fail_cleanly() {
    let valid = nef(&Nef::new(
        noise_plane(32, 8, 4095, 60),
        12,
        Data::Huffman(Lin::stepped(12)),
    ));
    read(&valid).expect("the uncut file reads");
    for len in 0..valid.len() {
        assert!(read(&valid[..len]).is_err(), "prefix of {len} bytes");
    }

    let mut spec = Nef::new(
        noise_plane(32, 8, 4095, 61),
        12,
        Data::Huffman(Lin::stepped(12)),
    );
    spec.lin_bytes = Some(vec![0x44, 0x20, 1, 0, 1, 0, 1, 0]);
    assert!(
        read(&nef(&spec)).is_err(),
        "a table shorter than its header"
    );

    let mut spec = Nef::new(
        noise_plane(32, 8, 4095, 62),
        12,
        Data::Huffman(Lin::lossless(12)),
    );
    spec.height_tag = Some(64);
    assert!(read(&nef(&spec)).is_err(), "data shorter than the plane");

    let small = ImageBudget {
        max_dimension: 16,
        ..ImageBudget::default()
    };
    let spec = Nef::new(noise_plane(32, 8, 4095, 63), 12, Data::Containers);
    assert!(
        read_raw(&nef(&spec), &small).is_err(),
        "wider than the budget"
    );
}

#[test]
fn nef_corpus_reads_when_present() {
    let Some(root) = std::env::var_os("PICTURA_RAW_CORPUS") else {
        return;
    };
    let mut seen = 0;
    for entry in std::fs::read_dir(&root).expect("corpus directory") {
        let path = entry.expect("corpus entry").path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("nef"))
        {
            continue;
        }
        seen += 1;
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            sniff_raw(&bytes),
            Some(RawFormat::Nef),
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
        let mut sorted = image.samples.clone();
        sorted.sort_unstable();
        let low = sorted[sorted.len() / 10000];
        let clipped = image.samples.iter().filter(|&&v| v == 65535).count();
        eprintln!(
            "{}: {} × {} {:?} cfa {:?} phase means {means:?} 0.01% {low} max {} clipped {clipped}",
            path.display(),
            image.width,
            image.height,
            image.model,
            image.cfa.colors,
            sorted[sorted.len() - 1],
        );
    }
    assert!(seen > 0, "PICTURA_RAW_CORPUS holds no .NEF file");
}
