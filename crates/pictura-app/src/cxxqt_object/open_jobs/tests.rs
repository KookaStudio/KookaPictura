//! Unit coverage for the open worker's decode/apply split (#729).

use super::*;
use pictura_core::{BitDepth, ColorMode, Document};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_path(extension: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "pictura-open-jobs-{}-{n}.{extension}",
        std::process::id()
    ))
}

fn white_psd() -> (std::path::PathBuf, Vec<u8>) {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data.fill(255);
    let bytes = pictura_codec::write_psd(&doc).expect("write PSD");
    let path = temp_path("psd");
    std::fs::write(&path, &bytes).expect("write fixture");
    (path, bytes)
}

fn rgba_document() -> Vec<u8> {
    let mut rgba = vec![0u8; 2 * 4];
    rgba[..4].copy_from_slice(&[255, 0, 0, 255]);
    rgba[4..].copy_from_slice(&[0, 255, 0, 255]);
    rgba
}

#[test]
fn open_psd_matches_sync() {
    let (path, _) = white_psd();
    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    assert_eq!(opened.doc.width, 4);
    assert_eq!(opened.doc.height, 4);
    assert!(matches!(&opened.path, OpenedPath::Set(Some(p)) if p == path.to_str().unwrap()));
    assert_eq!(opened.source_format, "psd");

    let mut rust = PictureViewRust::default();
    apply_opened(&mut rust, opened);
    assert!(rust.doc.is_some());
    assert_eq!(rust.path.as_deref(), Some(path.to_str().unwrap()));
    assert!(!rust.dirty);
    assert_eq!(rust.source_format, "psd");
    std::fs::remove_file(path).ok();
}

#[test]
fn open_psd_reports_phases() {
    let (path, _) = white_psd();
    let phases = std::cell::RefCell::new(Vec::new());
    let _ = decode_psd(
        path.to_str().unwrap(),
        Policy::Preserve,
        false,
        &|percent, phase| phases.borrow_mut().push((percent, phase.to_string())),
    )
    .expect("decode");
    let phases = phases.into_inner();
    assert_eq!(phases.first().map(|(percent, _)| *percent), Some(5));
    assert!(phases
        .iter()
        .any(|(percent, phase)| *percent == 25 && phase == "Decoding"));
    assert!(phases
        .iter()
        .any(|(percent, phase)| *percent == 65 && phase == "Compositing"));
    assert!(phases
        .iter()
        .any(|(percent, phase)| *percent == 90 && phase == "Preparing layers"));
    assert!(phases.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    std::fs::remove_file(path).ok();
}

#[test]
fn open_image_matches_sync() {
    let rgba = rgba_document();
    let webp = pictura_codec::encode_webp(&rgba, 2, 1).expect("encode WebP");
    let path = temp_path("webp");
    std::fs::write(&path, &webp).expect("write fixture");

    let opened = decode_image(path.to_str().unwrap(), false, &|_, _| {}).expect("decode");
    assert_eq!(opened.doc.width, 2);
    assert_eq!(opened.doc.height, 1);
    assert!(matches!(opened.path, OpenedPath::Keep));
    assert_eq!(opened.source_format, "webp");
    assert_eq!(opened.target.single(), Some("0"));

    let mut rust = PictureViewRust::default();
    apply_opened(&mut rust, opened);
    assert!(rust.doc.is_some());
    assert_eq!(rust.path, None);
    assert!(!rust.dirty);
    assert_eq!(rust.source_format, "webp");
    std::fs::remove_file(path).ok();
}

#[test]
fn open_smart_object_matches_sync() {
    let (path, _) = white_psd();
    let opened =
        decode_smart_object(path.to_str().unwrap(), false, &|_, _| {}).expect("smart object");
    assert_eq!(opened.doc.layers.len(), 1);
    assert!(opened.doc.layers[0].smart_object.is_some());
    assert!(matches!(opened.path, OpenedPath::Set(None)));
    assert_eq!(opened.source_format, "psd");
    std::fs::remove_file(path).ok();
}

#[test]
fn import_render_matches_sync() {
    let raster = Raster {
        width: 2,
        height: 1,
        rgba: rgba_document(),
    };
    let spec = ImportSpec {
        width: 2,
        height: 1,
        anti_alias: true,
        mode: "rgb".to_string(),
        depth: 8,
        ppi: 300.0,
        per_cm: false,
        name: "page.png".to_string(),
    };
    let opened = imported_document(&raster, &spec, false, &|_, _| {}).expect("import");
    assert_eq!(opened.doc.width, 2);
    assert_eq!(opened.doc.height, 1);
    assert_eq!(opened.doc.layers[0].name, "Layer 1");
    assert!(matches!(opened.path, OpenedPath::Set(None)));
    assert_eq!(opened.target.single(), Some("0"));
    assert_eq!(opened.source_format, "psd");
}

#[test]
fn cancelled_open_result_is_dropped() {
    let (path, _) = white_psd();
    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    let mut rust = PictureViewRust {
        open_cancelled: true,
        ..PictureViewRust::default()
    };
    assert!(!apply_if_live(&mut rust, Some(opened)));
    assert!(rust.doc.is_none());

    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    rust.open_cancelled = false;
    assert!(apply_if_live(&mut rust, Some(opened)));
    assert!(rust.doc.is_some());
    std::fs::remove_file(path).ok();
}

#[test]
fn pdf_open_status_classifies() {
    let path = temp_path("pdf");
    std::fs::write(&path, b"%PDF-1.4\n%%EOF\n").expect("write fixture");
    assert!(matches!(
        decode_pdf(path.to_str().unwrap(), "", Policy::Preserve, false),
        PdfOpen::None
    ));
    std::fs::remove_file(&path).ok();
    assert!(matches!(
        decode_pdf(path.to_str().unwrap(), "", Policy::Preserve, false),
        PdfOpen::None
    ));
}
