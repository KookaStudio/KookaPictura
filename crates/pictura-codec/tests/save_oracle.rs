//! Independent decoder oracles for the Save As writers (`magick` for raster
//! containers, Ghostscript for EPS/DCS). Self-skips when a tool is absent.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_codec::{save_document, save_document_with_jpeg, SaveKind, SaveOptions};
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
        "pictura-save-oracle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn doc() -> Document {
    // Four distinct pixels; non-uniform so a plane/interleave mix-up shows.
    let rgba = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
    ];
    Document::from_rgba("oracle", 2, 2, &rgba)
}

fn decode_rgba(path: &Path) -> Vec<u8> {
    // ImageMagick exposes Photoshop's layer stack as extra frames; the
    // flattened composite is frame 0.
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
    i32::from(a).abs_diff(i32::from(b)) <= 2
}

#[test]
fn magick_decodes_the_raster_containers() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    for (kind, suffix, options) in [
        (SaveKind::Png, "png", SaveOptions::default()),
        (
            SaveKind::Bmp,
            "bmp",
            SaveOptions {
                bmp_depth: 24,
                ..SaveOptions::default()
            },
        ),
        (
            SaveKind::Tga,
            "tga",
            SaveOptions {
                tga_depth: 32,
                ..SaveOptions::default()
            },
        ),
        (
            SaveKind::Tiff,
            "tif",
            SaveOptions {
                tiff_compression: 0,
                ..SaveOptions::default()
            },
        ),
        (SaveKind::Gif, "gif", SaveOptions::default()),
    ] {
        let path = temp_path(&format!("pixels.{suffix}"));
        save_document(kind, &source, &options, &path).expect("save");
        let rgba = decode_rgba(&path);
        assert_eq!(rgba.len(), 2 * 2 * 4, "{suffix}");
        for (i, expected) in [255u8, 0, 0].iter().enumerate() {
            assert!(close(rgba[i], *expected), "{suffix} red pixel {i}");
        }
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn magick_decodes_bmp_rle4_and_rle8() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let mut pixels = Vec::new();
    for i in 0..8u8 {
        pixels.extend_from_slice(&[i * 32, 255 - i * 32, i * 16, 255]);
    }
    let source = Document::from_rgba("rle", 8, 1, &pixels);
    for depth in [4u8, 8] {
        let path = temp_path(&format!("rle{depth}.bmp"));
        let options = SaveOptions {
            bmp_depth: depth,
            bmp_rle: true,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Bmp, &source, &options, &path).expect("save");
        let decoded = decode_rgba(&path);
        assert_eq!(decoded.len(), 8 * 4);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn ghostscript_opens_each_eps_encoding() {
    if !tool("gs") {
        eprintln!("skipping: Ghostscript not found");
        return;
    }
    let source = doc();
    for encoding in 0..3u8 {
        let path = temp_path(&format!("encoding{encoding}.eps"));
        let options = SaveOptions {
            eps_encoding: encoding,
            eps_preview: 0,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Eps, &source, &options, &path).expect("save");
        let png = path.with_extension("png");
        let out = Command::new("gs")
            .args(["-q", "-dNOPAUSE", "-dBATCH", "-dEPSCrop", "-sDEVICE=png16m"])
            .arg(format!("-sOutputFile={}", png.display()))
            .arg(&path)
            .output()
            .expect("run gs");
        assert!(
            out.status.success(),
            "encoding {encoding}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(png.is_file(), "encoding {encoding} produced no page");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn ghostscript_opens_dcs_files() {
    if !tool("gs") {
        eprintln!("skipping: Ghostscript not found");
        return;
    }
    let source = doc();
    let path = temp_path("set.eps");
    let options = SaveOptions {
        dcs_version: 1,
        dcs_preview: 0,
        eps_encoding: 2,
        ..SaveOptions::default()
    };
    save_document(SaveKind::Dcs, &source, &options, &path).expect("save");
    for suffix in ["eps", "C", "M", "Y", "K"] {
        let file = path.with_extension(suffix);
        assert!(file.is_file(), "{suffix} missing");
        let out = Command::new("gs")
            .args([
                "-q",
                "-dNOPAUSE",
                "-dBATCH",
                "-dEPSCrop",
                "-sDEVICE=nullpage",
            ])
            .arg(&file)
            .output()
            .expect("run gs");
        assert!(
            out.status.success(),
            "{suffix}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn magick_identifies_cineon() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    let path = temp_path("frame.cin");
    save_document(SaveKind::Cineon, &source, &SaveOptions::default(), &path).expect("save");
    let out = Command::new("magick")
        .arg(&path)
        .args(["-format", "%w %h %[channels]", "info:"])
        .output()
        .expect("run magick");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("2 2"), "unexpected Cineon header: {text}");
    let rgba = decode_rgba(&path);
    assert_eq!(rgba.len(), 2 * 2 * 4);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn magick_decodes_layered_tiff_flat_image() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let mut source = doc();
    let mut layer = source.layers.get(0).expect("layer").clone();
    layer.name = "Second".to_string();
    source.layers = pictura_core::LayerStack::from_layers(vec![layer.clone(), layer]);
    let path = temp_path("layers.tif");
    save_document(SaveKind::Tiff, &source, &SaveOptions::default(), &path).expect("save");
    let rgba = decode_rgba(&path);
    assert_eq!(rgba.len(), 2 * 2 * 4);
    // ImageMagick exposes the layers from tag 37724 as extra frames; the
    // composite plus two layers means at least three pages.
    let out = Command::new("magick")
        .arg(&path)
        .args(["-format", "%n", "info:"])
        .output()
        .expect("run magick");
    let frames: usize = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    assert!(frames >= 3, "layered TIFF exposed {frames} frames");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn magick_decodes_tiff_jpeg() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    let jpeg = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/one.jpg"
    ))
    .unwrap();
    let encode = |_: &[u8], _: u32, _: u32, _: u8, _: u8| Some(jpeg.clone());
    let path = temp_path("jpeg.tif");
    let options = SaveOptions {
        tiff_compression: 4,
        tiff_transparency: false,
        ..SaveOptions::default()
    };
    save_document_with_jpeg(SaveKind::Tiff, &source, &options, &path, Some(&encode)).expect("save");
    let rgba = decode_rgba(&path);
    assert_eq!(rgba.len(), 2 * 2 * 4);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn ghostscript_opens_eps_jpeg() {
    if !tool("gs") {
        eprintln!("skipping: Ghostscript not found");
        return;
    }
    let source = Document::from_rgba("oracle", 1, 1, &[255, 0, 0, 255]);
    let jpeg = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/one.jpg"
    ))
    .unwrap();
    let encode = |_: &[u8], _: u32, _: u32, _: u8, _: u8| Some(jpeg.clone());
    let path = temp_path("jpeg.eps");
    let options = SaveOptions {
        eps_encoding: 3,
        eps_preview: 0,
        ..SaveOptions::default()
    };
    save_document_with_jpeg(SaveKind::Eps, &source, &options, &path, Some(&encode)).expect("save");
    let png = path.with_extension("png");
    let out = Command::new("gs")
        .args(["-q", "-dNOPAUSE", "-dBATCH", "-dEPSCrop", "-sDEVICE=png16m"])
        .arg(format!("-sOutputFile={}", png.display()))
        .arg(&path)
        .output()
        .expect("run gs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn ghostscript_opens_dos_eps_preview() {
    if !tool("gs") {
        eprintln!("skipping: Ghostscript not found");
        return;
    }
    let source = doc();
    for preview in [1u8, 2] {
        let path = temp_path(&format!("preview{preview}.eps"));
        let options = SaveOptions {
            eps_preview: preview,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Eps, &source, &options, &path).expect("save");
        let out = Command::new("gs")
            .args([
                "-q",
                "-dNOPAUSE",
                "-dBATCH",
                "-dEPSCrop",
                "-sDEVICE=nullpage",
            ])
            .arg(&path)
            .output()
            .expect("run gs");
        assert!(
            out.status.success(),
            "preview {preview}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn magick_decodes_bmp_16_layouts_and_tga_depths() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    for advanced in 0..3u8 {
        let path = temp_path(&format!("bmp16-{advanced}.bmp"));
        let options = SaveOptions {
            bmp_depth: 16,
            bmp_advanced: advanced,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Bmp, &source, &options, &path).expect("save");
        let rgba = decode_rgba(&path);
        assert_eq!(rgba.len(), 2 * 2 * 4);
        assert!(
            close(rgba[0], 255) && close(rgba[1], 0) && close(rgba[2], 0),
            "layout {advanced}"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
    for depth in [16u8, 24, 32] {
        let path = temp_path(&format!("depth{depth}.tga"));
        let options = SaveOptions {
            tga_depth: depth,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Tga, &source, &options, &path).expect("save");
        let rgba = decode_rgba(&path);
        assert_eq!(rgba.len(), 2 * 2 * 4);
        assert!(
            close(rgba[0], 255) && close(rgba[1], 0) && close(rgba[2], 0),
            "TGA {depth}"
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn magick_preserves_gif_transparency() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = Document::from_rgba("gif", 2, 1, &[255, 0, 0, 0, 0, 0, 255, 255]);
    let path = temp_path("alpha.gif");
    save_document(SaveKind::Gif, &source, &SaveOptions::default(), &path).expect("save");
    let rgba = decode_rgba(&path);
    assert_eq!(rgba.len(), 2 * 4);
    assert_eq!(rgba[3], 0, "transparent index survives");
    assert_eq!(rgba[7], 255);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn magick_decodes_tiff_compressions() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    for compression in 0..4u8 {
        let path = temp_path(&format!("compression{compression}.tif"));
        let options = SaveOptions {
            tiff_compression: compression,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Tiff, &source, &options, &path).expect("save");
        let rgba = decode_rgba(&path);
        assert_eq!(rgba.len(), 2 * 2 * 4, "compression {compression}");
        assert!(close(rgba[0], 255) && close(rgba[1], 0) && close(rgba[2], 0));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn magick_decodes_planar_and_big_endian_tiff() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let mut source = doc();
    source.layers = pictura_core::LayerStack::default();
    for (planar, mac) in [(true, false), (false, true)] {
        let path = temp_path(&format!("layout-{planar}-{mac}.tif"));
        let options = SaveOptions {
            tiff_compression: 0,
            tiff_pixel_order: planar,
            tiff_byte_order_mac: mac,
            ..SaveOptions::default()
        };
        save_document(SaveKind::Tiff, &source, &options, &path).expect("save");
        let rgba = decode_rgba(&path);
        assert_eq!(rgba.len(), 2 * 2 * 4);
        assert!(close(rgba[0], 255) && close(rgba[1], 0) && close(rgba[2], 0));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

#[test]
fn magick_decodes_png_adam7() {
    if !tool("magick") {
        eprintln!("skipping: magick not found");
        return;
    }
    let source = doc();
    let path = temp_path("adam7.png");
    let options = SaveOptions {
        png_interlace: true,
        ..SaveOptions::default()
    };
    save_document(SaveKind::Png, &source, &options, &path).expect("save");
    let rgba = decode_rgba(&path);
    assert_eq!(rgba.len(), 2 * 2 * 4);
    assert!(close(rgba[0], 255) && close(rgba[1], 0) && close(rgba[2], 0));
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
