//! One Move tool gesture on a 16507×16196 document whose top layer covers the
//! canvas (#1148): the old whole-content press, the view press at fit and at
//! 100%, the deferred commit, its worker composite and its GUI landing, each
//! timed.

use super::super::super::document_session::DocumentSession;
use super::super::super::edit::Edit;
use super::super::super::state::PictureViewRust;
use super::super::translate_move_group;
use super::preview::backdrop_patch;
use super::view_preview::view_preview;
use crate::cxxqt_object::helpers_composite::{composite_regions, layer_image};
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};
use std::time::Instant;

fn ms(label: &str, t: Instant) {
    println!("{label}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
}

/// `PICTURA_PROFILE_DIVISOR` shrinks both sides for a machine without the
/// memory the full size takes; the costs measured here scale with area.
fn size() -> (u32, u32) {
    if let Some(side) = std::env::var("PICTURA_PROFILE_SIDE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
    {
        return (side.max(1), side.max(1));
    }
    let divisor = std::env::var("PICTURA_PROFILE_DIVISOR")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    (16507 / divisor, 16196 / divisor)
}

fn view(width: u32, height: u32) -> PictureViewRust {
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", width, height, (120, 170, 220)),
        pixel_layer("over", width, height, (220, 90, 40)),
    ]);
    doc.composite
        .replace(pictura_render::composite_rgba(&doc))
        .expect("valid composite");
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view.apply_edit("Open", Edit::Record);
    view
}

#[test]
#[ignore = "16507x16196 move gesture profile; run explicitly with --ignored --nocapture"]
fn move_gesture_profile() {
    let (width, height) = size();
    println!("document {width}x{height}");
    let mut view = view(width, height);
    let paths = vec!["1".to_string()];
    let doc = view.session.doc.as_ref().expect("document");

    let t = Instant::now();
    let rect = pictura_render::content_rect(doc, "1");
    ms("content rect", t);
    assert!(rect.is_some());

    let t = Instant::now();
    let base = backdrop_patch(doc, &paths, false);
    ms("whole-document backdrop", t);
    drop(base);

    let t = Instant::now();
    let image = layer_image(pictura_render::resolve_path(doc, "1").expect("layer"));
    ms("whole-layer image", t);
    drop(image);

    let fit = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    let t = Instant::now();
    let preview = view_preview(doc, &paths, 4, fit, false);
    ms("view press at fit (level 4)", t);
    assert!(preview.is_some());
    drop(preview);

    let screen = PsdRect {
        top: 4000,
        left: 4000,
        bottom: 4000 + 1440,
        right: 4000 + 2560,
    };
    let t = Instant::now();
    let preview = view_preview(doc, &paths, 0, screen, false);
    ms("view press at 100% (level 0)", t);
    assert!(preview.is_some());
    drop(preview);

    let doc = view.session.doc.as_mut().expect("document");
    let t = Instant::now();
    let dirty = translate_move_group(doc, &paths, 40, 25).expect("moved");
    ms("translate", t);

    let t = Instant::now();
    let applied = view.apply_edit_move("Move Layer", Edit::regions(vec![dirty], false));
    ms("commit edit (deferred)", t);
    println!("commit deferred: {}", view.move_composite_pending());
    drop(applied);

    if let Some(job) = view.pending_move_job.take() {
        let t = Instant::now();
        let regions = composite_regions(&job.doc, &job.rects, job.gpu_compute);
        ms("worker composite", t);
        let t = Instant::now();
        let landed = view.land_move_composite(job.generation, regions);
        ms("landing (GUI)", t);
        assert!(landed.is_some(), "the current job lands");
    } else {
        let t = Instant::now();
        view.settle_move_composite();
        ms("commit settle (synchronous)", t);
    }

    let t = Instant::now();
    view.fit_tiles();
    let (level, rect) = view.fit_view(700, 700);
    view.presented_level = level;
    view.presented_rect = rect;
    let keys = view.presented_tiles_to_build();
    ms("plain presented keys", t);
    let t = Instant::now();
    let built = view.run_tile_job(&keys);
    println!("plain built {built}/{} tiles at level {level}", keys.len());
    ms("plain lazy tile rebuild", t);
}

/// A paragraph type layer on a large document: the ink bounds, the press
/// images, the commit, and the lazy tile rebuild the Move leaves behind.
#[test]
#[ignore = "large-document type Move profile; run explicitly with --ignored --nocapture"]
fn type_move_gesture_profile() {
    use super::preview::moving_layer_image;
    use pictura_core::TypeSpec;

    let (width, height) = size();
    println!("type document {width}x{height}");
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        LayerStack::from_layers(vec![pixel_layer("under", width, height, (120, 170, 220))]);
    let spec = TypeSpec {
        box_bounds: Some([0.0, 0.0, f64::from(width) / 2.0, f64::from(height) / 2.0]),
        ..TypeSpec::new("Lorem Ipsum Pictura", "Liberation Sans", 20.0)
    };
    let path = pictura_render::add_type_layer(&mut doc, "0", &spec);
    assert!(!path.is_empty(), "a paragraph type layer");
    if std::env::var_os("PICTURA_PROFILE_TYPE_SOURCE").is_some() {
        if let Some(layer) = pictura_render::resolve_path_mut(&mut doc, &path) {
            layer.channels_mut().clear();
        }
        let t = Instant::now();
        let materialized = pictura_render::materialize_type_layer(&mut doc, &path);
        ms("type materialize (first press)", t);
        assert!(materialized);
    }
    let paths = vec![path.clone()];
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    let doc = view.session.doc.as_ref().expect("document");

    let t = Instant::now();
    let ink = pictura_render::damage::type_ink_influence(doc, &path);
    ms("type ink", t);
    assert!(ink.is_some());

    let t = Instant::now();
    let base = backdrop_patch(doc, &paths, false);
    ms("type backdrop patch", t);
    drop(base);

    let t = Instant::now();
    let image = moving_layer_image(doc, &path, ink.expect("ink"), 0, false);
    ms("type moving image", t);
    drop(image);

    let t = Instant::now();
    let _ = pictura_render::layer_content_bounds(doc, &path);
    ms("type content bounds", t);

    let doc = view.session.doc.as_mut().expect("document");
    let t = Instant::now();
    let dirty = translate_move_group(doc, &paths, 40, 25).expect("moved");
    ms("type translate", t);

    let t = Instant::now();
    let _applied = view.apply_edit_move("Move Layer", Edit::regions(vec![dirty], false));
    ms("type commit edit", t);
    println!("type commit pending: {}", view.move_composite_pending());

    let t = Instant::now();
    view.fit_tiles();
    let (level, rect) = view.fit_view(700, 700);
    view.presented_level = level;
    view.presented_rect = rect;
    let keys = view.presented_tiles_to_build();
    ms("type presented keys", t);

    let t = Instant::now();
    let built = view.run_tile_job(&keys);
    println!("type built {built}/{} tiles at level {level}", keys.len());
    ms("type lazy tile rebuild", t);
}

/// The user's recovered 20000² type document, moved by the Move tool: the
/// actual layer state, its reach, the press images, the commit, and the lazy
/// tile rebuild.
#[test]
#[ignore = "recovered-document Move profile; run explicitly with --ignored --nocapture"]
fn recovered_type_move_profile() {
    use super::preview::moving_layer_image;
    use crate::layer_target::LayerTarget;

    let Some(file) = std::env::var_os("PICTURA_RECOVERED_PSD").map(std::path::PathBuf::from) else {
        println!("skip: set PICTURA_RECOVERED_PSD");
        return;
    };
    let bytes = std::fs::read(&file).expect("read the recovered psd");
    let t = Instant::now();
    let doc = pictura_codec::read_psd(&bytes).expect("decode the recovered psd");
    ms("recovered read", t);
    let type_path = pictura_render::flatten_rows(&doc)
        .into_iter()
        .find(|(path, _)| {
            pictura_render::resolve_path(&doc, path).is_some_and(|layer| layer.is_type())
        })
        .map(|(path, _)| path.to_string())
        .expect("a type layer");
    let layer = pictura_render::resolve_path(&doc, &type_path).expect("type layer");
    println!(
        "type path {type_path} rect {:?} channels {} clipping {} knockout {:?} vector {} type_tool {}",
        layer.rect,
        layer.channels().len(),
        layer.clipping,
        layer.knockout,
        layer.vector_mask.is_some(),
        layer.type_tool().is_some()
    );
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view.layer_target = LayerTarget::only(&type_path);
    let paths = vec![type_path.clone()];
    let doc = view.session.doc.as_ref().expect("document");

    let t = Instant::now();
    let ink = pictura_render::damage::type_ink_influence(doc, &type_path);
    ms("recovered type ink", t);
    println!("recovered ink {ink:?}");
    let t = Instant::now();
    let conservative = pictura_render::damage::content_influence(doc, &type_path);
    ms("recovered conservative reach", t);
    println!("recovered conservative {conservative:?}");

    let t = Instant::now();
    let base = backdrop_patch(doc, &paths, false);
    ms("recovered backdrop patch", t);
    drop(base);
    let t = Instant::now();
    let image = ink.and_then(|ink| moving_layer_image(doc, &type_path, ink, 0, false));
    ms("recovered moving image", t);
    drop(image);

    let doc = view.session.doc.as_mut().expect("document");
    let t = Instant::now();
    let dirty = translate_move_group(doc, &paths, 40, 25).expect("moved");
    ms("recovered translate", t);
    println!("recovered dirty {dirty:?}");

    let low_room = std::env::var_os("PICTURA_PROFILE_LOW_ROOM").is_some();
    if low_room {
        pictura_core::memory::set_host_for_test(Some(pictura_core::memory::HostReading {
            total: Some(2 * 1024 * 1024 * 1024),
            available: Some(256 * 1024 * 1024),
            rss: Some(128 * 1024 * 1024),
            ..Default::default()
        }));
    }
    crate::HEAP.reset();
    let t = Instant::now();
    let _applied = view.apply_edit_move("Move Layer", Edit::regions(vec![dirty], false));
    ms("recovered commit edit", t);
    println!(
        "recovered commit heap peak {:.1} MiB, pending {}",
        crate::HEAP.peak() as f64 / (1024.0 * 1024.0),
        view.move_composite_pending()
    );
    if low_room {
        pictura_core::memory::set_host_for_test(None);
    }

    let t = Instant::now();
    view.fit_tiles();
    let (level, rect) = view.fit_view(700, 700);
    view.presented_level = level;
    view.presented_rect = rect;
    let keys = view.presented_tiles_to_build();
    ms("recovered presented keys", t);
    let t = Instant::now();
    let built = view.run_tile_job(&keys);
    println!(
        "recovered built {built}/{} tiles at level {level}",
        keys.len()
    );
    ms("recovered lazy tile rebuild", t);
}
