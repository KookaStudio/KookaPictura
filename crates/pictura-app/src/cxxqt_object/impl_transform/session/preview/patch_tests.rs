//! The Move and transform backdrop patch over the hidden layers' influence
//! (#1274).

use super::*;
use crate::cxxqt_object::helpers_composite::crop_planar;
use crate::cxxqt_object::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, Knockout, LayerStack};

fn at(mut layer: pictura_core::Layer, left: i32, top: i32) -> pictura_core::Layer {
    let (w, h) = (layer.rect.width(), layer.rect.height());
    layer.rect = PsdRect {
        top,
        left,
        bottom: top + h,
        right: left + w,
    };
    layer
}

/// A canvas-wide base, a drop-shadowed layer, and a group whose child is
/// shadowed too.
fn styled_doc() -> Document {
    let mut doc = Document::new(301, 211, ColorMode::Rgb, BitDepth::Eight);
    let base = pixel_layer("base", 301, 211, (40, 90, 160));
    let mut styled = at(pixel_layer("styled", 30, 20, (200, 10, 10)), 40, 50);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.size", 6.0);
    let mut child = at(pixel_layer("child", 12, 9, (20, 200, 20)), 150, 120);
    pictura_render::set_layer_style_value(&mut child, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut child, "dropShadow.size", 4.0);
    let mut group = at(pixel_layer("group", 12, 9, (0, 0, 0)), 150, 120);
    group.set_group_children(pictura_core::LayerStack::default());
    group.channels_mut().clear();
    group.set_group_children(LayerStack::from_layers(vec![child]));
    doc.layers = LayerStack::from_layers(vec![base, styled, group]);
    doc
}

/// A drop-shadowed layer and a group patch their influence, and the patch is
/// that crop of the whole composite with them hidden; a knockout keeps the
/// whole backdrop.
#[test]
fn move_backdrop_patches_the_influence() {
    let doc = styled_doc();
    for path in ["1", "2"] {
        let paths = vec![path.to_string()];
        let influence = pictura_render::damage::influence(&doc, path).expect("bounded");
        let rect = backdrop_patch_rect(&doc, &paths).expect("a patch");
        assert_eq!(rect, influence, "{path}");
        assert!(
            rect.width() < doc.width as i32,
            "{path} is smaller than the canvas"
        );
        let hidden_doc = hidden(&doc, &paths);
        let (patch, _) = pictura_render::composite(pictura_render::CompositeRequest::region(
            &hidden_doc,
            rect,
            false,
        ));
        let whole = pictura_render::composite_rgba(&hidden_doc);
        let want = crop_planar(
            &whole,
            rect.left,
            rect.top,
            rect.width() as u32,
            rect.height() as u32,
        );
        assert!(patch == want, "{path}");
    }
    let mut knocked = styled_doc();
    let mut layers: Vec<pictura_core::Layer> = knocked.layers.iter().cloned().collect();
    layers[1].knockout = Knockout::Shallow;
    knocked.layers = LayerStack::from_layers(layers);
    assert_eq!(backdrop_patch_rect(&knocked, &["1".to_string()]), None);
}
