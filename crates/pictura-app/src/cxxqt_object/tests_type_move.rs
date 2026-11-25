//! Move-preview bounds for type layers.

use super::impl_transform::{move_content_rect, translate_move_group};
use super::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, LayerBlock, PsdRect, TypeSpec};

#[test]
fn a_type_move_previews_and_damages_its_ink_not_the_whole_box() {
    let mut doc = Document::new(2000, 2000, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 2000, 2000, (30, 60, 90))]);
    let spec = TypeSpec {
        box_bounds: Some([0.0, 0.0, 1000.0, 1000.0]),
        ..TypeSpec::new("Type move", "Liberation Sans", 20.0)
    };
    let path = pictura_render::add_type_layer(&mut doc, "0", &spec);
    assert!(!path.is_empty(), "a paragraph type layer");
    let box_rect = pictura_render::resolve_path(&doc, &path)
        .expect("type layer")
        .rect;
    assert!(
        box_rect.width() >= 1000,
        "the paragraph box is the layer rect"
    );

    // A box this size is not near-canvas, so the edit helper keeps it; the move
    // helper trims it to the words.
    assert_eq!(
        pictura_render::damage::ink_influence(&doc, &path),
        box_rect.clip_to(doc.width, doc.height),
        "the edit helper keeps a box below three quarters of the canvas"
    );
    let (rect, offset) = move_content_rect(&doc, std::slice::from_ref(&path)).expect("type crop");
    assert!(
        rect.width() < box_rect.width() && rect.height() < box_rect.height(),
        "the moving image is the ink, not the box: {rect:?}"
    );
    assert_eq!(
        offset,
        (rect.left - box_rect.left, rect.top - box_rect.top),
        "the crop offset is measured from the layer rect"
    );

    // An empty/disabled effects block still makes the conservative reach the
    // canvas; the move trims it to the ink.
    {
        let layer = pictura_render::resolve_path_mut(&mut doc, &path).expect("type layer");
        layer.transparency_shapes = false;
        pictura_codec::edit_layer(layer, |parts| {
            parts.extra_blocks.push(LayerBlock {
                key: *b"lfx2",
                data: Vec::new().into(),
            });
        });
    }
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    assert_eq!(
        pictura_render::damage::influence(&doc, &path),
        Some(canvas),
        "an effects block keeps the conservative canvas reach"
    );
    let trimmed = pictura_render::damage::type_ink_influence(&doc, &path).expect("a trimmed reach");
    assert!(
        trimmed.width() < box_rect.width() && trimmed.height() < box_rect.height(),
        "an empty effects block does not stop the ink trim: {trimmed:?}"
    );

    let before_ink = pictura_render::layer_content_bounds(&doc, &path).expect("the text inks");
    let dirty = translate_move_group(&mut doc, std::slice::from_ref(&path), 5, 7)
        .expect("the type layer moves");
    assert!(
        dirty.width() < box_rect.width() && dirty.height() < box_rect.height(),
        "the damage is the ink, not the box: {dirty:?}"
    );
    let moved = pictura_render::resolve_path(&doc, &path).expect("moved type");
    assert_eq!((moved.rect.left, moved.rect.top), (5, 7));
    assert!(moved.is_type(), "the type stays editable");
    let after_ink = pictura_render::layer_content_bounds(&doc, &path).expect("the moved text inks");
    assert_eq!(
        (after_ink.left, after_ink.top),
        (before_ink.left + 5, before_ink.top + 7),
        "the layer edges (content bounds) follow the moved text"
    );
}

#[test]
fn a_channel_less_type_layer_materializes_for_a_move() {
    let mut doc = Document::new(400, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 400, 400, (30, 60, 90))]);
    let spec = TypeSpec {
        box_bounds: Some([0.0, 0.0, 400.0, 400.0]),
        ..TypeSpec::new("Type move", "Liberation Sans", 20.0)
    };
    let path = pictura_render::add_type_layer(&mut doc, "0", &spec);
    assert!(!path.is_empty(), "a paragraph type layer");
    let layer = pictura_render::resolve_path_mut(&mut doc, &path).expect("type layer");
    layer.channels_mut().clear();
    assert!(
        pictura_render::resolve_path(&doc, &path)
            .is_some_and(|layer| !layer.channels().iter().any(|c| c.id == 0)),
        "the layer starts channel-less"
    );

    assert!(pictura_render::materialize_type_layer(&mut doc, &path));
    assert!(
        pictura_render::resolve_path(&doc, &path)
            .is_some_and(|layer| layer.channels().iter().any(|c| c.id == 0)),
        "the move materializes the type raster"
    );
    let (rect, _) = move_content_rect(&doc, std::slice::from_ref(&path)).expect("type crop");
    assert!(
        rect.width() < 400 && rect.height() < 400,
        "the materialized ink bounds the move: {rect:?}"
    );
}

#[test]
fn a_clipped_type_layer_move_trims_to_its_ink() {
    let mut doc = Document::new(400, 400, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("base", 400, 400, (30, 60, 90))]);
    let spec = TypeSpec {
        box_bounds: Some([0.0, 0.0, 400.0, 400.0]),
        ..TypeSpec::new("Type move", "Liberation Sans", 20.0)
    };
    let path = pictura_render::add_type_layer(&mut doc, "0", &spec);
    assert!(!path.is_empty(), "a paragraph type layer");
    if let Some(layer) = pictura_render::resolve_path_mut(&mut doc, &path) {
        layer.clipping = true;
    }
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: 400,
        right: 400,
    };
    assert_eq!(
        pictura_render::damage::influence(&doc, &path),
        Some(canvas),
        "a clipped member's conservative reach is its base"
    );
    let (rect, _) = move_content_rect(&doc, std::slice::from_ref(&path)).expect("type crop");
    assert!(
        rect.width() < 400 && rect.height() < 400,
        "a clipped type layer's move is its own ink: {rect:?}"
    );
}
