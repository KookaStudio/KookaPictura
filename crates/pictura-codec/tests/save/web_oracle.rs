//! Independent ImageMagick oracle for the Save for Web encoders: GIFs (plain
//! and interlaced, with a transparent entry) and a WBMP written by
//! `pictura_codec` are decoded by `magick` and compared pixel for pixel with
//! the indexed image they were written from.
//!
//! Needs `magick` (ImageMagick 7); self-skips with a message otherwise.

use std::process::Command;

use pictura_codec::{encode_gif, encode_wbmp, quantize, ColorReduction, Dither, PaletteOptions};

fn magick_available() -> bool {
    Command::new("magick")
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Decode `bytes` (in `format`) with ImageMagick to raw RGBA. The bytes go
/// through a file, not stdin: newer ImageMagick's WBMP reader checks the
/// pixel count against the input's size, which a pipe reports as zero.
fn decode(format: &str, bytes: &[u8]) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "pictura-web-oracle-{}-{}.{format}",
        std::process::id(),
        bytes.len()
    ));
    std::fs::write(&path, bytes).expect("write temp file");
    let out = Command::new("magick")
        .arg(format!("{format}:{}", path.display()))
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

/// A 40 x 30 colour field with a clear 10 x 10 hole.
fn picture() -> (Vec<u8>, u32, u32) {
    let (w, h) = (40u32, 30u32);
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let clear = (5..15).contains(&x) && (5..15).contains(&y);
            rgba.extend_from_slice(&[
                (x * 6) as u8,
                (y * 8) as u8,
                ((x + y) * 3) as u8,
                if clear { 0 } else { 255 },
            ]);
        }
    }
    (rgba, w, h)
}

#[test]
fn magick_decodes_our_gifs_exactly() {
    if !magick_available() {
        eprintln!("skipping: magick not available");
        return;
    }
    let (rgba, w, h) = picture();
    let options = PaletteOptions {
        reduction: ColorReduction::Selective,
        colors: 64,
        dither: Dither::Diffusion,
        amount: 100,
        transparency: true,
        mapped_transparency: false,
        matte: [255, 255, 255],
    };
    let indexed = quantize(&rgba, w, h, options);
    let clear = indexed.transparent_index().expect("a transparent entry");
    for interlaced in [false, true] {
        let decoded = decode("gif", &encode_gif(&indexed, interlaced));
        assert_eq!(decoded.len(), (w * h * 4) as usize);
        for (i, &index) in indexed.indices.iter().enumerate() {
            let got = &decoded[i * 4..i * 4 + 4];
            if index == clear {
                assert_eq!(
                    got[3], 0,
                    "pixel {i} is transparent (interlaced {interlaced})"
                );
            } else {
                let want = indexed.palette[usize::from(index)];
                assert_eq!(got, &want[..], "pixel {i} (interlaced {interlaced})");
            }
        }
    }
}

#[test]
fn magick_decodes_our_wbmp_exactly() {
    if !magick_available() {
        eprintln!("skipping: magick not available");
        return;
    }
    let (rgba, w, h) = picture();
    let wbmp = encode_wbmp(&rgba, w, h, Dither::Pattern, 80);
    let decoded = decode("wbmp", &wbmp);
    assert_eq!(decoded.len(), (w * h * 4) as usize);
    let row = w.div_ceil(8) as usize;
    let header = 4;
    for y in 0..h as usize {
        for x in 0..w as usize {
            let bit = wbmp[header + y * row + x / 8] & (0x80 >> (x % 8)) != 0;
            let px = &decoded[(y * w as usize + x) * 4..][..3];
            assert_eq!(px, if bit { &[255u8; 3] } else { &[0u8; 3] }, "({x}, {y})");
        }
    }
}
