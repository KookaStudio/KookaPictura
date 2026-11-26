//! Independent PICT oracles: `magick` decodes the engine writer's output, and
//! the engine decoder reads `magick`'s output. Self-skips when `magick` is
//! absent.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_codec::{decode_pict, save_document, ImageBudget, SaveKind, SaveOptions};
use pictura_core::Document;

static NEXT: AtomicU64 = AtomicU64::new(1);

fn tool(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pictura-pict-oracle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn picture() -> (Vec<u8>, u32, u32) {
    let (width, height) = (4u32, 3u32);
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[
                (x * 60 + 10) as u8,
                (y * 70 + 20) as u8,
                ((x + y) * 40) as u8,
                255,
            ]);
        }
    }
    (rgba, width, height)
}

fn write_ppm(path: &Path, rgba: &[u8], width: u32, height: u32) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).unwrap();
}

fn magick_rgba(path: &Path) -> Vec<u8> {
    let out = Command::new("magick")
        .arg(format!("{}[0]", path.display()))
        .args(["-depth", "8", "rgba:-"])
        .output()
        .expect("run magick");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn close(a: u8, b: u8) -> bool {
    i32::from(a).abs_diff(i32::from(b)) <= 4
}

#[test]
fn magick_decodes_pict() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let (rgba, width, height) = picture();
    let doc = Document::from_rgba("pict", width, height, &rgba);
    for (name, options) in [
        ("pictura.pct", SaveOptions::default()),
        (
            "pictura-jpeg.pct",
            SaveOptions {
                pict_compression: 4,
                ..SaveOptions::default()
            },
        ),
    ] {
        let path = temp_path(name);
        save_document(SaveKind::Pict, &doc, &options, &path).expect("save");
        let decoded = magick_rgba(&path);
        assert_eq!(decoded.len(), (width * height * 4) as usize);
        for (index, (a, b)) in rgba.iter().zip(decoded.iter()).enumerate() {
            assert!(close(*a, *b), "{name} byte {index}: {a} != {b}");
        }
    }
}

#[test]
fn pict_decodes_magick_output() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let (rgba, width, height) = picture();
    let ppm = temp_path("pictura.ppm");
    let pct = temp_path("magick.pct");
    write_ppm(&ppm, &rgba, width, height);
    let out = Command::new("magick")
        .arg(&ppm)
        .arg(&pct)
        .output()
        .expect("run magick");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&pct).expect("read pct");
    let image = decode_pict(&bytes, ImageBudget::default()).expect("decode");
    assert_eq!((image.width, image.height), (width, height));
    for (index, (a, b)) in rgba.iter().zip(image.rgba.iter()).enumerate() {
        assert!(close(*a, *b), "byte {index}: {a} != {b}");
    }
}
