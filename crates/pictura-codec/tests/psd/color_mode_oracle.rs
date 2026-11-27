//! Differential tests for the Bitmap, Indexed, CMYK, and Lab color modes.
//!
//! `read_psd` normalizes each to RGB; the committed fixtures (authored by
//! `scripts/generate-fixtures.py`) are re-read here with the independent
//! `psd-tools` decoder and compared pixel for pixel. The oracle self-skips when
//! `psd-tools` is absent.

use std::path::PathBuf;

use pictura_codec::{read_psd, write_psd};
use pictura_core::{BitDepth, ColorMode, Compression, Document};
use pictura_testkit::oracle;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(name: &str) -> Document {
    let path = fixture_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `python3 scripts/generate-fixtures.py`",
            path.display()
        )
    });
    read_psd(&bytes).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()))
}

/// Decode `path` with `psd-tools` and return its composite as planar RGB hex.
fn psd_tools_composite_planes(path: &std::path::Path) -> Vec<String> {
    let script = r#"
import sys
import numpy as np
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1], lazy=False)
arr = np.array(psd.composite().convert("RGB"))
for c in range(3):
    print(arr[:, :, c].tobytes().hex())
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools failed on {}:\n{}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

/// Decode `path` with `psd-tools` and convert its Lab composite to RGB with
/// lcms2's exact (unoptimized) transform. psd-tools' own `.convert("RGB")` uses
/// an optimized color LUT that can differ by up to ~20 on some in-gamut colors,
/// so the exact transform is the reference for the profile-free Lab formula.
fn lcms2_exact_lab_planes(path: &std::path::Path) -> Vec<String> {
    let script = r#"
import sys
from psd_tools import PSDImage
from PIL import ImageCms
psd = PSDImage.open(sys.argv[1], lazy=False)
lab = psd.composite()
lab_prof = ImageCms.createProfile("LAB")
srgb = ImageCms.createProfile("sRGB")
tr = ImageCms.buildTransform(
    lab_prof, srgb, "LAB", "RGB", flags=ImageCms.Flags.NOOPTIMIZE
)
rgb = tr.apply(lab)
for c in range(3):
    print(rgb.getchannel(c).tobytes().hex())
"#;
    let out = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "psd-tools/lcms2 failed on {}:\n{}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn assert_planes_within(label: &str, planes: &[&[u8]], reference: &[String], tolerance: u8) {
    assert_eq!(
        reference.len(),
        planes.len(),
        "{label}: reference plane count"
    );
    for (c, (ours, expected_hex)) in planes.iter().zip(reference.iter()).enumerate() {
        let expected: Vec<u8> = (0..expected_hex.len() / 2)
            .map(|i| u8::from_str_radix(&expected_hex[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        assert_eq!(ours.len(), expected.len(), "{label}: plane {c} length");
        for (i, (ours, theirs)) in ours.iter().zip(expected.iter()).enumerate() {
            let diff = (*ours as i32 - *theirs as i32).abs();
            assert!(
                diff <= tolerance as i32,
                "{label}: plane {c} pixel {i} ours={ours} reference={theirs} diff={diff}"
            );
        }
    }
}

fn ours_matching(name: &str, tolerance: u8, reference: fn(&std::path::Path) -> Vec<String>) {
    let path = fixture_dir().join(name);
    let doc = load(name);
    let data = &doc.composite.data;
    let plane = doc.composite.pixel_count();
    let our_planes: Vec<&[u8]> = (0..3).map(|c| &data[c * plane..(c + 1) * plane]).collect();
    let theirs = reference(&path);
    assert_planes_within(name, &our_planes, &theirs, tolerance);
}

#[test]
fn indexed_fixture_matches_psd_tools() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    // Palette expansion is exact: no tolerance.
    let path = fixture_dir().join("indexed.psd");
    let doc = load("indexed.psd");
    let plane = doc.composite.pixel_count();
    let theirs = psd_tools_composite_planes(&path);
    for (c, expected_hex) in theirs.iter().enumerate() {
        assert_eq!(
            hex(&doc.composite.data[c * plane..(c + 1) * plane]),
            *expected_hex
        );
    }
}

#[test]
fn cmyk_fixture_matches_psd_tools_within_one() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    // psd-tools/Pillow rounds `color * black / 255`; the engine floors it, a
    // <=1 difference the varied fixture planes make visible.
    ours_matching("cmyk.psd", 1, psd_tools_composite_planes);
    let doc = load("cmyk.psd");
    assert_eq!(doc.layers.len(), 1, "cmyk.psd carries one pixel layer");
    assert_eq!(
        doc.layers[0]
            .channels()
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, -1],
        "the CMYK layer becomes three RGB channels plus transparency"
    );
}

#[test]
fn lab_fixture_matches_lcms2_exact_within_one() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    // The exact lcms2 transform, not psd-tools' optimized-LUT `.convert("RGB")`.
    ours_matching("lab.psd", 1, lcms2_exact_lab_planes);
}

#[test]
fn bitmap_fixture_matches_psd_tools() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    ours_matching("bitmap.psd", 0, psd_tools_composite_planes);
}

/// A flat, unchanged Bitmap document saves back as Bitmap: the output header is
/// mode 0, depth 1, one color channel, and the retained packed plane re-emits
/// byte-identically, so a re-read reproduces the working RGB. `psd-tools` opens
/// the written file as mode 0.
#[test]
fn bitmap_document_saves_as_bitmap() {
    let name = "bitmap.psd";
    let doc = load(name);
    assert_eq!(doc.mode, ColorMode::Rgb, "{name}: working mode");
    assert_eq!(
        doc.source.mode(),
        Some(ColorMode::Bitmap),
        "{name}: recorded source mode"
    );
    assert_eq!(doc.depth, BitDepth::Eight, "{name}: normalized depth");
    let store = doc.source.planes().as_ref().expect("packed plane retained");
    assert_eq!(store.depth, BitDepth::One, "{name}: depth-1 store");
    let packed = store.samples.to_bytes();
    assert_eq!(
        packed.len(),
        doc.height as usize,
        "{name}: one packed row byte per row"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "{name}: output has one color channel"
    );
    assert_eq!(
        u16::from_be_bytes(out[22..24].try_into().unwrap()),
        1,
        "{name}: output depth is 1"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        0,
        "{name}: output header color mode is Bitmap"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb, "{name}: re-read working mode");
    assert_eq!(
        back.source.mode(),
        Some(ColorMode::Bitmap),
        "{name}: re-read source mode"
    );
    assert_eq!(
        back.source
            .planes()
            .as_ref()
            .expect("re-read packed")
            .samples
            .to_bytes(),
        packed,
        "{name}: the written packed plane is byte-identical"
    );
    assert_eq!(
        back.composite, doc.composite,
        "{name}: re-reading the packed plane reproduces the working RGB exactly"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("bitmap-write-back");
    let path = dir.join(name);
    std::fs::write(&path, &out).unwrap();
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(int(psd.color_mode))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.status.success(),
        "psd-tools failed on the written {name}:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "0",
        "{name}: psd-tools opens the output as Bitmap"
    );
}

/// A flat depth-1 Bitmap PSD (`mode 0`) with hand-built RLE image data: one
/// pre-encoded PackBits stream per scanline, `rows` in order.
fn bitmap_rle_psd(width: u32, rows: &[&[u8]]) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(b"8BPS");
    p.extend_from_slice(&1u16.to_be_bytes());
    p.extend_from_slice(&[0u8; 6]);
    p.extend_from_slice(&1u16.to_be_bytes()); // channels
    p.extend_from_slice(&(rows.len() as u32).to_be_bytes()); // height
    p.extend_from_slice(&width.to_be_bytes());
    p.extend_from_slice(&1u16.to_be_bytes()); // depth
    p.extend_from_slice(&0u16.to_be_bytes()); // mode Bitmap
    p.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    p.extend_from_slice(&0u32.to_be_bytes()); // image resources
    p.extend_from_slice(&0u32.to_be_bytes()); // layer/mask section
    p.extend_from_slice(&1u16.to_be_bytes()); // RLE
    for row in rows {
        p.extend_from_slice(&(row.len() as u16).to_be_bytes());
    }
    for row in rows {
        p.extend_from_slice(row);
    }
    p
}

/// The byte offset of the image-data compression word: skip the 26-byte header
/// and the color-mode-data, image-resource, and layer/mask sections.
fn image_data_offset(psd: &[u8]) -> usize {
    let mut o = 26;
    for _ in 0..3 {
        let len = u32::from_be_bytes(psd[o..o + 4].try_into().unwrap()) as usize;
        o += 4 + len;
    }
    o
}

/// A flat unchanged depth-1 Bitmap source read from RLE keeps its source
/// compression on save: the output compression word is RLE (not Raw), the
/// retained packed plane is byte-identical, a re-read matches the original RGB,
/// and psd-tools opens the output as mode 0.
#[test]
fn bitmap_rle_document_saves_as_bitmap() {
    // A whole-byte-run packed plane: row 0 is one 3-copy run, row 1 a 3-byte
    // literal, so both PackBits arms are exercised.
    let rows: [&[u8]; 2] = [&[0xFE, 0xF0], &[0x02, 0x0F, 0xC3, 0x8A]];
    let packed = [0xF0u8, 0xF0, 0xF0, 0x0F, 0xC3, 0x8A];
    let doc = read_psd(&bitmap_rle_psd(24, &rows)).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb, "working mode");
    assert_eq!(doc.source.mode(), Some(ColorMode::Bitmap), "source mode");
    assert_eq!(
        doc.composite_compression,
        Compression::Rle,
        "read compression"
    );
    assert_eq!(
        doc.source
            .planes()
            .as_ref()
            .expect("packed plane retained")
            .samples
            .to_bytes(),
        packed,
        "the RLE rows decode to the known bytes"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        0,
        "output header color mode is Bitmap"
    );
    let off = image_data_offset(&out);
    assert_eq!(
        u16::from_be_bytes(out[off..off + 2].try_into().unwrap()),
        1,
        "the output source compression is RLE, not Raw"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(
        back.source
            .planes()
            .as_ref()
            .expect("re-read packed")
            .samples
            .to_bytes(),
        packed,
        "the written packed plane is byte-identical"
    );
    assert_eq!(
        back.composite, doc.composite,
        "a re-read reproduces the original working RGB"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("bitmap-rle-write-back");
    let path = dir.join("bitmap_rle.psd");
    std::fs::write(&path, &out).unwrap();
    let script = r#"
import sys
from psd_tools import PSDImage
print(int(PSDImage.open(sys.argv[1]).color_mode))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.status.success(),
        "psd-tools failed on the written RLE Bitmap:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "0",
        "psd-tools opens the RLE output as Bitmap"
    );
}

/// An unchanged Indexed document saves back as Indexed: header mode 2 with one
/// index channel, the retained palette and index plane re-emitted byte-identically,
/// so a re-read reproduces the working RGB exactly. `psd-tools` opens it as mode 2.
#[test]
fn indexed_document_saves_as_indexed() {
    let doc = load("indexed.psd");
    assert_eq!(doc.mode, ColorMode::Rgb, "working mode");
    assert_eq!(
        doc.source.mode(),
        Some(ColorMode::Indexed),
        "recorded source mode"
    );
    assert!(doc.color_mode_data.is_empty(), "palette consumed");
    let palette = doc.source.palette().expect("palette retained");
    let plane = doc.composite.pixel_count();
    let retained = doc.source.planes().clone().expect("index plane retained");
    assert_eq!(retained.depth, BitDepth::Eight, "8-bit index store");
    assert!(
        !doc.retains_source_depth(),
        "the index store is not a native-depth store"
    );
    let source_index = retained.samples.to_bytes()[..plane].to_vec();

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "output has one color channel"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        2,
        "output header color mode is Indexed"
    );
    assert_eq!(
        &out[30..30 + 768],
        &palette[..],
        "palette is byte-identical"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb, "re-read working mode");
    assert_eq!(
        back.source.mode(),
        Some(ColorMode::Indexed),
        "re-read source mode"
    );
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes()[..plane],
        source_index[..],
        "the written index plane is byte-identical"
    );
    assert_eq!(
        back.composite, doc.composite,
        "re-reading the retained palette reproduces the working RGB exactly"
    );

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("indexed-write-back");
    let path = dir.join("indexed.psd");
    std::fs::write(&path, &out).unwrap();
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(int(psd.color_mode))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.status.success(),
        "psd-tools failed on the written indexed.psd:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "2",
        "psd-tools opens the output as Indexed"
    );
}

/// A Lab document saves back as Lab: the output header color mode is Lab, the
/// planes stay three channels, and an **unedited** document re-emits the retained
/// Lab planes byte-identically, so a re-read reproduces the working RGB exactly.
/// lcms2's exact transform of the written Lab planes is the independent
/// reference; lcms2 and psd-tools are test-only.
#[test]
fn lab_document_saves_as_lab() {
    let doc = load("lab.psd");
    assert_eq!(
        doc.source.mode(),
        Some(ColorMode::Lab),
        "recorded source mode"
    );
    let plane = doc.composite.pixel_count();
    let retained = doc.source.planes().clone().expect("Lab planes retained");
    assert_eq!(retained.depth, BitDepth::Eight, "8-bit Lab store");
    assert!(
        !doc.retains_source_depth(),
        "the Lab store is not a native-depth store"
    );
    let source_lab = retained.samples.to_bytes()[..3 * plane].to_vec();

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        3,
        "output keeps three color channels"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        9,
        "output header color mode is Lab"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb, "re-read working mode");
    assert_eq!(
        back.source.mode(),
        Some(ColorMode::Lab),
        "re-read source mode"
    );
    let back_retained = back.source.planes().clone().expect("re-read Lab planes");
    assert_eq!(
        back_retained.samples.to_bytes()[..3 * plane],
        source_lab[..],
        "unedited Lab planes are re-emitted byte-identically"
    );
    assert_eq!(
        back.composite, doc.composite,
        "re-reading the retained planes reproduces the working RGB exactly"
    );
    let working: Vec<&[u8]> = (0..3)
        .map(|c| &doc.composite.data[c * plane..(c + 1) * plane])
        .collect();

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("lab-write-back");
    let path = dir.join("lab.psd");
    std::fs::write(&path, &out).unwrap();
    // lcms2's exact transform of the written Lab planes must reproduce the working
    // RGB the document was editing within tolerance.
    let reference = lcms2_exact_lab_planes(&path);
    assert_planes_within("lab.psd lcms2", &working, &reference, 1);
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(int(psd.color_mode))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.status.success(),
        "psd-tools failed on the written lab.psd:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "9",
        "psd-tools opens the output as Lab"
    );
}

/// A CMYK document saves back as CMYK: the output header color mode is CMYK with
/// four color channels, and an **unedited** document re-emits the retained source
/// planes byte-identically, so a re-read reproduces the working RGB exactly. Our
/// own `read_psd` is the authority for the retained bytes (psd-tools reports mode
/// 4 but its raw channel bytes are `255 - stored`); psd-tools is used for the
/// mode and the independent RGB conversion. lcms2 and psd-tools are test-only.
#[test]
fn cmyk_document_saves_as_cmyk() {
    let doc = load("cmyk.psd");
    assert_eq!(
        doc.source.mode(),
        Some(ColorMode::Cmyk),
        "recorded source mode"
    );
    let plane = doc.composite.pixel_count();
    let retained = doc.source.planes().clone().expect("CMYK planes retained");
    assert_eq!(retained.depth, BitDepth::Eight, "8-bit CMYK store");
    assert!(
        !doc.retains_source_depth(),
        "the CMYK store is not a native-depth store"
    );
    let source_cmyk = retained.samples.to_bytes()[..4 * plane].to_vec();

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        4,
        "output keeps four color channels"
    );
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        4,
        "output header color mode is CMYK"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.mode, ColorMode::Rgb, "re-read working mode");
    assert_eq!(
        back.source.mode(),
        Some(ColorMode::Cmyk),
        "re-read source mode"
    );
    let back_retained = back.source.planes().clone().expect("re-read CMYK planes");
    assert_eq!(
        back_retained.samples.to_bytes()[..4 * plane],
        source_cmyk[..],
        "unedited CMYK planes are re-emitted byte-identically"
    );
    assert_eq!(
        back.composite, doc.composite,
        "re-reading the retained planes reproduces the working RGB exactly"
    );
    let working: Vec<&[u8]> = (0..3)
        .map(|c| &doc.composite.data[c * plane..(c + 1) * plane])
        .collect();

    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let dir = scratch_dir("cmyk-write-back");
    let path = dir.join("cmyk.psd");
    std::fs::write(&path, &out).unwrap();
    // psd-tools' RGB conversion of the written CMYK file agrees with the working
    // RGB within the documented tolerance.
    let reference = psd_tools_composite_planes(&path);
    assert_planes_within("cmyk.psd psd-tools", &working, &reference, 1);
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(int(psd.color_mode))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        result.status.success(),
        "psd-tools failed on the written cmyk.psd:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "4",
        "psd-tools opens the output as CMYK"
    );
}

/// A flat 1x1 PSD at `depth` bits with a raw-compressed composite of `planes`.
fn flat_depth_psd(depth: u16, mode: u16, channels: u16, planes: &[&[u8]]) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(b"8BPS");
    p.extend_from_slice(&1u16.to_be_bytes());
    p.extend_from_slice(&[0u8; 6]);
    p.extend_from_slice(&channels.to_be_bytes());
    p.extend_from_slice(&1u32.to_be_bytes()); // height
    p.extend_from_slice(&1u32.to_be_bytes()); // width
    p.extend_from_slice(&depth.to_be_bytes());
    p.extend_from_slice(&mode.to_be_bytes());
    p.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    p.extend_from_slice(&0u32.to_be_bytes()); // image resources
    p.extend_from_slice(&0u32.to_be_bytes()); // layer/mask section
    p.extend_from_slice(&0u16.to_be_bytes()); // raw compression
    for plane in planes {
        p.extend_from_slice(plane);
    }
    p
}

/// `"<color_mode> <depth>"` as reported by `psd-tools` for the PSD at `path`.
fn psd_tools_mode_and_depth(path: &std::path::Path) -> String {
    let script = r#"
import sys
from psd_tools import PSDImage
psd = PSDImage.open(sys.argv[1])
print(int(psd.color_mode), int(psd.depth))
"#;
    let result = oracle::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(path)
        .output()
        .expect("run python3");
    assert!(
        result.status.success(),
        "psd-tools failed on {}:\n{}",
        path.display(),
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8_lossy(&result.stdout).trim().to_string()
}

/// A 16-bit CMYK or Lab source retains its native samples, so the save keeps the
/// source mode and depth; `psd-tools` opens the output as mode 4/9 at depth 16.
#[test]
fn sixteen_bit_cmyk_and_lab_save_back_in_source_mode_and_depth() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let cmyk = [[0x80u8, 0x00], [0x40, 0x00], [0x20, 0x00], [0xc8, 0x00]];
    let lab = [[0x80u8, 0x00], [0x40, 0x00], [0x20, 0x00]];
    for (mode, planes, expected) in [
        (
            4u16,
            cmyk.iter().map(|p| &p[..]).collect::<Vec<_>>(),
            "4 16",
        ),
        (9u16, lab.iter().map(|p| &p[..]).collect::<Vec<_>>(), "9 16"),
    ] {
        let doc = read_psd(&flat_depth_psd(16, mode, planes.len() as u16, &planes)).unwrap();
        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            16,
            "mode {mode}: output depth"
        );
        assert_eq!(
            u16::from_be_bytes(out[24..26].try_into().unwrap()),
            mode,
            "mode {mode}: output header mode"
        );

        let dir = scratch_dir("color16-write-back");
        let path = dir.join(format!("mode{mode}.psd"));
        std::fs::write(&path, &out).unwrap();
        let got = psd_tools_mode_and_depth(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            got, expected,
            "psd-tools reads mode {mode} at its source depth"
        );
    }
}

/// A 32-bit CMYK or Lab source retains its native samples, so the save keeps the
/// source mode and depth; `psd-tools` opens the output as mode 4/9 at depth 32.
#[test]
fn thirtytwo_bit_cmyk_and_lab_save_back_in_source_mode_and_depth() {
    if let Err(skip) = oracle::require(oracle::Tool::PsdTools) {
        skip.report();
        return;
    }
    let sample = 0.5f32.to_be_bytes();
    let cmyk = [sample; 4];
    let lab = [sample; 3];
    for (mode, planes, expected) in [
        (
            4u16,
            cmyk.iter().map(|p| &p[..]).collect::<Vec<_>>(),
            "4 32",
        ),
        (9u16, lab.iter().map(|p| &p[..]).collect::<Vec<_>>(), "9 32"),
    ] {
        let doc = read_psd(&flat_depth_psd(32, mode, planes.len() as u16, &planes)).unwrap();
        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[22..24].try_into().unwrap()),
            32,
            "mode {mode}: output depth"
        );
        assert_eq!(
            u16::from_be_bytes(out[24..26].try_into().unwrap()),
            mode,
            "mode {mode}: output header mode"
        );

        let dir = scratch_dir("color32-write-back");
        let path = dir.join(format!("mode{mode}.psd"));
        std::fs::write(&path, &out).unwrap();
        let got = psd_tools_mode_and_depth(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            got, expected,
            "psd-tools reads mode {mode} at its source depth"
        );
    }
}

fn scratch_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-codec-color-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
