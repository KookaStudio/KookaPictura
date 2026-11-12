use super::preview::{build_transform_preview, rendered_layer_image};
use super::*;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{
    BitDepth, ColorMode, Document, Layer, LayerBlock, LayerMask, PixelBuffer, PsdRect,
};

const RECT: PsdRect = PsdRect {
    top: 8,
    left: 10,
    bottom: 17,
    right: 22,
};

fn small_layer() -> Layer {
    let mut layer = pixel_layer("top", 12, 9, (200, 100, 50));
    layer.rect = RECT;
    layer
}

fn document(top: Layer) -> Document {
    let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 40, 30, (30, 60, 90)), top]);
    doc
}

fn pixels(image: &QImage, x: i32, y: i32, w: i32, h: i32) -> Vec<(i32, i32, i32, i32)> {
    (y..y + h)
        .flat_map(|py| (x..x + w).map(move |px| (px, py)))
        .map(|(px, py)| {
            let c = image.pixel_color(px, py);
            (c.red(), c.green(), c.blue(), c.alpha())
        })
        .collect()
}

// [tpr_layer_region]
#[test]
fn rendered_layer_preview_equals_the_cropped_full_render() {
    let mut top = small_layer();
    top.mask = Some(LayerMask {
        rect: RECT,
        default_color: 0,
        data: Some(vec![128; 12 * 9].into()),
        ..Default::default()
    });
    let doc = document(top);
    let (image, x, y) = rendered_layer_image(&doc, "1", false).expect("preview");
    assert_eq!((x, y), (10, 8));
    assert_eq!((image.width(), image.height()), (12, 9));

    let mut scratch = doc.clone();
    let source = doc.layers[1].clone();
    scratch.layers = LayerStack::from_layers(vec![source]);
    let rendered = current_buffer(&scratch, false);
    let full = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    assert_eq!(pixels(&image, 0, 0, 12, 9), pixels(&full, 10, 8, 12, 9));
}

// [tpr_patch_exact]
#[test]
fn a_small_layer_gets_a_backdrop_patch() {
    let doc = document(small_layer());
    let preview =
        build_transform_preview(&doc, "1", &["1".to_string()], RECT, false, true).expect("preview");
    assert_eq!(preview.base_origin, Some((10, 8)));
    assert_eq!((preview.base.width(), preview.base.height()), (12, 9));

    let mut hidden = doc.clone();
    hidden.layers[1].visible = false;
    let rendered = current_buffer(&hidden, false);
    let full = buffer_to_image(&pictura_codec::buffer_to_srgb(&doc, &rendered));
    assert_eq!(
        pixels(&preview.base, 0, 0, 12, 9),
        pixels(&full, 10, 8, 12, 9)
    );
}

// [tpr_full_backdrop]
#[test]
fn effects_and_lifts_keep_the_whole_backdrop() {
    let mut effected = small_layer();
    effected.extra_blocks.push(LayerBlock {
        key: *b"lfx2",
        data: Vec::new().into(),
    });
    let styled = build_transform_preview(
        &document(effected),
        "1",
        &["1".to_string()],
        RECT,
        false,
        true,
    )
    .expect("preview");
    assert_eq!(styled.base_origin, None);
    assert_eq!((styled.base.width(), styled.base.height()), (40, 30));

    let lifted = build_transform_preview(
        &document(small_layer()),
        "1",
        &["1".to_string()],
        RECT,
        false,
        false,
    )
    .expect("lift");
    assert_eq!(lifted.base_origin, None);
    assert_eq!((lifted.base.width(), lifted.base.height()), (40, 30));
}

// [tpr_sample_banded]
#[test]
fn sample_planar_argb_reads_a_diverged_composite_without_materializing() {
    let (w, h) = (4usize, 300usize);
    let mut buffer = PixelBuffer::new(w as u32, h as u32, 4);
    buffer.data.tile(w, h);
    let held = buffer.data.clone();
    buffer.data.write_rect_from(0..1, 256..257, w, &[200]);
    let before = pictura_core::materializations();
    assert_eq!(sample_planar_argb(&buffer, 0, 256) >> 16 & 0xff, 200);
    assert_eq!(pictura_core::materializations(), before);
    drop(held);
}

/// A 100² document with a plain 60 × 40 layer at (3, 5) over its base.
fn large_layer_document(top: Layer) -> Document {
    let mut doc = Document::new(100, 100, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 100, 100, (30, 60, 90)), top]);
    doc
}

const LARGE: PsdRect = PsdRect {
    top: 5,
    left: 3,
    bottom: 45,
    right: 63,
};

fn large_layer() -> Layer {
    let mut layer = pixel_layer("top", 60, 40, (200, 100, 50));
    layer.rect = LARGE;
    layer
}

fn path() -> Vec<String> {
    vec!["1".to_string()]
}

// [tpm_reduced]
#[test]
fn a_large_plain_layer_previews_at_a_reduced_level() {
    set_transform_preview_pixels_for_test(1000);
    let doc = large_layer_document(large_layer());
    let preview = build_transform_preview(&doc, "1", &path(), LARGE, false, true).expect("preview");
    set_transform_preview_pixels_for_test(-1);
    assert_eq!(preview.level, 1);
    // The layer aligned inside itself to the 2-px grid: 58 × 38 at (4, 6).
    assert_eq!((preview.x, preview.y), (4, 6));
    assert_eq!((preview.image.width(), preview.image.height()), (29, 19));
    assert_eq!(
        pixels(&preview.image, 14, 9, 1, 1),
        vec![(200, 100, 50, 255)]
    );
    // The backdrop patch aligned outward: 62 × 42 at (2, 4).
    assert_eq!(preview.base_origin, Some((2, 4)));
    assert_eq!((preview.base.width(), preview.base.height()), (31, 21));
    assert_eq!(pixels(&preview.base, 15, 10, 1, 1), vec![(30, 60, 90, 255)]);
}

// [tpm_small_exact]
#[test]
fn a_small_or_styled_layer_previews_at_full_size() {
    set_transform_preview_pixels_for_test(2400);
    let doc = large_layer_document(large_layer());
    let at = build_transform_preview(&doc, "1", &path(), LARGE, false, true).expect("preview");
    assert_eq!(at.level, 0);
    assert_eq!((at.x, at.y), (3, 5));
    assert_eq!((at.image.width(), at.image.height()), (60, 40));

    set_transform_preview_pixels_for_test(1000);
    let mut masked = large_layer();
    masked.mask = Some(LayerMask {
        rect: LARGE,
        default_color: 0,
        data: Some(vec![128; 60 * 40].into()),
        ..Default::default()
    });
    let doc = large_layer_document(masked);
    let styled = build_transform_preview(&doc, "1", &path(), LARGE, false, true).expect("preview");
    set_transform_preview_pixels_for_test(-1);
    assert_eq!(styled.level, 0);
    assert_eq!((styled.image.width(), styled.image.height()), (60, 40));
}

// [tpm_bytes]
#[test]
fn preview_bytes_follow_the_level() {
    set_transform_preview_pixels_for_test(1000);
    let doc = large_layer_document(large_layer());
    assert_eq!(
        super::preview::transform_preview_bytes(&doc, "1", &path(), LARGE, true),
        20 * 600
    );
    let mut square = pixel_layer("top", 30, 30, (200, 100, 50));
    square.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 30,
        right: 30,
    };
    let rect = square.rect;
    let doc_square = large_layer_document(square);
    assert_eq!(
        super::preview::transform_preview_bytes(&doc_square, "1", &path(), rect, true),
        20 * 900
    );
    // A lifted selection previews at full size over the whole backdrop.
    assert_eq!(
        super::preview::transform_preview_bytes(&doc, "1", &path(), LARGE, false),
        20 * 2400 + 12 * 100 * 100
    );
    set_transform_preview_pixels_for_test(-1);
}
