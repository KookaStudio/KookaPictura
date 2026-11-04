//! One Move tool gesture on a 16507×16196 document whose top layer covers the
//! canvas (#1148): the old whole-content press, the view press at fit and at
//! 100%, and the commit, each timed.

use super::super::super::document_session::DocumentSession;
use super::super::super::edit::Edit;
use super::super::super::state::PictureViewRust;
use super::super::translate_move_group;
use super::preview::backdrop_patch;
use super::view_preview::view_preview;
use crate::cxxqt_object::helpers_composite::layer_image;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};
use std::time::Instant;

fn ms(label: &str, t: Instant) {
    println!("{label}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
}

/// `PICTURA_PROFILE_DIVISOR` shrinks both sides for a machine without the
/// memory the full size takes; the costs measured here scale with area.
fn size() -> (u32, u32) {
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
    doc.composite = pictura_render::composite_rgba(&doc);
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
    ms("commit edit", t);
    println!("commit deferred: {}", view.move_composite_pending());
    drop(applied);

    let t = Instant::now();
    view.settle_move_composite();
    ms("commit settle (worker-equivalent)", t);
}
