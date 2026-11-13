use super::super::super::document_session::{TransformMode, TransformSession};
use super::super::geometry::{
    apply_session_transform, pivot_point, rect_center, set_pivot, source_corners, MapParams,
};
use super::super::group::{commit_cost, move_commit_cost, warp_strip_bytes};
use super::*;
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{BitDepth, Channel, ColorMode, Document, LockFlags, PsdRect};
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
        members: vec![("0".to_string(), rect)],
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
        untrimmed: Vec::new(),
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
    } = build_transform_preview(
        &doc,
        &path,
        std::slice::from_ref(&path),
        PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        false,
        false,
    )
    .expect("shape preview");
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
fn a_transform_commit_spills_its_box_and_the_composite_around_it() {
    let orig = PsdRect {
        top: 0,
        left: 0,
        bottom: 10,
        right: 10,
    };
    // Scaled to twice its size from its top-left corner: a 20×20 box.
    let quad = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
    assert_eq!(commit_cost(&quad, orig).spillable, 4 * 400 + 4 * 400);
    // A fractional corner rounds the box outward.
    let quad = [(0.5, 0.5), (10.2, 0.5), (10.2, 9.7), (0.5, 9.7)];
    assert_eq!(commit_cost(&quad, orig).spillable, 4 * 110 + 4 * 110);
}

#[test]
fn a_whole_pixel_move_spills_only_the_composite() {
    let orig = PsdRect {
        top: 0,
        left: 0,
        bottom: 10,
        right: 10,
    };
    let shifted = |dx: f64| [(dx, 0.0), (10.0 + dx, 0.0), (10.0 + dx, 10.0), (dx, 10.0)];
    assert_eq!(commit_cost(&shifted(5.0), orig).spillable, 4 * 150);
    assert_eq!(
        commit_cost(&shifted(5.5), orig).spillable,
        4 * 110 + 4 * 160
    );
}

#[test]
fn a_move_about_an_off_centre_pivot_keeps_the_planes() {
    let rgba: Vec<u8> = (0..40 * 30 * 4).map(|i| (i * 31 % 253) as u8).collect();
    let original = Document::from_rgba("move", 40, 30, &rgba);
    let mut doc = original.clone();
    let orig_rect = pictura_render::resolve_path(&doc, "0").expect("layer").rect;
    let params = MapParams {
        scale_x: 1.0,
        scale_y: 1.0,
        angle: 0.0,
        dx: 7.0,
        dy: -3.0,
        pivot: (0.1, 29.3),
        skew_x: 0.0,
        skew_y: 0.0,
    };
    let quad = params.map_quad(orig_rect);
    assert!(apply_session_transform(
        &mut doc,
        "0",
        false,
        quad,
        params,
        orig_rect,
        Interpolation::Bilinear,
    ));
    let before = pictura_render::resolve_path(&original, "0").expect("layer");
    let after = pictura_render::resolve_path(&doc, "0").expect("layer");
    assert_eq!(after.rect.left, before.rect.left + 7);
    assert_eq!(after.rect.top, before.rect.top - 3);
    for (a, b) in before.channels.iter().zip(&after.channels) {
        assert!(b.data.shares(&a.data), "channel {} was resampled", a.id);
    }
}

/// A 17000² full-canvas layer scaled 1.5 about its centre.
fn huge_resize() -> super::super::group::CommitCost {
    let side = 17000;
    let orig = PsdRect {
        top: 0,
        left: 0,
        bottom: side,
        right: side,
    };
    let (c, half) = (f64::from(side) / 2.0, f64::from(side) * 0.75);
    let quad = [
        (c - half, c - half),
        (c + half, c - half),
        (c + half, c + half),
        (c - half, c + half),
    ];
    commit_cost(&quad, orig)
}

const MIB: i64 = 1024 * 1024;

/// Gate `cost` with `available` bytes of host memory (the room is that less
/// 256 MiB of headroom) and `scratch` bytes free on the scratch disks.
fn gate(cost: &super::super::group::CommitCost, available: i64, scratch: i64) -> bool {
    use crate::cxxqt_object::helpers::memory_gate;
    memory_gate::set_available_memory_for_test(available);
    memory_gate::set_scratch_available_for_test(scratch);
    memory_gate::clear();
    let admitted = memory_gate::admit_spilling(cost.working, cost.spillable);
    memory_gate::set_available_memory_for_test(-1);
    memory_gate::set_scratch_available_for_test(-1);
    admitted
}

#[test]
fn a_large_resize_is_admitted_when_scratch_holds_it() {
    use crate::cxxqt_object::helpers::memory_gate;
    let cost = huge_resize();
    assert!(
        cost.spillable > 4 * 1024 * MIB as u64,
        "the output is gigabytes"
    );
    // 2 GiB available leaves 1.75 GiB of room: enough for the working set even
    // with one window per thread on a many-core runner.
    assert!(gate(&cost, 2048 * MIB, 64 * 1024 * MIB));
    assert!(!memory_gate::edit_refused_for_memory());
}

#[test]
fn full_scratch_disks_refuse_the_transform() {
    use crate::cxxqt_object::helpers::memory_gate;
    assert!(!gate(&huge_resize(), 2048 * MIB, 0));
    assert!(memory_gate::edit_refused_for_scratch_full());
    assert!(memory_gate::edit_refused_for_memory());
}

// [ocw_gate_spills]
#[test]
fn a_large_warp_is_admitted_when_scratch_holds_it() {
    use crate::cxxqt_object::helpers::memory_gate;
    let side = 17_000i32;
    let orig = PsdRect {
        top: 0,
        left: 0,
        bottom: side,
        right: side,
    };
    let s = f64::from(side);
    // An arc's preview quad: the top rises and spreads past the box.
    let quad = [(-900.0, -1500.0), (s + 900.0, -1500.0), (s, s), (0.0, s)];
    let mut cost = commit_cost(&quad, orig);
    let plain = cost.working;
    cost.working += warp_strip_bytes(&quad);
    assert_eq!(cost.working - plain, 5 * 4 * 256 * (17_000 + 1800));
    // 2 GiB available leaves 1.75 GiB of room, far under the dense warp's
    // planes, yet enough for the strips and windows.
    assert!(gate(&cost, 2048 * MIB, 64 * 1024 * MIB));
    assert!(!memory_gate::edit_refused_for_memory());
}

// [oct_move_gate_spills]
#[test]
fn a_large_layer_move_spills_its_composite() {
    use crate::cxxqt_object::helpers::memory_gate;
    let dirty = PsdRect {
        top: 0,
        left: 0,
        bottom: 17_005,
        right: 17_005,
    };
    // 2 GiB available leaves 1.75 GiB of room, far under 12 bytes a pixel.
    let room = (2048 - 256) * MIB as u64;
    assert!(12 * memory_gate::area(dirty) > room);
    assert!(gate(&move_commit_cost(dirty), 2048 * MIB, 64 * 1024 * MIB));
    assert!(!memory_gate::edit_refused_for_memory());
}

#[test]
fn a_working_set_beyond_ram_is_a_memory_refusal() {
    use crate::cxxqt_object::helpers::memory_gate;
    // 300 MiB available leaves 44 MiB of room, under the working set.
    let cost = huge_resize();
    assert!(cost.working > 44 * MIB as u64);
    assert!(!gate(&cost, 300 * MIB, 64 * 1024 * MIB));
    assert!(memory_gate::edit_refused_for_memory());
    assert!(!memory_gate::edit_refused_for_scratch_full());
}
