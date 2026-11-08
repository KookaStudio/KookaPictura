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
    let (w, h) = (4usize, 128usize);
    let mut buffer = PixelBuffer::new(w as u32, h as u32, 4);
    buffer.data.fracture(w);
    let held = buffer.data.clone();
    buffer.data.write_band(1).expect("band")[0] = 200;
    let before = pictura_core::materializations();
    assert_eq!(sample_planar_argb(&buffer, 0, 64) >> 16 & 0xff, 200);
    assert_eq!(pictura_core::materializations(), before);
    drop(held);
}
