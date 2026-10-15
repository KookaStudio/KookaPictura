//! Independent ImageMagick oracle for the native WebP codec.
//!
//! A lossless WebP we encode is decoded by `magick` and compared to the source
//! pixel for pixel; a lossy WebP `magick` writes is decoded by us and compared
//! to the source within a lossy tolerance.
//!
//! Needs `magick` (ImageMagick 7); self-skips with a message otherwise.

use std::path::PathBuf;
use std::process::Command;

use pictura_codec::{decode_webp, encode_webp, ImageBudget};
use pictura_testkit::compare;

fn magick_available() -> bool {
    Command::new("magick")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn scratch(tag: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "pictura-webp-oracle-{}-{tag}.{ext}",
        std::process::id()
    ))
}

/// A 64 x 48 smooth opaque gradient, so a lossy encode stays close to it.
fn opaque_picture() -> (Vec<u8>, u32, u32) {
    let (w, h) = (64u32, 48u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&[(x * 4) as u8, (y * 5) as u8, ((x + y) * 2) as u8, 255]);
        }
    }
    (rgba, w, h)
}

/// A 16 x 16 field with a transparent corner, for the lossless alpha check.
fn alpha_picture() -> (Vec<u8>, u32, u32) {
    let (w, h) = (16u32, 16u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let clear = x < 4 && y < 4;
            rgba.extend_from_slice(&[
                (x * 16) as u8,
                (y * 16) as u8,
                ((x * y) % 256) as u8,
                if clear { 0 } else { 255 },
            ]);
        }
    }
    (rgba, w, h)
}

/// Decode a WebP `bytes` with ImageMagick to raw RGBA.
fn magick_decode_webp(bytes: &[u8]) -> Vec<u8> {
    let path = scratch("decode", "webp");
    std::fs::write(&path, bytes).expect("write temp webp");
    let out = Command::new("magick")
        .arg(format!("webp:{}", path.display()))
        .args(["-depth", "8", "rgba:-"])
        .output()
        .expect("run magick");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// Have ImageMagick write a lossy WebP at `quality` from raw RGBA.
fn magick_encode_lossy_webp(rgba: &[u8], w: u32, h: u32, quality: u32) -> Vec<u8> {
    let raw = scratch("raw", "rgba");
    let webp = scratch("lossy", "webp");
    std::fs::write(&raw, rgba).expect("write temp raw");
    let out = Command::new("magick")
        .args(["-size", &format!("{w}x{h}"), "-depth", "8"])
        .arg(format!("rgba:{}", raw.display()))
        .arg("-quality")
        .arg(quality.to_string())
        .arg(format!("webp:{}", webp.display()))
        .output()
        .expect("run magick");
    let _ = std::fs::remove_file(&raw);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&webp).expect("read temp webp");
    let _ = std::fs::remove_file(&webp);
    bytes
}

#[test]
fn magick_decodes_our_lossless_webp_exactly() {
    if !magick_available() {
        eprintln!("skipping: magick not available");
        return;
    }
    let (rgba, w, h) = alpha_picture();
    let webp = encode_webp(&rgba, w, h).expect("encode");
    let decoded = magick_decode_webp(&webp);
    let diff = compare(&decoded, &rgba, 0).expect("compare");
    assert!(diff.is_empty(), "lossless webp differs: {diff:?}");
}

#[test]
fn our_decoder_reads_a_lossy_webp_within_tolerance() {
    if !magick_available() {
        eprintln!("skipping: magick not available");
        return;
    }
    let (rgba, w, h) = opaque_picture();
    let lossy = magick_encode_lossy_webp(&rgba, w, h, 90);
    let (decoded, dw, dh) = decode_webp(&lossy, ImageBudget::default()).expect("decode lossy");
    assert_eq!((dw, dh), (w, h));
    let diff = compare(&decoded, &rgba, 48).expect("compare");
    assert!(diff.is_empty(), "lossy webp differs: {diff:?}");
}

#[test]
fn the_oracle_has_teeth() {
    if !magick_available() {
        eprintln!("skipping: magick not available");
        return;
    }
    let (rgba, w, h) = alpha_picture();
    let webp = encode_webp(&rgba, w, h).expect("encode");
    let decoded = magick_decode_webp(&webp);
    // A single wrong byte must not compare clean.
    let mut wrong = rgba.clone();
    wrong[0] ^= 0xFF;
    let diff = compare(&decoded, &wrong, 0).expect("compare");
    assert!(!diff.is_empty(), "a wrong image compared clean");
}
