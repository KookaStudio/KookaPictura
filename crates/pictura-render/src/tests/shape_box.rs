//! A shape layer's content box (#1185): its rect is the canvas, but its fill
//! composite and its effects' source stop at its mask's box. The result must
//! equal the whole-rect composite.

use std::sync::atomic::Ordering;

use super::*;
use crate::layer_effects::WHOLE_FILL_RECT;
use crate::tests::layer_effects::{spec_block, ShadowSpec};
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};

const W: u32 = 300;
const H: u32 = 200;

/// A white document with an ellipse shape layer at (60, 40)-(160, 120), a drop
/// shadow on it when `shadow`, and a half-transparent multiply layer below.
fn shape_doc(shadow: bool) -> Document {
    let mut d = doc(
        W,
        H,
        vec![
            solid(
                "Background",
                full(W, H),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "Half",
                rect(0, 0, 120, 200),
                (0, 0, 200),
                128,
                BlendMode::Multiply,
                255,
            ),
        ],
    );
    let ellipse = outline_in_box(
        ShapeOptions::new(ShapeKind::Ellipse, 0.0, 3),
        (60.0, 40.0, 100.0, 80.0),
    )
    .unwrap();
    let path = crate::add_shape_layer(&mut d, "", [255, 0, 0, 255], "Ellipse", &ellipse, None);
    if shadow {
        let layer = crate::resolve_path_mut(&mut d, &path).unwrap();
        layer.extra_blocks.push(spec_block(&ShadowSpec::default()));
    }
    d
}

/// The composite with the content box (`bounded`) or the whole rect.
fn composite_with(d: &Document, bounded: bool) -> PixelBuffer {
    WHOLE_FILL_RECT.store(!bounded, Ordering::Relaxed);
    crate::purge_source_cache();
    let out = composite_rgba(d);
    WHOLE_FILL_RECT.store(false, Ordering::Relaxed);
    out
}

#[test]
fn a_shape_composites_as_its_whole_rect_did() {
    let _guard = crate::composite_rows::serial_test_lock();
    let _cache = crate::layer_source::cache_test_guard();
    for shadow in [false, true] {
        let d = shape_doc(shadow);
        let whole = composite_with(&d, false);
        let bounded = composite_with(&d, true);
        assert_eq!(
            bounded.data.as_slice(),
            whole.data.as_slice(),
            "shadow {shadow}"
        );
        assert_eq!(rgb(&bounded, 110, 80), [255, 0, 0], "the fill is drawn");
    }
}

#[test]
fn a_shape_samples_its_mask_only_in_its_box() {
    let _guard = crate::composite_rows::serial_test_lock();
    let d = shape_doc(false);
    crate::composite_blend::reset_mask_samples();
    let _ = composite_with(&d, true);
    let sampled = crate::composite_blend::mask_samples();
    let box_area = 102 * 82;
    assert!(
        sampled <= box_area,
        "sampled {sampled} mask pixels, more than the {box_area} of the shape's box"
    );
}
