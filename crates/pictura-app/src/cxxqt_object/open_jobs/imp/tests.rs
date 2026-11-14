//! Unit coverage for the open worker's decode/apply split (#729).

use super::*;
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer};
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

/// A 1x1 32-bit PSD whose half-opaque layer holds 2.0 and whose merged
/// composite holds the 1.0 result; opening it must keep the merged store (the
/// display recomposite clamps the layer to 1.0 and halves it).
fn hdr_layer_psd() -> (std::path::PathBuf, Vec<u8>) {
    use pictura_core::{
        BlendMode, Channel, Layer, LayerStack, PsdRect, Samples, SourceChannels, SourcePlanes,
    };
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 1,
        right: 1,
    };
    let channel = |id: i16, value: u8| Channel {
        id,
        data: vec![value].into(),
    };
    let f32_plane = |value: f32| Samples::F32(vec![value].into());
    let layer = Layer {
        name: "HDR".to_string(),
        rect,
        opacity: 128,
        blend: BlendMode::Normal,
        channels: vec![
            channel(0, 255),
            channel(1, 255),
            channel(2, 255),
            channel(-1, 255),
        ],
        source_channels: Some(SourceChannels::new(
            BitDepth::ThirtyTwo,
            rect,
            vec![
                (0, f32_plane(2.0)),
                (1, f32_plane(2.0)),
                (2, f32_plane(2.0)),
                (-1, f32_plane(1.0)),
            ],
        )),
        ..Default::default()
    };
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.source_depth = Some(BitDepth::ThirtyTwo);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::ThirtyTwo,
        width: 1,
        height: 1,
        samples: Samples::F32(vec![1.0, 1.0, 1.0].into()),
    });
    doc.composite = pictura_core::PixelBuffer {
        width: 1,
        height: 1,
        channels: 3,
        data: vec![255, 255, 255].into(),
    };
    doc.layers = LayerStack::from_layers(vec![layer]);
    let bytes = pictura_codec::write_psd(&doc).expect("write 32-bit fixture");
    let path = temp_path("psd");
    std::fs::write(&path, &bytes).expect("write fixture");
    (path, bytes)
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
    assert!(rust.session.doc.is_some());
    assert_eq!(rust.path.as_deref(), Some(path.to_str().unwrap()));
    assert!(!rust.session.dirty);
    assert_eq!(rust.source_format, "psd");
    std::fs::remove_file(path).ok();
}

#[test]
fn opened_hdr_conversion_uses_the_retained_merged_store() {
    let (path, _) = hdr_layer_psd();
    let mut opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    assert_eq!(
        opened.doc.composite.data,
        vec![255, 255, 255],
        "the merged composite stays the store's narrowing"
    );

    pictura_render::convert_depth_exposure_gamma(
        &mut opened.doc,
        BitDepth::Eight,
        pictura_render::ExposureGamma {
            exposure_ev: -1.0,
            gamma: 1.0,
        },
    )
    .expect("HDR conversion");

    // 1.0 with -1 EV is 0.5. The clamped layer recomposite would give 0.25.
    assert_eq!(opened.doc.composite.data, vec![128, 128, 128]);
    assert_eq!(opened.doc.layers.len(), 1, "the merged tone map flattens");
    assert_eq!(opened.doc.layers[0].name, "Background");
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

    let expected = path.to_str().unwrap().to_string();
    let opened = decode_image(path.to_str().unwrap(), false, &|_, _| {}).expect("decode");
    assert_eq!(opened.doc.width, 2);
    assert_eq!(opened.doc.height, 1);
    assert!(
        matches!(&opened.path, OpenedPath::Set(Some(p)) if p == &expected),
        "a raster open records its path"
    );
    assert_eq!(opened.source_format, "webp");
    assert_eq!(opened.target.single(), Some("0"));

    let mut rust = PictureViewRust::default();
    apply_opened(&mut rust, opened);
    assert!(rust.session.doc.is_some());
    assert_eq!(rust.path.as_deref(), Some(expected.as_str()));
    assert!(!rust.session.dirty);
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
    let mut rust = PictureViewRust::default();
    let request = rust.begin_open_request();
    rust.open_cancelled = true;
    assert!(!apply_if_current(&mut rust, request, Some(opened)));
    assert!(rust.session.doc.is_none());

    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    rust.open_cancelled = false;
    assert!(apply_if_current(&mut rust, request, Some(opened)));
    assert!(rust.session.doc.is_some());
    std::fs::remove_file(path).ok();
}

#[test]
fn a_stale_async_install_is_discarded() {
    let (path, _) = white_psd();
    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    let mut rust = PictureViewRust::default();
    let stale = rust.begin_open_request();
    // A newer request supersedes the first one.
    let current = rust.begin_open_request();
    assert_ne!(stale, current);

    assert!(
        !apply_if_current(&mut rust, stale, Some(opened)),
        "the late install is dropped"
    );
    assert!(rust.session.doc.is_none(), "the document is unchanged");
    assert_eq!(rust.session.history.count(), 0);

    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("decode");
    assert!(
        apply_if_current(&mut rust, current, Some(opened)),
        "the current install lands"
    );
    assert!(rust.session.doc.is_some());
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

fn profile_jpeg(icc: &[u8]) -> Vec<u8> {
    let jpeg = [0xFF, 0xD8, 0xFF, 0xD9];
    pictura_codec::embed_raster_metadata(
        &jpeg,
        "jpg",
        &pictura_codec::RasterMetadata {
            icc: Some(icc.to_vec()),
            ..pictura_codec::RasterMetadata::default()
        },
    )
    .expect("embed ICC")
}

fn profile_rgba_document() -> Document {
    Document::from_rgba("profile", 1, 1, &[200, 50, 100, 255])
}

#[test]
fn raster_import_policy_preserve() {
    let icc = pictura_codec::Profile::adobe_rgb().to_icc();
    let bytes = profile_jpeg(&icc);
    let mut doc = profile_rgba_document();
    let before = doc.composite.data.to_vec();
    super::apply_raster_profile(
        &mut doc,
        &bytes,
        Policy::Preserve,
        pictura_codec::ConversionOptions::default(),
    );
    assert_eq!(doc.document_icc.as_deref(), Some(icc.as_slice()));
    assert!(doc.source_icc.is_none());
    assert_eq!(doc.composite.data.to_vec(), before);
}

#[test]
fn raster_import_policy_convert() {
    let icc = pictura_codec::Profile::adobe_rgb().to_icc();
    let bytes = profile_jpeg(&icc);
    let mut doc = profile_rgba_document();
    let before = doc.composite.data.to_vec();
    super::apply_raster_profile(
        &mut doc,
        &bytes,
        Policy::Convert,
        pictura_codec::ConversionOptions::default(),
    );
    assert_eq!(doc.source_icc.as_deref(), Some(icc.as_slice()));
    assert!(doc.document_icc.is_none());
    assert_ne!(doc.composite.data.to_vec(), before);
}

#[test]
fn raster_import_policy_off() {
    let icc = pictura_codec::Profile::adobe_rgb().to_icc();
    let bytes = profile_jpeg(&icc);
    let mut doc = profile_rgba_document();
    let before = doc.composite.data.to_vec();
    super::apply_raster_profile(
        &mut doc,
        &bytes,
        Policy::Off,
        pictura_codec::ConversionOptions::default(),
    );
    assert!(doc.document_icc.is_none());
    assert!(doc.source_icc.is_none());
    assert_eq!(doc.composite.data.to_vec(), before);
}

#[test]
fn raster_import_policy_malformed_profile() {
    let mut jpeg = vec![0xFF, 0xD8];
    jpeg.extend_from_slice(&[0xFF, 0xE2, 0x00, 0x18]);
    jpeg.extend_from_slice(b"ICC_PROFILE\0");
    jpeg.extend_from_slice(&[1, 1]);
    jpeg.extend_from_slice(b"not an ICC profile");
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    let mut doc = profile_rgba_document();
    super::apply_raster_profile(
        &mut doc,
        &jpeg,
        Policy::Convert,
        pictura_codec::ConversionOptions::default(),
    );
    assert!(doc.document_icc.is_none());
    assert!(doc.source_icc.is_none());
}

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// Pin this thread's host reading to `available` bytes and the scratch disks
/// to `scratch` bytes, returning the governor's room.
fn pin(available: Option<u64>, scratch: i64) -> Option<u64> {
    pictura_core::memory::set_host_for_test(Some(pictura_core::memory::HostReading {
        available,
        ..Default::default()
    }));
    super::super::super::helpers::memory_gate::set_scratch_available_for_test(scratch);
    pictura_core::memory::governor().room()
}

fn unpin() {
    pictura_core::memory::set_host_for_test(None);
    super::super::super::helpers::memory_gate::set_scratch_available_for_test(-1);
}

// [oo_budget]
#[test]
fn open_budget_adds_scratch_to_the_tile_allowance() {
    let default = pictura_codec::ImageBudget::default();
    for scratch in [10 * GIB, 0] {
        let room = pin(Some(3 * GIB + 256 * MIB), scratch as i64).expect("a room");
        let (budget, bounded) = open_budget();
        assert!(bounded);
        let contiguous = room / 3 * 2;
        assert_eq!(
            budget.max_document_bytes,
            default.max_document_bytes.min(contiguous)
        );
        assert_eq!(
            budget.max_tiled_bytes,
            contiguous + scratch,
            "scratch {scratch}"
        );
    }
    assert_eq!(pin(None, 10 * GIB as i64), None, "no reading, no room");
    let (budget, bounded) = open_budget();
    assert!(!bounded);
    assert_eq!(budget.max_tiled_bytes, budget.max_document_bytes);
    unpin();
}

// [oo_admits]
#[test]
fn an_open_larger_than_the_room_is_admitted_with_scratch() {
    // 1024² of noise in four channels: 4 MiB of tiles, while a channel's
    // strip is about a quarter of a MiB.
    let side = 1024usize;
    let rgba: Vec<u8> = (0..side * side * 4)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).rotate_right(13) as u8)
        .collect();
    let doc = Document::from_rgba("Noise", side as u32, side as u32, &rgba);
    let path = temp_path("psd");
    std::fs::write(&path, pictura_codec::write_psd(&doc).expect("write")).expect("fixture");
    let path_str = path.to_str().unwrap();

    // Find an available amount whose room is 4.5 MiB: two thirds of it
    // (3 MiB) hold the strips, the merged transparency plane and the RLE row
    // table, but not the 4 MiB of tiles.
    let mut available = 4 * MIB;
    while pin(Some(available), 0).unwrap_or(0) < 9 * MIB / 2 {
        available += MIB / 4;
    }
    let tiles = 4 * side as u64 * side as u64;
    assert!(open_budget().0.max_tiled_bytes < tiles);

    pin(Some(available), (64 * GIB) as i64);
    let opened = decode_psd(path_str, Policy::Preserve, false, &|_, _| {});
    assert!(opened.is_some(), "scratch holds the tiles");
    assert!(!open_refused_for_memory());
    assert_eq!(
        opened.unwrap().doc.layers[0].channels,
        doc.layers[0].channels
    );

    pin(Some(available), 0);
    assert!(decode_psd(path_str, Policy::Preserve, false, &|_, _| {}).is_none());
    assert!(open_refused_for_memory(), "the refusal is for memory");
    unpin();
    std::fs::remove_file(path).ok();
}

// ---- Lazy, file-backed open (#1218) ----

const LW: u32 = 600;
const LH: u32 = 520;
const GRAY: u8 = 128;

fn noise(seed: u32, n: usize) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_add(seed).wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect()
}

/// An opaque noise Background under a partly transparent noise layer, saved
/// with a uniform gray merged image: the file's picture and the layers' tell
/// apart at every pixel.
fn lazy_psd() -> std::path::PathBuf {
    let mut rgba = noise(1, (LW * LH * 4) as usize);
    rgba.as_chunks_mut::<4>()
        .0
        .iter_mut()
        .for_each(|px| px[3] = 255);
    let mut doc = Document::from_rgba("Background", LW, LH, &rgba);
    let bottom = doc.layers.get_mut(0).expect("a layer");
    bottom.channels.retain(|c| c.id != -1);
    bottom.background = true;
    let top = Document::from_rgba("Top", 300, 200, &noise(9, 300 * 200 * 4));
    let mut layer = top.layers.get(0).expect("a layer").clone();
    layer.rect.left += 150;
    layer.rect.right += 150;
    layer.rect.top += 100;
    layer.rect.bottom += 100;
    doc.layers.push(layer).expect("push");
    let plane = (LW * LH) as usize;
    let mut merged = vec![GRAY; plane * 4];
    merged[plane * 3..].fill(255);
    doc.composite = PixelBuffer {
        width: LW,
        height: LH,
        channels: 4,
        data: merged.into(),
    };
    let path = temp_path("psd");
    std::fs::write(&path, pictura_codec::write_psd(&doc).expect("write")).expect("fixture");
    path
}

fn lazy_view(path: &std::path::Path) -> PictureViewRust {
    let opened =
        decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {}).expect("open");
    assert!(
        opened.preview.is_some(),
        "the fixture offers its merged image"
    );
    let mut view = PictureViewRust {
        gpu_compute: false,
        ..Default::default()
    };
    apply_opened(&mut view, opened);
    view
}

fn whole_rect(w: u32, h: u32) -> pictura_core::PsdRect {
    pictura_core::PsdRect {
        top: 0,
        left: 0,
        bottom: h as i32,
        right: w as i32,
    }
}

/// Whether every layer channel tile is still away in the file.
fn layers_untouched(view: &PictureViewRust) -> bool {
    let doc = view.session.doc.as_ref().expect("document");
    doc.layers.iter().flat_map(|l| l.channels.iter()).all(|c| {
        (0..c.data.tile_count())
            .all(|i| c.data.tile_residency(i) == Some(pictura_core::Residency::Backed))
    })
}

/// The tiles of `level` against a whole rebuild from the layers.
fn assert_tiles_are_the_layers(view: &PictureViewRust, level: i32) {
    let doc = view.session.doc.as_ref().expect("document");
    let source = super::super::super::tiles::display_region(doc, whole_rect(LW, LH), false);
    let planes = super::super::super::helpers_composite::planes_of(&source).expect("RGBA");
    let pyramid = pictura_render::ViewPyramid::rebuild(planes);
    let (w, h) = view.tiles.cache.borrow().level_size(level as usize);
    let (crop, missing) = view.tile_crop(
        pictura_render::TileSource::Composite,
        level,
        whole_rect(w, h),
    );
    assert!(missing.is_empty());
    assert_eq!(
        crop.data(),
        pyramid
            .crop(planes, level as usize, whole_rect(w, h))
            .data()
    );
}

// [sb_first_paint]
#[test]
fn first_paint_shows_the_files_merged_image() {
    let path = lazy_psd();
    let view = lazy_view(&path);
    let level = view.display_level_count() - 1;
    let (w, h) = view.tiles.cache.borrow().level_size(level as usize);
    let (crop, missing) = view.tile_crop(
        pictura_render::TileSource::Composite,
        level,
        whole_rect(w, h),
    );
    assert!(missing.is_empty(), "a small document builds at once");
    assert!(crop
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .all(|px| *px == [GRAY, GRAY, GRAY, 255]));
    assert!(layers_untouched(&view), "no layer tile was decoded");
    std::fs::remove_file(path).ok();
}

// [sb_first_edit]
#[test]
fn an_edit_replaces_the_merged_preview() {
    let path = lazy_psd();
    let mut view = lazy_view(&path);
    let level = view.display_level_count() - 1;
    let (w, h) = view.tiles.cache.borrow().level_size(level as usize);
    drop(view.tile_crop(
        pictura_render::TileSource::Composite,
        level,
        whole_rect(w, h),
    ));
    {
        let doc = view.session.doc.as_mut().expect("document");
        let top = doc.layers.get_mut(1).expect("the top layer");
        top.channels[0]
            .data
            .write_rect_from(0..50, 0..40, 300, &[250u8; 50 * 40]);
    }
    view.refresh_display_rect(pictura_core::PsdRect {
        top: 100,
        left: 150,
        bottom: 140,
        right: 200,
    });
    assert!(
        view.tiles.preview.borrow().is_none(),
        "the preview is dropped"
    );
    for level in 0..view.display_level_count() {
        assert_tiles_are_the_layers(&view, level);
    }
    std::fs::remove_file(path).ok();
}

// [sb_picture]
#[test]
fn picture_reads_layers_while_the_preview_shows() {
    let path = lazy_psd();
    let view = lazy_view(&path);
    // Level-0 tiles from the preview, which the picture must not reuse.
    drop(view.tile_crop(pictura_render::TileSource::Composite, 0, whole_rect(LW, LH)));
    let doc = view.session.doc.as_ref().expect("document");
    let picture = super::super::super::picture::picture(doc, &view);
    let layers = pictura_render::composite_region(doc, whole_rect(LW, LH));
    assert!(view.tiles.preview.borrow().is_some());
    assert!(*picture == layers, "the picture is the layers'");
    assert!(layers.data.as_slice()[..LW as usize]
        .iter()
        .any(|&v| v != GRAY));
    std::fs::remove_file(path).ok();
}

/// Save as the save job does: the save view, encoded, landed atomically.
fn save_to(doc: &Document, path: &std::path::Path) -> Vec<u8> {
    let view = pictura_render::save_view(doc);
    let bytes = pictura_codec::write_psd(&view).expect("encode");
    super::super::super::helpers::safe_io::write_atomically(path.to_str().unwrap(), &bytes, false)
        .expect("save");
    bytes
}

// [sb_save_over]
#[test]
fn save_over_source_with_backed_tiles() {
    let path = lazy_psd();
    let bytes = std::fs::read(&path).unwrap();
    let mut lazy = decode_psd(path.to_str().unwrap(), Policy::Preserve, false, &|_, _| {})
        .expect("lazy open")
        .doc;
    let mut eager = pictura_codec::read_psd_within_options(
        &bytes,
        &open_options(Policy::Preserve),
        pictura_codec::ImageBudget::default(),
    )
    .expect("eager open");
    save_to(&lazy, &path);
    for doc in [&mut lazy, &mut eager] {
        let top = doc.layers.get_mut(1).expect("the top layer");
        top.channels[1]
            .data
            .write_rect_from(10..60, 5..45, 300, &[7u8; 50 * 40]);
    }
    let lazy_bytes = save_to(&lazy, &path);
    let eager_path = temp_path("psd");
    let eager_bytes = save_to(&eager, &eager_path);
    assert!(
        lazy_bytes == eager_bytes,
        "the edit saves as on an eager open"
    );
    std::fs::remove_file(path).ok();
    std::fs::remove_file(eager_path).ok();
}
