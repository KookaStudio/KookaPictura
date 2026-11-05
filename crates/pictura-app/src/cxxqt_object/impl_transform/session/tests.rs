use super::super::super::document_session::{TransformMode, TransformSession};
use super::super::geometry::{pivot_point, rect_center, set_pivot, source_corners};
use super::*;
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{BitDepth, Channel, ColorMode, Document, PsdRect};
use pictura_render::Interpolation;

#[test]
fn the_selection_test_agrees_with_the_lift_coverage() {
    let mut rust = PictureViewRust::default();
    rust.session.doc = Some(Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight));
    let mut corner = vec![0u8; 64];
    corner[63] = 1;
    let cases = [
        None,
        Some((8, 8, vec![0u8; 64])),
        Some((8, 8, corner)),
        Some((8, 8, vec![255u8; 64])),
        Some((4, 4, vec![255u8; 16])),
    ];
    for (i, case) in cases.into_iter().enumerate() {
        rust.session.selection = case.map(|(width, height, data)| pictura_select::Selection {
            width,
            height,
            data: data.into(),
        });
        assert_eq!(
            selection_covers(&rust),
            lift_coverage(&rust).is_some(),
            "case {i}"
        );
    }
}

fn transform_session(rect: PsdRect) -> TransformSession {
    TransformSession {
        path: "0".to_string(),
        orig_rect: rect,
        scale_x: 1.0,
        scale_y: 1.0,
        angle: 0.0,
        dx: 0.0,
        dy: 0.0,
        handle: -1,
        press_x: 0.0,
        press_y: 0.0,
        start: [1.0, 1.0, 0.0, 0.0, 0.0],
        dragging: false,
        mode: TransformMode::Free,
        type_layer: false,
        quad: None,
        base_quad: None,
        free_perspective: false,
        free_distort: false,
        free_skew: false,
        perspective_side: None,
        start_quad: source_corners(rect),
        pivot: rect_center(rect),
        pivot_origin: rect_center(rect),
        start_pivot: rect_center(rect),
        show_pivot: true,
        skew_x: 0.0,
        skew_y: 0.0,
        interpolation: Interpolation::Bicubic,
        warp: None,
        warp_drag: None,
        warp_editing: false,
        warp_post: None,
        lifted: None,
        untrimmed: None,
        history_label: "Free Transform".to_string(),
    }
}

#[test]
fn repeatable_transform_records_the_source_pivot() {
    let rect = PsdRect {
        left: 0,
        top: 0,
        right: 10,
        bottom: 10,
    };
    let mut session = transform_session(rect);
    // A translated session takes a document pivot at the origin; the
    // stored source point is its inverse, and the recorded offset is in
    // the source frame, not the translated document frame.
    session.dx = 5.0;
    session.dy = 5.0;
    assert!(set_pivot(&mut session, 0.0, 0.0));
    assert_eq!(session.pivot, (-5.0, -5.0));
    assert_eq!(pivot_point(&session), (0.0, 0.0));
    let repeat = repeatable_transform(&session).expect("repeatable");
    assert_eq!(repeat.pivot_offset, (-10.0, -10.0));
}

#[test]
fn a_shape_preview_ignores_empty_channels() {
    let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
    let outline = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (4.0, 4.0, 24.0, 20.0),
    )
    .expect("outline");
    let path =
        pictura_render::add_shape_layer(&mut doc, "", [0, 0, 0, 255], "Shape", &outline, None);
    assert!(!path.is_empty());
    // A PSD shape layer may carry empty pixel channels: its visible fill
    // comes from the descriptor and vector mask, not the channels.
    let plane = (doc.width * doc.height) as usize;
    let layer = pictura_render::resolve_path_mut(&mut doc, &path).expect("shape");
    layer.channels = [0, 1, 2, -1]
        .into_iter()
        .map(|id| Channel {
            id,
            data: vec![0; plane].into(),
        })
        .collect();

    let TransformPreview {
        image: preview,
        x,
        y,
        ..
    } = build_transform_preview(&doc, &path, false, false).expect("shape preview");
    assert_eq!((x, y), (0, 0));
    let filled = (0..preview.height()).any(|py| {
        (0..preview.width()).any(|px| {
            let c = preview.pixel_color(px, py);
            c.alpha() > 200 && c.red() < 20 && c.green() < 20 && c.blue() < 20
        })
    });
    assert!(filled, "the shape's fill is in the preview layer");
}

#[test]
fn a_shape_layer_is_a_transform_target_unless_position_locked() {
    let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
    let outline = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (4.0, 4.0, 24.0, 20.0),
    )
    .expect("outline");
    let path =
        pictura_render::add_shape_layer(&mut doc, "", [0, 0, 0, 255], "Shape", &outline, None);
    assert!(can_free_transform(&doc, &path));
    pictura_render::resolve_path_mut(&mut doc, &path)
        .expect("shape")
        .lock = LockFlags::default().with(LockFlags::POSITION, true);
    assert!(!can_free_transform(&doc, &path));
}

#[test]
fn a_transform_commit_charges_its_box_and_the_composite_around_it() {
    let orig = PsdRect {
        top: 0,
        left: 0,
        bottom: 10,
        right: 10,
    };
    // Scaled to twice its size from its top-left corner: a 20×20 box.
    let quad = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    assert_eq!(transform_commit_bytes(&quad, orig), 4 * 400 + 12 * 400);
    // A fractional corner rounds the box outward.
    let quad = [(0.5, 0.5), (10.2, 0.5), (10.2, 9.7), (0.5, 9.7)];
    assert_eq!(transform_commit_bytes(&quad, orig), 4 * 110 + 12 * 110);
}
