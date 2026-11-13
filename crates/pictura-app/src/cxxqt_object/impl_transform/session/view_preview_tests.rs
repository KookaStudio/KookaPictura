use super::*;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{BitDepth, ColorMode, Layer, LayerMask, LayerStack};
use pictura_render::{ShapeStroke, StrokePosition};

const W: u32 = 2400;
const H: u32 = 2000;
const UNDER: (u8, u8, u8) = (30, 60, 90);
const OVER: (u8, u8, u8) = (200, 100, 50);

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

/// A background under a canvas-sized opaque layer: 4.8 MP of content.
fn document() -> Document {
    let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("under", W, H, UNDER),
        pixel_layer("over", W, H, OVER),
    ]);
    doc
}

fn paths() -> Vec<String> {
    vec!["1".to_string()]
}

/// A background under a large rectangle shape. Its reach is about 4.4 MP, over
/// the view-preview threshold.
fn shape_document(stroke: Option<ShapeStroke>) -> (Document, String) {
    let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("under", W, H, UNDER)]);
    let outline = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (50.0, 50.0, 2350.0, 1950.0),
    )
    .expect("outline");
    let path =
        pictura_render::add_shape_layer(&mut doc, "", [200, 100, 50, 255], "Shape", &outline, None);
    if let Some(stroke) = stroke {
        let layer = pictura_render::resolve_path_mut(&mut doc, &path).expect("shape");
        assert!(pictura_render::set_shape_stroke(layer, Some(&stroke)));
    }
    (doc, path)
}

fn rgb(image: &QImage, x: i32, y: i32) -> (u8, u8, u8) {
    let c = image.pixel_color(x, y);
    (c.red() as u8, c.green() as u8, c.blue() as u8)
}

// [mvp_view_preview]
#[test]
fn a_large_layer_previews_from_the_view_at_its_level() {
    let doc = document();
    let visible = rect(400, 400, 1200, 1000);
    let preview = view_preview(&doc, &paths(), 2, visible, false).expect("view preview");
    assert_eq!(preview.view.level, 2);
    assert_eq!(preview.view.scale(), 4);
    assert_eq!(preview.view.content, rect(0, 0, W as i32, H as i32));

    // The backdrop covers the layer's on-screen part only, a quarter size.
    assert_eq!(preview.base_origin, (400, 400));
    assert_eq!((preview.base.width(), preview.base.height()), (200, 150));
    assert_eq!(rgb(&preview.base, 100, 75), UNDER, "the layer is hidden");

    // The moving image reaches a quarter view past each edge, on the level grid.
    assert_eq!(preview.view.region, rect(200, 248, 1400, 1152));
    assert_eq!((preview.layer.width(), preview.layer.height()), (300, 226));
    assert_eq!(rgb(&preview.layer, 150, 100), OVER);
}

// [mvp_view_preview]
#[test]
fn level_zero_composites_the_backdrop_exactly() {
    let doc = document();
    let visible = rect(10, 20, 330, 260);
    let preview = view_preview(&doc, &paths(), 0, visible, false).expect("view preview");
    assert_eq!(preview.base_origin, (10, 20));
    assert_eq!((preview.base.width(), preview.base.height()), (320, 240));
    assert_eq!(rgb(&preview.base, 0, 0), UNDER);
    assert_eq!(preview.view.region, rect(0, 0, 410, 320));
}

// [mvp_shape_view]
#[test]
fn a_large_shape_previews_from_the_view_at_its_level() {
    let (doc, path) = shape_document(None);
    let visible = rect(400, 400, 1200, 1000);
    let preview = view_preview(&doc, std::slice::from_ref(&path), 2, visible, false)
        .expect("shape view preview");
    assert_eq!(preview.view.level, 2);
    let reach = pictura_render::damage::content_influence(&doc, &path).expect("a bounded shape");
    assert_eq!(preview.view.content, reach);

    // The backdrop is the background over the on-screen part, a quarter size.
    assert_eq!(preview.base_origin, (400, 400));
    assert_eq!((preview.base.width(), preview.base.height()), (200, 150));
    assert_eq!(rgb(&preview.base, 100, 75), UNDER, "the shape is hidden");

    // The moving image is the shape alone over the view's region at level 2.
    assert_eq!(preview.view.region, rect(200, 248, 1400, 1152));
    assert_eq!((preview.layer.width(), preview.layer.height()), (300, 226));
    assert_eq!(rgb(&preview.layer, 25, 13), OVER);
}

// [mvp_shape_stroke]
#[test]
fn a_stroked_shape_preview_scales_its_effect() {
    let stroke = ShapeStroke {
        color: [0, 255, 0],
        width: 20,
        position: StrokePosition::Inside,
    };
    let (doc, path) = shape_document(Some(stroke));
    let visible = rect(0, 0, 800, 600);
    let preview =
        view_preview(&doc, &[path], 2, visible, false).expect("stroked shape view preview");
    assert_eq!(preview.view.level, 2);

    // The 20 px stroke is inside the top edge at document y = 50..70. At level
    // 2 it occupies about five image rows, not the full twenty.
    let column = (500 - preview.view.region.left) / 4;
    let green = (0..preview.layer.height())
        .filter(|&y| {
            let (r, g, _) = rgb(&preview.layer, column, y);
            r < 50 && g > 200
        })
        .count();
    assert!(
        (2..=8).contains(&green),
        "the stroke should be scaled to the level, got {green} rows"
    );
}

// [mvp_shape_follow]
#[test]
fn a_shape_drag_past_the_margin_rebuilds_the_moving_image() {
    let (doc, path) = shape_document(None);
    let visible = rect(400, 400, 1200, 1000);
    let view = view_preview(&doc, std::slice::from_ref(&path), 2, visible, false)
        .expect("shape view preview")
        .view;
    assert!(
        follow(&doc, &path, view, 100, 80, false).is_none(),
        "inside the margin"
    );

    let (image, moved) = follow(&doc, &path, view, -1000, 0, false).expect("rebuilt");
    let shown = rect(1400, 400, 2200, 1000);
    assert!(
        contains(moved.region, shown),
        "covers the shape's reach now on screen"
    );
    assert_eq!(moved.level, view.level);
    assert_eq!(
        (image.width as i32 * 4, image.height as i32 * 4),
        (moved.region.width(), moved.region.height())
    );
}

// [mvp_exact_fallback]
#[test]
fn small_grouped_or_unviewed_moves_keep_the_exact_preview() {
    let doc = document();
    let visible = rect(0, 0, 800, 600);
    assert!(
        view_preview(&doc, &paths(), -1, visible, false).is_none(),
        "no view"
    );
    assert!(
        view_preview(&doc, &paths(), 0, rect(0, 0, 0, 0), false).is_none(),
        "an empty view"
    );
    let two = vec!["1".to_string(), "0".to_string()];
    assert!(
        view_preview(&doc, &two, 2, visible, false).is_none(),
        "a group"
    );

    let mut small = document();
    let mut layer = pixel_layer("small", 1000, 1000, OVER);
    layer.rect = rect(100, 100, 1100, 1100);
    small.layers = LayerStack::from_layers(vec![pixel_layer("under", W, H, UNDER), layer]);
    assert!(
        view_preview(&small, &paths(), 2, visible, false).is_none(),
        "a small layer bakes whole"
    );
}

// [mvp_exact_fallback]
#[test]
fn a_document_a_reduction_would_misdraw_keeps_the_exact_preview_above_level_zero() {
    let mut doc = document();
    let mut styled: Layer = pixel_layer("styled", 10, 10, UNDER);
    styled.extra_blocks.push(pictura_core::LayerBlock {
        key: *b"lfx2",
        data: vec![0u8; 4].into(),
    });
    let layers: Vec<Layer> = doc.layers.iter().cloned().chain([styled]).collect();
    doc.layers = LayerStack::from_layers(layers);
    let visible = rect(0, 0, 800, 600);
    assert!(view_preview(&doc, &paths(), 2, visible, false).is_none());
    assert!(view_preview(&doc, &paths(), 0, visible, false).is_some());
}

// [mvp_follow]
#[test]
fn a_drag_past_the_margin_rebuilds_the_moving_image() {
    let doc = document();
    let visible = rect(400, 400, 1200, 1000);
    let view = view_preview(&doc, &paths(), 2, visible, false)
        .expect("view preview")
        .view;
    assert!(
        follow(&doc, "1", view, 100, 80, false).is_none(),
        "inside the margin"
    );

    let (image, moved) = follow(&doc, "1", view, -500, 0, false).expect("rebuilt");
    let shown = rect(900, 400, 1700, 1000);
    assert!(
        contains(moved.region, shown),
        "covers the layer now on screen"
    );
    assert_eq!(moved.level, view.level);
    assert_eq!(
        (image.width as i32 * 4, image.height as i32 * 4),
        (moved.region.width(), moved.region.height())
    );
}

// [lct_move_preview_mask_view]
#[test]
fn a_view_preview_applies_the_layer_mask() {
    let mut doc = document();
    let (w, h) = (W as usize, H as usize);
    let mut mask = vec![255u8; w * h];
    for y in 0..h {
        for x in 0..w / 2 {
            mask[y * w + x] = 0;
        }
    }
    doc.layers[1].mask = Some(LayerMask {
        rect: rect(0, 0, W as i32, H as i32),
        default_color: 0,
        data: Some(mask.into()),
        ..Default::default()
    });

    let visible = rect(400, 400, 1200, 1000);
    let preview = view_preview(&doc, &paths(), 0, visible, false).expect("view preview");
    assert_eq!(preview.view.region, rect(200, 250, 1400, 1150));

    let hidden = preview.layer.pixel_color(100, 100);
    assert_eq!(hidden.alpha(), 0, "the masked half is transparent");
    assert_eq!(rgb(&preview.layer, 1100, 100), OVER);
    assert_eq!(
        preview.layer.pixel_color(1100, 100).alpha(),
        255,
        "the revealed half shows the layer"
    );
}

// [mvp_view_preview]
#[test]
fn a_layer_far_off_screen_starts_without_a_moving_image() {
    let mut doc = document();
    let layer = &mut doc.layers[1];
    layer.rect = rect(4000, 0, 4000 + W as i32, H as i32);
    let visible = rect(0, 0, 800, 600);
    let preview = view_preview(&doc, &paths(), 0, visible, false).expect("view preview");
    assert!(preview.base.is_null(), "nothing of the layer is on screen");
    assert!(preview.layer.is_null());
    assert!(follow(&doc, "1", preview.view, -4000, 0, false).is_some());
}

// [lct_move_preview_stack]
#[test]
fn above_patch_is_the_layers_above_the_group() {
    let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
    let mut top = pixel_layer("top", 10, 10, (255, 0, 0));
    top.rect = rect(5, 5, 15, 15);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("base", 40, 30, (30, 60, 90)),
        pixel_layer("moving", 20, 20, (0, 0, 255)),
        top,
    ]);
    let (image, origin) =
        above_patch(&doc, &["1".to_string()], 0, rect(0, 0, 40, 30), false).expect("above");
    assert_eq!(origin, (0, 0));
    assert_eq!((image.width(), image.height()), (40, 30));
    assert_eq!(rgb(&image, 10, 10), (255, 0, 0), "the top layer is drawn");
    assert_eq!(image.pixel_color(25, 25).alpha(), 0, "nothing below it");
}

// [lct_move_preview_top]
#[test]
fn topmost_layer_has_no_above_patch() {
    let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("base", 40, 30, (30, 60, 90)),
        pixel_layer("moving", 20, 20, (0, 0, 255)),
    ]);
    assert!(above_patch(&doc, &["1".to_string()], 0, rect(0, 0, 40, 30), false).is_none());
}
