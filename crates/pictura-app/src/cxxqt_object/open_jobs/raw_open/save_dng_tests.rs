//! Save Image (`document/raw-save-image`, #402): the worker's output, its
//! refusals and cancellation, and the default name, on the minimal DNG of
//! `tests.rs`.

use super::imp::PendingRaw;
use super::save_dng::{
    default_dng_path, save_dng, DngOptions, DngOutcome, DngRequest, PreviewSize,
};
use super::store::StoreKind;
use super::tests::{crs_packet, fixture_dng, pending_with, stores, temp_dir, write};
use pictura_codec::{DngCompatibility, ImageBudget};
use pictura_core::PicturaRawSettings;
use std::path::{Path, PathBuf};

/// The start of the `image` crate's baseline JPEG (SOI then a JFIF APP0);
/// lossless-JPEG tiles start with a DHT instead.
const JFIF: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];

fn options(
    target: PathBuf,
    preview: PreviewSize,
    embed_original: bool,
    linear: bool,
) -> DngOptions {
    DngOptions {
        target,
        compatibility: DngCompatibility::default(),
        preview,
        embed_original,
        linear,
    }
}

fn save(
    pending: &PendingRaw,
    settings: PicturaRawSettings,
    options: DngOptions,
    cancelled: bool,
) -> Result<DngOutcome, String> {
    let request = DngRequest::new(pending, settings, options);
    save_dng(&request, &ImageBudget::default(), &|_| {}, &|| cancelled)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn setup(name: &str) -> (PathBuf, PathBuf, Vec<u8>, PendingRaw) {
    let dir = temp_dir();
    let bytes = fixture_dng(16, 12, true, None);
    let path = write(&dir, name, &bytes);
    let pending = pending_with(&path, &stores(&dir, StoreKind::Database));
    (dir, path, bytes, pending)
}

// raw_save_default_path
#[test]
fn default_dng_path_skips_taken_names() {
    let dir = temp_dir();
    write(&dir, "shot.dng", b"x");
    write(&dir, "shot-1.dng", b"x");
    assert_eq!(
        default_dng_path(&dir.join("shot.dng")),
        dir.join("shot-2.dng")
    );
    let empty = temp_dir();
    assert_eq!(
        default_dng_path(&empty.join("frame.cr2")),
        empty.join("frame.dng")
    );
}

// raw_save_dng_contents
#[test]
fn saved_mosaic_dng_holds_settings_and_original() {
    let (dir, _, bytes, pending) = setup("shot.dng");
    let mut settings = pending.seed.clone();
    settings.basic.exposure = Some(0.5);
    let target = dir.join("out.dng");
    assert_eq!(
        save(
            &pending,
            settings,
            options(target.clone(), PreviewSize::Medium, true, false),
            false
        ),
        Ok(DngOutcome::Written)
    );
    let out = std::fs::read(&target).unwrap();
    let budget = ImageBudget::default();
    let written = pictura_codec::read_dng(&out, &budget).unwrap();
    let source = pictura_codec::read_raw(&bytes, &budget).unwrap();
    assert_eq!(written.samples, source.samples);
    assert_eq!(written.cfa, source.cfa);
    assert_eq!(written.default_crop, source.default_crop);
    assert_eq!(written.orientation, source.orientation);
    assert_eq!(written.color, source.color);
    let xmp = pictura_codec::raw_xmp(&out).expect("settings embedded");
    assert_eq!(
        pictura_codec::pictura_raw_from_xmp(&xmp).basic.exposure,
        Some(0.5)
    );
    assert!(contains(&out, JFIF), "Medium preview written");
    assert_eq!(pictura_codec::read_original_raw(&out), Some(bytes));
}

// raw_save_dng_linear
#[test]
fn saved_linear_dng_holds_camera_rgb() {
    let (dir, _, bytes, pending) = setup("shot.dng");
    let target = dir.join("linear.dng");
    assert_eq!(
        save(
            &pending,
            pending.seed.clone(),
            options(target.clone(), PreviewSize::None, false, true),
            false
        ),
        Ok(DngOutcome::Written)
    );
    let out = std::fs::read(&target).unwrap();
    let budget = ImageBudget::default();
    let written = pictura_codec::read_linear_dng(&out, &budget).unwrap();
    let source = pictura_codec::decode_raw_source(&bytes, &budget).unwrap();
    assert_eq!(written.camera, source.camera);
    assert!(!contains(&out, JFIF), "no preview IFD");
    assert_eq!(pictura_codec::read_original_raw(&out), None);
}

// raw_save_refuses
#[test]
fn save_refuses_source_and_changed_file() {
    let (dir, path, _, pending) = setup("shot.dng");
    let onto_source = save(
        &pending,
        pending.seed.clone(),
        options(path.clone(), PreviewSize::None, false, false),
        false,
    )
    .unwrap_err();
    assert!(onto_source.contains("shot.dng"), "{onto_source}");

    let rewritten = fixture_dng(
        16,
        12,
        true,
        Some(&crs_packet(r#"crs:Exposure2012="+1.00""#)),
    );
    std::fs::write(&path, &rewritten).unwrap();
    let changed = save(
        &pending,
        pending.seed.clone(),
        options(dir.join("out.dng"), PreviewSize::None, false, false),
        false,
    )
    .unwrap_err();
    assert!(changed.contains("changed"), "{changed}");
    assert_eq!(std::fs::read(&path).unwrap(), rewritten);
    assert!(!dir.join("out.dng").exists());
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// raw_save_cancel
#[test]
fn cancelled_save_leaves_no_file() {
    let (dir, _, _, pending) = setup("shot.dng");
    let before = listing(&dir);
    assert_eq!(
        save(
            &pending,
            pending.seed.clone(),
            options(dir.join("out.dng"), PreviewSize::Medium, true, false),
            true
        ),
        Ok(DngOutcome::Cancelled)
    );
    assert_eq!(listing(&dir), before);
}
