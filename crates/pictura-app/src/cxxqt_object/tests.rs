use super::helpers::*;
use super::helpers_composite::*;
use super::impl_core::{finalize_import, format_for_path, raster_writer_for_suffix};
use super::impl_transform::{build_move_preview_base, duplicate_move_target};
use crate::history::{History, Snapshot};
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerBlock, LayerMask,
    LockFlags, PixelBuffer, PsdRect,
};
use pictura_select::Selection;

pub(super) fn pixel_layer(name: &str, w: u32, h: u32, rgb: (u8, u8, u8)) -> Layer {
    let n = (w * h) as usize;
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![rgb.0; n].into(),
            },
            Channel {
                id: 1,
                data: vec![rgb.1; n].into(),
            },
            Channel {
                id: 2,
                data: vec![rgb.2; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255; n].into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

#[test]
fn converts_planar_rgb_to_rgba() {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![10, 20, 30, 40, 50, 60].into();
    let image = document_to_image(&doc, false);
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 1);
    assert_eq!(image.pixel_color(0, 0).red(), 10);
    assert_eq!(image.pixel_color(0, 0).green(), 30);
    assert_eq!(image.pixel_color(0, 0).blue(), 50);
    assert_eq!(image.pixel_color(0, 0).alpha(), 255);
}

#[test]
fn clamp_region_matches_document_bounds() {
    let rect = |t, l, b, r| PsdRect {
        top: t,
        left: l,
        bottom: b,
        right: r,
    };
    assert_eq!(clamp_region(rect(0, 0, 4, 4), 4, 4), Some((0, 0, 4, 4)));
    // Partially outside clamps to the intersection.
    assert_eq!(clamp_region(rect(-2, -2, 3, 3), 4, 4), Some((0, 0, 3, 3)));
    // Entirely outside, and zero-area rects, are empty.
    assert_eq!(clamp_region(rect(10, 10, 12, 12), 4, 4), None);
    assert_eq!(clamp_region(rect(0, 0, 0, 4), 4, 4), None);
}

#[test]
fn layered_document_composites_with_source_alpha() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![Layer {
        name: "red".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![255].into(),
            },
            Channel {
                id: 1,
                data: vec![0].into(),
            },
            Channel {
                id: 2,
                data: vec![0].into(),
            },
            Channel {
                id: -1,
                data: vec![255].into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }];

    let image = document_to_image(&doc, false);
    assert_eq!(image.pixel_color(0, 0).red(), 255);
    assert_eq!(image.pixel_color(0, 0).alpha(), 255);
    // Uncovered canvas stays transparent, not the embedded composite.
    assert_eq!(image.pixel_color(1, 1).alpha(), 0);
}

#[test]
fn invert_and_visibility_change_composite() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![Layer {
        name: "red".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![255].into(),
            },
            Channel {
                id: 1,
                data: vec![0].into(),
            },
            Channel {
                id: 2,
                data: vec![0].into(),
            },
            Channel {
                id: -1,
                data: vec![255].into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }];

    let before = document_to_image(&doc, false);
    assert_eq!(before.pixel_color(0, 0).red(), 255);

    doc.layers
        .push(adjustment_layer("invert", None).expect("known kind"));
    let after = document_to_image(&doc, false);
    assert_eq!(after.pixel_color(0, 0).red(), 0);
    assert_eq!(after.pixel_color(0, 0).green(), 255);
    assert_eq!(after.pixel_color(0, 0).blue(), 255);

    doc.layers[0].visible = false;
    let hidden = document_to_image(&doc, false);
    assert_eq!(hidden.pixel_color(0, 0).alpha(), 0);

    assert!(adjustment_layer("bogus", None).is_none());
}

#[test]
fn row_thumbnails_respect_contents_and_masks() {
    let mut layer = pixel_layer("red", 2, 2, (255, 0, 0));
    layer.rect = PsdRect {
        top: 2,
        left: 2,
        bottom: 4,
        right: 4,
    };

    let positioned = layer_thumbnail_positioned(&layer, 4, 4, 4).expect("entire-document square");
    assert_eq!((positioned.width(), positioned.height()), (4, 4));
    assert_eq!(positioned.pixel_color(2, 2).red(), 255);
    assert_eq!(positioned.pixel_color(2, 2).alpha(), 255);
    assert_eq!(
        positioned.pixel_color(0, 0).alpha(),
        0,
        "document is empty outside the layer"
    );

    let bounds = layer_thumbnail_image(&layer, 4).expect("layer-bounds thumbnail");
    assert_eq!(
        (bounds.width(), bounds.height()),
        (4, 4),
        "layer bounds fill the square"
    );
    assert_eq!(bounds.pixel_color(0, 0).red(), 255);

    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(vec![0, 128, 255, 64].into()),
        ..Default::default()
    };
    let mask_image = mask_thumbnail_image(&mask, 4).expect("mask thumbnail");
    assert_eq!((mask_image.width(), mask_image.height()), (4, 4));
    assert_eq!(mask_image.pixel_color(0, 0).red(), 0);
    assert_eq!(mask_image.pixel_color(3, 3).red(), 64);
}

#[test]
fn move_preview_base_equals_full_composite_with_layer_hidden() {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 64, 64, (30, 60, 90)),
        pixel_layer("top", 32, 32, (200, 100, 50)),
    ];
    let index = topmost_pixel_layer_index(&doc).expect("pixel layer");
    let rect = doc.layers[index].rect;
    assert_eq!(
        (rect.left, rect.top, rect.right, rect.bottom),
        (0, 0, 32, 32)
    );

    // The authoritative composite holds the layer visible, as M34 keeps it.
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);

    // (a) Full composite with the topmost layer hidden.
    doc.layers[index].visible = false;
    let hidden = document_to_image(&doc, false);
    // (b) With the layer still hidden, region-composite its rect.
    let (buffer, _) = pictura_render::composite_region_active(&doc, rect, false);
    doc.layers[index].visible = true;
    let (x0, y0, w, h) = clamp_region(rect, doc.width, doc.height).expect("in-bounds");
    assert_eq!((buffer.width, buffer.height), (w, h));

    // Patch that region into a planar clone of the composite: the bridge path.
    let mut base_buffer = doc.composite.clone();
    patch_buffer_region(&mut base_buffer, &buffer, x0, y0);
    let base = buffer_to_image(&base_buffer);

    assert_eq!(
        (base.width(), base.height()),
        (hidden.width(), hidden.height())
    );
    for y in 0..hidden.height() {
        for x in 0..hidden.width() {
            assert_eq!(
                base.pixel_color(x, y),
                hidden.pixel_color(x, y),
                "pixel ({x},{y}) differs"
            );
        }
    }
}

#[test]
fn move_preview_region_guards_empty_and_missing_composite() {
    let rect = |t, l, b, r| PsdRect {
        top: t,
        left: l,
        bottom: b,
        right: r,
    };
    // An in-bounds rect takes the region path.
    assert_eq!(
        move_preview_region(rect(0, 0, 64, 64), 64, 64, true),
        Some((0, 0, 64, 64))
    );
    // A rect larger than the old 1 MP budget no longer falls back.
    assert_eq!(
        move_preview_region(rect(0, 0, 2000, 2000), 4000, 4000, true),
        Some((0, 0, 2000, 2000))
    );
    // A missing/mismatched composite falls back.
    assert_eq!(move_preview_region(rect(0, 0, 64, 64), 64, 64, false), None);
    // An empty rect (here, zero area after clamping) falls back.
    assert_eq!(move_preview_region(rect(0, 0, 0, 64), 64, 64, true), None);
}

#[test]
fn layer_visibility_region_bounds_raster_and_bounded_adjustments() {
    // A raster layer is bounded by its own rect (clamped by refresh_region).
    let raster = pixel_layer("raster", 8, 4, (10, 20, 30));
    assert_eq!(layer_visibility_region(&raster), Some(raster.rect));

    // A type layer, with or without its rasterized proxy, draws inside its rect.
    let mut doc = Document::new(200, 120, ColorMode::Rgb, BitDepth::Eight);
    let spec = pictura_core::TypeSpec::new("Type", "Liberation Sans", 32.0);
    let path = pictura_render::add_type_layer(&mut doc, "", &spec);
    let text = pictura_render::resolve_path_mut(&mut doc, &path).expect("type layer");
    assert_eq!(layer_visibility_region(text), Some(text.rect));
    text.channels.clear();
    assert_eq!(layer_visibility_region(text), Some(text.rect));

    let mask = |t, l, b, r, data: Option<Vec<u8>>, default_color, disabled| LayerMask {
        rect: PsdRect {
            top: t,
            left: l,
            bottom: b,
            right: r,
        },
        default_color,
        disabled,
        flags: 0,
        data: data.map(Into::into),
        ..Default::default()
    };

    // An adjustment with an enabled, data-carrying, zero-default mask is
    // bounded by its mask rect.
    let bounded = adjustment_layer(
        "invert",
        Some(mask(2, 3, 6, 9, Some(vec![128; 4 * 6]), 0, false)),
    )
    .expect("known kind");
    assert_eq!(
        layer_visibility_region(&bounded),
        Some(PsdRect {
            top: 2,
            left: 3,
            bottom: 6,
            right: 9,
        })
    );

    // No mask, a disabled mask, missing mask data, and a non-zero default
    // are all unbounded: full recomposite.
    let unmasked = adjustment_layer("invert", None).expect("known kind");
    assert_eq!(layer_visibility_region(&unmasked), None);
    let disabled = adjustment_layer(
        "invert",
        Some(mask(0, 0, 4, 4, Some(vec![128; 16]), 0, true)),
    )
    .expect("known kind");
    assert_eq!(layer_visibility_region(&disabled), None);
    let no_data =
        adjustment_layer("invert", Some(mask(0, 0, 4, 4, None, 0, false))).expect("known kind");
    assert_eq!(layer_visibility_region(&no_data), None);
    let non_zero = adjustment_layer(
        "invert",
        Some(mask(0, 0, 4, 4, Some(vec![128; 16]), 255, false)),
    )
    .expect("known kind");
    assert_eq!(layer_visibility_region(&non_zero), None);

    // A group is unbounded.
    let mut group = pixel_layer("group", 4, 4, (0, 0, 0));
    group.is_group = true;
    assert_eq!(layer_visibility_region(&group), None);

    // A layer effect (lfx2/lrFX) spills past the layer rect, so a bounded
    // region refresh cannot repair it: full recomposite.
    let mut effected = pixel_layer("effected", 8, 4, (10, 20, 30));
    effected.extra_blocks.push(LayerBlock {
        key: *b"lfx2",
        data: Vec::new(),
    });
    assert_eq!(layer_visibility_region(&effected), None);
    let mut legacy = pixel_layer("legacy", 8, 4, (10, 20, 30));
    legacy.extra_blocks.push(LayerBlock {
        key: *b"lrFX",
        data: Vec::new(),
    });
    assert_eq!(layer_visibility_region(&legacy), None);
}

#[test]
fn visibility_region_path_matches_full_recomposite() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 16, 16, (30, 60, 90)),
        pixel_layer("top", 6, 6, (200, 100, 50)),
    ];
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);

    // The region path: apply the toggle, union the changed regions, composite
    // and patch only that union.
    let mut region_doc = doc.clone();
    let (changed, region) = set_visible_paths_union(&mut region_doc, &["1"], false);
    assert_eq!(changed, 1);
    let rect = region.expect("raster layer is bounded");
    let (x0, y0, w, h) =
        clamp_region(rect, region_doc.width, region_doc.height).expect("non-empty region");
    let (buffer, _) = pictura_render::composite_region_active(&region_doc, rect, false);
    assert_eq!((buffer.width, buffer.height), (w, h));
    patch_composite_region(&mut region_doc, &buffer, x0, y0);

    // A full recomposite of the same state, stored the way `recomposite` does.
    let mut full_doc = doc.clone();
    assert_eq!(
        pictura_render::set_visible_paths(&mut full_doc, &["1"], false),
        1
    );
    let rendered = current_buffer(&full_doc, false);
    store_composite(&mut full_doc, &rendered);

    assert_eq!(
        buffer_to_image(&region_doc.composite),
        buffer_to_image(&full_doc.composite),
        "region path must match a full recomposite byte-for-byte"
    );
}

#[test]
fn visibility_union_falls_back_for_unbounded_layer() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut group = pixel_layer("group", 4, 4, (9, 9, 9));
    group.is_group = true;
    doc.layers = vec![pixel_layer("base", 4, 4, (0, 0, 0)), group];

    // A group is unbounded, so the union is `None` and the caller falls back.
    let (changed, region) = set_visible_paths_union(&mut doc, &["1"], false);
    assert_eq!(changed, 1);
    assert_eq!(region, None);

    // Two bounded layers union into their bounding box.
    let mut two = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let mut far = pixel_layer("far", 3, 3, (1, 1, 1));
    far.rect = PsdRect {
        top: 4,
        left: 4,
        bottom: 7,
        right: 7,
    };
    two.layers = vec![pixel_layer("base", 2, 2, (0, 0, 0)), far];
    let (changed, region) = set_visible_paths_union(&mut two, &["0", "1"], false);
    assert_eq!(changed, 2);
    assert_eq!(
        region,
        Some(PsdRect {
            top: 0,
            left: 0,
            bottom: 7,
            right: 7,
        })
    );
}

#[test]
fn active_layer_visible_tracks_the_active_raster_layer() {
    let mut hidden = pixel_layer("hidden", 4, 4, (1, 2, 3));
    hidden.visible = false;
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 4, 4, (0, 0, 0)), hidden];

    assert!(active_layer_visible(&doc, Some("0")));
    assert!(!active_layer_visible(&doc, Some("1")));
    // No active layer, a group, and an unknown path are not editable targets
    // and are not treated as invisible.
    assert!(active_layer_visible(&doc, None));
    assert!(active_layer_visible(&doc, Some("")));
    assert!(active_layer_visible(&doc, Some("9")));
    let mut group = pixel_layer("group", 4, 4, (0, 0, 0));
    group.is_group = true;
    doc.layers.push(group);
    assert!(active_layer_visible(&doc, Some("2")));
}

#[test]
fn invisible_active_layer_refuses_edits_but_not_move() {
    let mut hidden = pixel_layer("hidden", 8, 8, (200, 100, 50));
    hidden.visible = false;
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (30, 60, 90)), hidden];

    // Paint and filter gate on `active_layer_visible`, so both refuse.
    assert!(!active_layer_visible(&doc, Some("1")));
    // Move resolves the same layer through `active_pixel_layer`, which is
    // visibility-agnostic, so the layer is still a move target.
    assert!(active_pixel_layer(&doc, Some("1")).is_some());
}

#[test]
fn move_preview_base_preserves_layer_visibility() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 8, 8, (30, 60, 90)),
        pixel_layer("top", 8, 8, (200, 100, 50)),
    ];
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    doc.layers[1].visible = false;

    // The region path.
    let base = build_move_preview_base(&mut doc, "1", false);
    assert!(
        !doc.layers[1].visible,
        "a move must not reveal the invisible layer"
    );
    assert!(!base.is_null());

    // The full-composite fallback (a missing/mismatched composite).
    let mut no_composite = doc.clone();
    no_composite.composite.data.clear();
    no_composite.layers[1].visible = false;
    let _ = build_move_preview_base(&mut no_composite, "1", false);
    assert!(
        !no_composite.layers[1].visible,
        "the fallback must restore the prior visibility"
    );
}

#[test]
fn duplicate_move_target_duplicates_and_repoints_the_active_layer() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 8, 8, (30, 60, 90)),
        pixel_layer("top", 8, 8, (200, 100, 50)),
    ];
    let mut active = Some("1".to_string());
    let new_path = duplicate_move_target(&mut doc, &mut active).expect("duplicate");
    assert_eq!(new_path, "2");
    assert_eq!(doc.layers.len(), 3, "one layer is added");
    assert_eq!(doc.layers[2].name, "top copy");
    assert_eq!(active.as_deref(), Some("2"), "the copy becomes active");
}

#[test]
fn duplicate_move_target_preserves_visibility_and_refuses_locks() {
    let mut hidden = pixel_layer("hidden", 8, 8, (1, 2, 3));
    hidden.visible = false;
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (0, 0, 0)), hidden];
    let mut active = Some("1".to_string());
    assert_eq!(
        duplicate_move_target(&mut doc, &mut active),
        Some("2".to_string())
    );
    assert!(
        !doc.layers[2].visible,
        "the clone of an invisible layer stays invisible"
    );

    // A position-locked target refuses without mutating the document or the
    // active path, and a group is not a target at all.
    let mut locked = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    locked.layers = vec![pixel_layer("base", 8, 8, (0, 0, 0))];
    locked.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
    let mut active = Some("0".to_string());
    let before = locked.clone();
    assert!(duplicate_move_target(&mut locked, &mut active).is_none());
    assert_eq!(locked, before, "a locked target refuses without mutation");
    assert_eq!(active.as_deref(), Some("0"));

    let mut group = pixel_layer("group", 8, 8, (0, 0, 0));
    group.is_group = true;
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![group];
    assert!(duplicate_move_target(&mut doc, &mut Some("0".to_string())).is_none());
}

#[test]
fn move_duplicate_records_one_state_and_undo_restores_the_layer_count() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (30, 60, 90))];
    let mut history = History::default();
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Open",
    );
    let before = history.count();

    let mut active = Some("0".to_string());
    let new_path = duplicate_move_target(&mut doc, &mut active).expect("duplicate");
    assert_eq!(history.count(), before, "the drag start records no state");

    // The commit: translate the clone, then the bridge's one `record_move`.
    assert!(pictura_render::translate_layer_path(
        &mut doc, &new_path, 2, 2
    ));
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Move Layer",
    );
    assert_eq!(
        history.count(),
        before + 1,
        "the commit is exactly one state"
    );
    assert_eq!(history.label(before), "Move Layer");

    let restored = history.undo().expect("undo");
    assert_eq!(restored.doc.layers.len(), 1, "undo removes the clone");
}

#[test]
fn selection_becomes_full_frame_mask_that_confines_adjustment() {
    let mut doc = Document::new(4, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 4, 1, (255, 0, 0))];

    let selection = Selection {
        width: 4,
        height: 1,
        data: vec![255, 255, 0, 0],
    };
    let mask = selection_to_mask(&selection, &doc);
    assert_eq!(mask.rect.right, 4);
    assert_eq!(mask.rect.bottom, 1);
    assert_eq!(mask.data.as_deref(), Some(&[255u8, 255, 0, 0][..]));

    doc.layers
        .push(adjustment_layer("invert", Some(mask)).expect("known kind"));
    let image = document_to_image(&doc, false);
    // Selected half inverts red -> cyan; unselected half is untouched.
    assert_eq!(image.pixel_color(0, 0).red(), 0);
    assert_eq!(image.pixel_color(0, 0).green(), 255);
    assert_eq!(image.pixel_color(0, 0).blue(), 255);
    assert_eq!(image.pixel_color(3, 0).red(), 255);
    assert_eq!(image.pixel_color(3, 0).blue(), 0);
}

#[test]
fn test_image_is_not_null() {
    assert!(!test_image().is_null());
}

#[test]
fn store_composite_stores_rgba_for_rgb_and_keeps_grayscale_plane() {
    let mut rgb = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    rgb.layers = vec![pixel_layer("base", 8, 8, (30, 60, 90))];
    let rendered = current_buffer(&rgb, false);
    assert_eq!(rendered.channels, 4);
    store_composite(&mut rgb, &rendered);
    assert_eq!(rgb.composite.channels, 4);
    assert_eq!(rgb.composite.data, rendered.data);

    let mut gray = Document::new(8, 8, ColorMode::Grayscale, BitDepth::Eight);
    gray.layers = vec![Layer {
        name: "gray".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![120; 64].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 64].into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }];
    let rendered = current_buffer(&gray, false);
    assert_eq!(rendered.channels, 4);
    store_composite(&mut gray, &rendered);
    assert_eq!(gray.composite.channels, 1, "grayscale stays one plane");
    assert_eq!(
        gray.composite.data,
        rendered.data[..64],
        "stored grey plane"
    );
}

#[test]
fn undo_display_from_snapshot_composite_equals_full_recomposite() {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 16, 16, (30, 60, 90)),
        pixel_layer("top", 6, 6, (200, 100, 50)),
    ];
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    let open = Snapshot {
        doc: doc.clone(),
        selection: None,
    };
    let mut history = History::default();
    history.capture(open, "Open");

    // A full-composite mutation, captured after the store.
    assert!(pictura_render::translate_layer(&mut doc, 3, 2));
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Move Layer",
    );

    // A region-refresh mutation (`refresh_region`'s non-painting body).
    let before = doc.layers.last().unwrap().rect;
    assert!(pictura_render::translate_layer_rect(&mut doc, 2, 1));
    let after = doc.layers.last().unwrap().rect;
    let dirty = union_rect(before, after);
    let (x0, y0, w, h) = clamp_region(dirty, doc.width, doc.height).expect("in-bounds");
    assert_eq!((w, h), (dirty.width() as u32, dirty.height() as u32));
    let (buffer, _) = pictura_render::composite_region_active(&doc, dirty, false);
    assert_eq!((buffer.width, buffer.height), (w, h));
    patch_composite_region(&mut doc, &buffer, x0, y0);
    history.capture(
        Snapshot {
            doc: doc.clone(),
            selection: None,
        },
        "Move Layer",
    );

    // Walk the undo stack; every restored display must be the snapshot
    // composite and equal a full recomposite of the restored document.
    let mut undone = 0;
    while let Some(snapshot) = history.undo() {
        let display = buffer_to_image(&snapshot.doc.composite);
        let full = document_to_image(&snapshot.doc, false);
        assert_eq!(
            display, full,
            "undo display differs from a full recomposite"
        );
        undone += 1;
    }
    assert_eq!(undone, 2);
    // And back up the redo stack.
    let mut redone = 0;
    while let Some(snapshot) = history.redo() {
        let display = buffer_to_image(&snapshot.doc.composite);
        let full = document_to_image(&snapshot.doc, false);
        assert_eq!(
            display, full,
            "redo display differs from a full recomposite"
        );
        redone += 1;
    }
    assert_eq!(redone, 2);
}

/// The region path patches `doc.composite` in place (no FFI) and the image
/// rebuilt from it equals a full recomposite, across several refreshes.
#[test]
fn region_refresh_keeps_composite_and_rebuilt_image_equal_to_full() {
    let mut doc = Document::new(32, 32, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data.fill(255);
    doc.layers = vec![
        pixel_layer("base", 32, 32, (30, 60, 90)),
        pixel_layer("top", 8, 8, (200, 100, 50)),
    ];
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);

    // Replay `refresh_region`'s non-painting body for a sequence of moves:
    // patch the composite, then rebuild the display image from it as
    // `image()` does when dirty.
    for (dx, dy) in [(2, 1), (1, 3), (0, 2), (3, 0)] {
        let before = doc.layers.last().unwrap().rect;
        assert!(pictura_render::translate_layer_rect(&mut doc, dx, dy));
        let after = doc.layers.last().unwrap().rect;
        let dirty = union_rect(before, after);
        let (x0, y0, w, h) = clamp_region(dirty, doc.width, doc.height).expect("in-bounds");
        let (buffer, _) = pictura_render::composite_region_active(&doc, dirty, false);
        assert_eq!((buffer.width, buffer.height), (w, h));
        patch_composite_region(&mut doc, &buffer, x0, y0);
        let image = buffer_to_image(&doc.composite);
        let full = document_to_image(&doc, false);
        assert_eq!(
            image, full,
            "region-refreshed image differs after move ({dx},{dy})"
        );
    }
}

#[test]
fn sample_planar_argb_matches_buffer_to_image_for_every_plane_count() {
    for channels in 1..=4u8 {
        let mut buffer = PixelBuffer::new(3, 2, channels);
        let plane = 3 * 2;
        for c in 0..channels as usize {
            for i in 0..plane {
                buffer.data[c * plane + i] = ((c * 37 + i * 11 + 5) % 256) as u8;
            }
        }
        let image = buffer_to_image(&buffer);
        for y in 0..2 {
            for x in 0..3 {
                let color = image.pixel_color(x, y);
                let expect = ((color.alpha() as u32) << 24)
                    | ((color.red() as u32) << 16)
                    | ((color.green() as u32) << 8)
                    | (color.blue() as u32);
                assert_eq!(
                    sample_planar_argb(&buffer, x, y),
                    expect,
                    "channels={channels} pixel ({x},{y})"
                );
            }
        }
        assert_eq!(sample_planar_argb(&buffer, -1, 0), 0);
        assert_eq!(sample_planar_argb(&buffer, 0, 2), 0);
    }
}

#[test]
fn layer_can_edit_smart_object_contents_eligibility() {
    use pictura_core::{SmartObject, SmartObjectKind};

    let payload = {
        let mut src = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        let n = 4usize;
        for i in 0..n {
            src.composite.data[i] = 10;
            src.composite.data[n + i] = 20;
            src.composite.data[2 * n + i] = 30;
        }
        pictura_codec::write_psd(&src).expect("payload writes")
    };
    let smart = |payload: Option<Vec<u8>>| {
        let mut layer = pixel_layer("smart", 4, 4, (1, 2, 3));
        layer.smart_object = Some(SmartObject {
            kind: SmartObjectKind::Embedded,
            payload,
            filename: "source.psd".into(),
            ..Default::default()
        });
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![layer];
        doc
    };

    let valid = smart(Some(payload.clone()));
    let before = valid.clone();
    assert!(pictura_render::can_edit_smart_object_contents(&valid, "0"));

    assert!(!pictura_render::can_edit_smart_object_contents(
        &smart(Some(vec![0, 1, 2, 3])),
        "0"
    ));
    assert!(!pictura_render::can_edit_smart_object_contents(
        &smart(Some(Vec::new())),
        "0"
    ));
    assert!(!pictura_render::can_edit_smart_object_contents(
        &smart(None),
        "0"
    ));

    let mut grouped = smart(Some(payload.clone()));
    grouped.layers[0].is_group = true;
    assert!(!pictura_render::can_edit_smart_object_contents(
        &grouped, "0"
    ));

    let mut external = smart(Some(payload.clone()));
    external.layers[0].smart_object.as_mut().unwrap().kind = SmartObjectKind::External;
    assert!(!pictura_render::can_edit_smart_object_contents(
        &external, "0"
    ));

    let mut adjustment = smart(Some(payload));
    adjustment.layers[0].adjustment = Some(pictura_render::encode_invert());
    assert!(!pictura_render::can_edit_smart_object_contents(
        &adjustment,
        "0"
    ));

    assert!(!pictura_render::can_edit_smart_object_contents(
        &valid, "99"
    ));
    assert!(!pictura_render::can_edit_smart_object_contents(
        &valid, "bad"
    ));

    assert_eq!(valid, before, "the predicate must not mutate the document");
}

#[test]
fn save_after_edit_serializes_the_current_composite() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (40, 40, 40))];
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    let plane = 64usize;
    let before = doc.composite.data[..3 * plane].to_vec();

    // Edit every colour channel, then make the composite current (the app's
    // `recomposite`). The pre-edit merged image is no longer the composite.
    let layer = doc.layers.last_mut().unwrap();
    for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
        for byte in ch.data.iter_mut() {
            *byte = byte.wrapping_add(100);
        }
    }
    let rendered = current_buffer(&doc, false);
    store_composite(&mut doc, &rendered);
    assert_eq!(doc.composite.channels, 4);
    let edited = doc.composite.data[..3 * plane].to_vec();
    assert_ne!(edited, before, "the edit must change the composite");

    let bytes = pictura_codec::write_psd(&doc).expect("write psd");
    let back = pictura_codec::read_psd(&bytes).expect("read psd");
    assert_eq!(
        back.composite.channels, 3,
        "RGB read-back is the mode's planes"
    );
    assert_eq!(
        back.composite.data, edited,
        "reopened composite is the edited render, not the pre-edit one"
    );
}

/// A `POSITION` lock does not block a pixel edit: the topmost selector is not
/// filtered by position lock (the D5 warning).
#[test]
fn topmost_selector_still_edits_under_position_lock() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (40, 40, 40))];
    doc.layers[0].lock = LockFlags::default().with(LockFlags::POSITION, true);
    let layer =
        active_pixel_layer_mut(&mut doc, Some("0")).expect("selector finds the pixel layer");
    assert!(pictura_render::apply_filter(
        layer,
        &pictura_filters::Filter::GaussianBlur { radius: 1.0 },
        None,
        false,
    )
    .is_ok());
}

#[test]
fn filter_refused_and_unchanged_on_pixel_locked_layer() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("base", 8, 8, (40, 40, 40))];
    doc.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
    let before = doc.layers[0].clone();
    let layer =
        active_pixel_layer_mut(&mut doc, Some("0")).expect("selector finds the pixel layer");
    let err = pictura_render::apply_filter(
        layer,
        &pictura_filters::Filter::GaussianBlur { radius: 1.0 },
        None,
        false,
    )
    .unwrap_err();
    assert!(
        matches!(err, pictura_filters::FilterError::Locked),
        "{err:?}"
    );
    assert_eq!(
        doc.layers[0], before,
        "a refused filter leaves the layer bit-identical"
    );
}

#[test]
fn content_move_refused_on_locked_layer() {
    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(vec![255; 64].into()),
        ..Default::default()
    };
    for flag in [LockFlags::POSITION, LockFlags::PIXELS] {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = vec![pixel_layer("base", 8, 8, (40, 40, 40))];
        doc.layers[0].lock = LockFlags::default().with(flag, true);
        let before = doc.clone();
        assert!(
            !pictura_render::move_selection_content(&mut doc, "0", &mask, 2, 0, false),
            "flag {flag} refuses the content move"
        );
        assert_eq!(doc, before, "refusal leaves the document unchanged");
    }
}

#[test]
fn finalize_import_marks_opaque_import_as_locked_background() {
    let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
    let mut doc = Document::from_rgba("photo", 2, 1, &rgba);
    finalize_import(&mut doc, &rgba);
    assert_eq!(doc.layers.len(), 1);
    let layer = &doc.layers[0];
    assert_eq!(layer.name, "Background");
    assert!(layer.background, "opaque import becomes the Background");
    assert_eq!(
        layer.lock.bits(),
        LockFlags::TRANSPARENCY | LockFlags::POSITION,
        "Background locks transparency and position only"
    );
    assert!(
        !layer.channels.iter().any(|channel| channel.id == -1),
        "the redundant opaque alpha channel is dropped"
    );
}

#[test]
fn finalize_import_keeps_non_opaque_import_as_regular_alpha_layer() {
    let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
    let mut doc = Document::from_rgba("photo", 2, 1, &rgba);
    finalize_import(&mut doc, &rgba);
    let layer = &doc.layers[0];
    assert_eq!(layer.name, "photo", "the file stem names the alpha layer");
    assert!(
        !layer.background,
        "a non-opaque import stays a regular layer"
    );
    assert_eq!(layer.lock, LockFlags::default(), "no locks are added");
    assert!(
        layer.channels.iter().any(|channel| channel.id == -1),
        "the real alpha channel is preserved"
    );
}

#[test]
fn active_layer_resolution_walks_the_tree_to_the_leaf() {
    let mut group = pixel_layer("group", 8, 8, (0, 0, 0));
    group.is_group = true;
    group.children = vec![pixel_layer("nested", 8, 8, (0, 0, 0))];
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("base", 8, 8, (40, 40, 40)),
        group,
        adjustment_layer("invert", None).expect("known kind"),
    ];

    assert_eq!(
        active_pixel_layer(&doc, Some("0"))
            .expect("chosen layer")
            .name,
        "base"
    );
    assert!(
        active_pixel_layer(&doc, Some("1")).is_none(),
        "a group has no target"
    );
    assert!(
        active_pixel_layer(&doc, Some("2")).is_none(),
        "an adjustment has no target"
    );
    assert!(
        active_pixel_layer(&doc, Some("1/0")).is_some_and(|layer| layer.name == "nested"),
        "a nested path resolves to its leaf"
    );
    assert!(
        active_pixel_layer(&doc, Some("0/0")).is_none(),
        "a leaf has no children to resolve"
    );
    assert!(
        active_pixel_layer(&doc, Some("")).is_none(),
        "an empty (multi) selection has no target"
    );
    assert!(
        active_pixel_layer(&doc, None).is_none(),
        "no active path has no target"
    );
    assert!(
        active_pixel_layer(&doc, Some("9")).is_none(),
        "an out-of-range path has no target"
    );

    active_pixel_layer_mut(&mut doc, Some("0"))
        .expect("mutable target")
        .name = "renamed".to_string();
    assert_eq!(doc.layers[0].name, "renamed");
    assert_eq!(doc.layers[1].name, "group", "other layers are untouched");

    active_pixel_layer_mut(&mut doc, Some("1/0"))
        .expect("mutable nested target")
        .name = "leaf".to_string();
    assert_eq!(doc.layers[1].children[0].name, "leaf");
}

#[test]
fn nested_layer_paint_targets_the_leaf_and_refuses_a_group() {
    use pictura_paint::{paint_stroke, Rgba, StrokeConfig, StrokeSample};

    let nested = pixel_layer("nested", 8, 8, (0, 0, 0));
    let nested_before = nested
        .channels
        .iter()
        .find(|c| c.id == 0)
        .unwrap()
        .data
        .clone();
    let mut group = pixel_layer("folder", 8, 8, (0, 0, 0));
    group.is_group = true;
    group.children = vec![nested];
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![group, pixel_layer("base", 8, 8, (10, 20, 30))];
    let base_before = doc.layers[1].clone();

    assert!(active_pixel_layer(&doc, Some("0/0")).is_some());
    assert!(
        active_pixel_layer(&doc, Some("0")).is_none(),
        "a group is refused"
    );

    let cfg = StrokeConfig {
        color: Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        diameter: 6,
        ..StrokeConfig::default()
    };
    let samples = [StrokeSample {
        x: 4.0,
        y: 4.0,
        pressure: 1.0,
    }];
    paint_stroke(&mut doc, "0/0", &cfg, &samples).expect("paint the nested leaf");
    let nested_after = doc.layers[0].children[0]
        .channels
        .iter()
        .find(|c| c.id == 0)
        .unwrap()
        .data
        .clone();
    assert_ne!(
        nested_after.as_ref(),
        nested_before.as_ref(),
        "the stroke lands in the nested leaf"
    );
    assert_eq!(doc.layers[1], base_before, "the sibling is untouched");
}

#[test]
fn brush_shortcut_delta_maps_us_and_native_keys() {
    // US layout keys: `[`/`]` size, `{`/`}` hardness.
    assert_eq!(brush_shortcut_delta(0x5B, 0, false, true), -1);
    assert_eq!(brush_shortcut_delta(0x5D, 0, false, true), 1);
    assert_eq!(brush_shortcut_delta(0x7B, 0, true, true), -5);
    assert_eq!(brush_shortcut_delta(0x7D, 0, true, true), 5);

    // Nordic physical keys: the evdev scan code wins over the key value.
    assert_eq!(brush_shortcut_delta(0, 34, false, true), -1);
    assert_eq!(brush_shortcut_delta(0, 35, false, true), 1);
    assert_eq!(brush_shortcut_delta(0, 34, true, true), -5);
    assert_eq!(brush_shortcut_delta(0, 35, true, true), 5);
    assert_eq!(brush_shortcut_delta(0x5B, 999, false, true), -1);

    // Other keys and non-paint tools are ignored.
    assert_eq!(brush_shortcut_delta(0x41, 0, false, true), 0);
    assert_eq!(brush_shortcut_delta(0x5D, 35, false, false), 0);
    assert_eq!(brush_shortcut_delta(0x5D, 35, true, false), 0);
}

#[test]
fn placed_smart_object_predicate_accepts_external_and_alias() {
    use pictura_core::{SmartObject, SmartObjectKind};

    let with_kind = |kind: SmartObjectKind| {
        let mut layer = pixel_layer("placed", 2, 2, (1, 2, 3));
        layer.smart_object = Some(SmartObject {
            kind,
            ..Default::default()
        });
        layer
    };

    assert!(is_placed_smart_object(&with_kind(
        SmartObjectKind::External
    )));
    assert!(is_placed_smart_object(&with_kind(SmartObjectKind::Alias)));
    assert!(!is_placed_smart_object(&with_kind(
        SmartObjectKind::Embedded
    )));
    assert!(!is_placed_smart_object(&with_kind(
        SmartObjectKind::Unresolved
    )));
    assert!(!is_placed_smart_object(&pixel_layer(
        "plain",
        2,
        2,
        (1, 2, 3)
    )));
}

#[test]
fn type_layer_kind_reports_type() {
    use pictura_core::LayerBlock;

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("type", 2, 2, (10, 20, 30));
    layer.extra_blocks = vec![LayerBlock {
        key: *b"TySh",
        data: vec![1, 2, 3, 4],
    }];
    doc.layers = vec![layer];
    assert_eq!(
        layer_kind_str(&doc, "0", &doc.layers[0]).to_string(),
        "type"
    );
    assert_eq!(
        layer_kind_str(&doc, "0", &pixel_layer("plain", 2, 2, (1, 2, 3))).to_string(),
        "pixel"
    );
}

#[test]
fn buffer_to_rgba_bytes_matches_image_pixels() {
    let gray = PixelBuffer {
        width: 1,
        height: 1,
        channels: 1,
        data: vec![42].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&gray), vec![42, 42, 42, 255]);

    let gray_alpha = PixelBuffer {
        width: 1,
        height: 1,
        channels: 2,
        data: vec![10, 200].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&gray_alpha), vec![10, 10, 10, 200]);

    let rgb = PixelBuffer {
        width: 1,
        height: 1,
        channels: 3,
        data: vec![1, 2, 3].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&rgb), vec![1, 2, 3, 255]);

    let rgba = PixelBuffer {
        width: 1,
        height: 1,
        channels: 4,
        data: vec![4, 5, 6, 7].into(),
    };
    assert_eq!(buffer_to_rgba_bytes(&rgba), vec![4, 5, 6, 7]);

    let image = buffer_to_image(&rgba);
    assert_eq!(image.pixel_color(0, 0).red(), 4);
    assert_eq!(image.pixel_color(0, 0).green(), 5);
    assert_eq!(image.pixel_color(0, 0).blue(), 6);
    assert_eq!(image.pixel_color(0, 0).alpha(), 7);
}

#[test]
fn output_format_classification_matches_suffix() {
    assert_eq!(format_for_path("/tmp/photo.PNG"), "png");
    assert_eq!(format_for_path("/tmp/doc.psd"), "psd");
    assert_eq!(format_for_path("/tmp/noext"), "psd");

    assert_eq!(raster_writer_for_suffix("png"), Some("PNG"));
    assert_eq!(raster_writer_for_suffix("jpeg"), Some("JPG"));
    assert_eq!(raster_writer_for_suffix("jpe"), Some("JPG"));
    assert_eq!(raster_writer_for_suffix("tiff"), Some("TIF"));
    assert_eq!(raster_writer_for_suffix("webp"), Some("WEBP"));
    assert_eq!(raster_writer_for_suffix("bmp"), Some("BMP"));
    assert_eq!(raster_writer_for_suffix("psd"), None);
    assert_eq!(raster_writer_for_suffix("gif"), None);

    assert_eq!(super::PictureViewRust::default().source_format, "psd");
}
