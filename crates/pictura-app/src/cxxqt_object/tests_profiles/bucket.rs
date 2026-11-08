//! A Paint Bucket click that fills an empty layer on a 16507×16196 document,
//! and its undo, timed stage by stage the way `fills::imp::apply` runs them
//! (#1149).

use super::super::document_session::DocumentSession;
use super::super::edit::Edit;
use super::super::helpers::paint_timing;
use super::{ms, pixel_layer, LayerTarget, PictureViewRust};
use crate::history::Restored;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PsdRect};
use pictura_paint::bucket::{self, BucketPaint, BucketSource};
use pictura_paint::{Alpha, PaintMode, Rgba};
use std::time::Instant;

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

#[test]
#[ignore = "16507x16196 Paint Bucket profile; run explicitly with --ignored --nocapture"]
fn bucket_fill_profile() {
    let (width, height) = size();
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer(
        "Background",
        width,
        height,
        (120, 170, 220),
    )]);
    assert!(pictura_render::add_layer(&mut doc, -1, "Layer") >= 0);
    doc.composite = pictura_render::composite_rgba(&doc);
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("1"),
        gpu_compute: false,
        ..Default::default()
    };
    view.reset_pyramid();
    view.apply_edit("Open", Edit::Record);

    let total = Instant::now();
    let t = Instant::now();
    let mut edited = view.session.doc.as_ref().expect("document").clone();
    ms("clone", t.elapsed());
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    let t = Instant::now();
    let source = BucketSource::layer(&edited, "1").expect("pixel layer");
    let seed = (width as i32 / 2, height as i32 / 2);
    let size = (width, height);
    let flood = bucket::flood(source, size, canvas, None, seed, 32, true).expect("a flood");
    ms("flood", t.elapsed());
    let t = Instant::now();
    let mask = bucket::antialias(flood, size, canvas, None);
    ms("antialias", t.elapsed());
    let t = Instant::now();
    let red = Rgba {
        r: 220,
        g: 30,
        b: 40,
        a: 255,
    };
    let rect = bucket::fill_tiled(
        &mut edited,
        "1",
        canvas,
        &mask,
        BucketPaint::Foreground(red),
        PaintMode::Normal,
        1.0,
        None,
        Alpha::Blend,
    )
    .expect("a change");
    ms("fill", t.elapsed());
    drop(mask);
    let t = Instant::now();
    view.session.doc = Some(edited);
    ms("adopt", t.elapsed());
    let t = Instant::now();
    paint_timing::start("refresh regions");
    view.refresh_regions(&[rect]);
    paint_timing::report();
    ms("refresh regions", t.elapsed());
    let t = Instant::now();
    let target = view.layer_target.clone();
    assert!(view.session.record(&target, "Paint Bucket"));
    ms("history capture", t.elapsed());
    ms("fill total", total.elapsed());

    let session = &mut view.session;
    let doc = session.doc.as_mut().expect("document");
    let t = Instant::now();
    let restored = session
        .history
        .undo_live(doc, &mut session.selection, &mut LayerTarget::default())
        .expect("a state to undo");
    ms(&format!("undo history ({restored:?})"), t.elapsed());
    let t = Instant::now();
    match restored {
        Restored::Region(rect) | Restored::Composite(rect) => {
            view.refresh_from_composite(rect);
        }
        Restored::Everywhere => view.reset_pyramid(),
    }
    ms("undo display", t.elapsed());
}
