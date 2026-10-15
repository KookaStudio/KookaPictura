//! Map a document layer to its [`SvgContent`] for the SVG exporter.
//!
//! A shape layer (a `vmsk` outline plus a solid `vscg` fill, or an app-authored
//! adjustment fill) is emitted as a filled primitive; every other layer uses the
//! caller-supplied raster fallback. The actual SVG text is built by
//! `pictura_codec::svg`; this module only decodes the engine views the codec
//! cannot reach.

use pictura_codec::SvgContent;
use pictura_core::Layer;

/// The SVG content for a top-level `layer`. `raster` is the layer's fallback as
/// straight-alpha RGBA8888 at the document size (`width * height * 4` bytes), or
/// `None` when the caller did not composite it. A shape layer with a solid fill
/// ignores the raster; a gradient or pattern shape has no simple SVG fill and
/// falls back to it.
pub fn layer_content(
    layer: &Layer,
    width: u32,
    height: u32,
    raster: Option<Vec<u8>>,
) -> SvgContent {
    if let Some(fill) = crate::shape_fill_color(layer) {
        if let Some(subpaths) = crate::layer_shape_paths(layer, width, height) {
            return SvgContent::Shape {
                subpaths,
                fill,
                live: crate::layer_live_shape(layer),
            };
        }
    }
    match raster {
        Some(rgba) => SvgContent::Raster {
            rgba,
            width,
            height,
        },
        None => SvgContent::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    use pictura_core::{Document, Layer, LayerBlock};

    fn vscg_solid(rgb: [u8; 3]) -> Vec<u8> {
        let fill = crate::encode_solid_color_fill(rgb);
        let mut block = fill.key.to_vec();
        block.extend_from_slice(&fill.data);
        block
    }

    fn shape_layer(rgb: [u8; 3]) -> Layer {
        let outline = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (1.0, 1.0, 5.0, 5.0),
        )
        .unwrap();
        Layer {
            extra_blocks: vec![
                LayerBlock {
                    key: *b"vmsk",
                    data: pictura_codec::encode_vector_mask(std::slice::from_ref(&outline), 8, 8),
                },
                LayerBlock {
                    key: *b"vscg",
                    data: vscg_solid(rgb),
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn an_imported_shape_layer_becomes_a_filled_primitive() {
        let layer = shape_layer([10, 20, 30]);
        match layer_content(&layer, 8, 8, Some(vec![0; 8 * 8 * 4])) {
            SvgContent::Shape { fill, subpaths, .. } => {
                assert_eq!(fill, [10, 20, 30, 255]);
                assert!(!subpaths.is_empty());
            }
            other => panic!("expected a shape, got {other:?}"),
        }
    }

    #[test]
    fn a_pixel_layer_uses_its_raster_fallback() {
        let layer = Layer::default();
        match layer_content(&layer, 8, 8, Some(vec![7; 8 * 8 * 4])) {
            SvgContent::Raster {
                rgba,
                width,
                height,
            } => {
                assert_eq!((width, height), (8, 8));
                assert_eq!(rgba.len(), 8 * 8 * 4);
            }
            other => panic!("expected a raster, got {other:?}"),
        }
        assert!(matches!(
            layer_content(&Layer::default(), 8, 8, None),
            SvgContent::None
        ));
    }

    #[test]
    fn a_shape_without_a_raster_still_emits() {
        // The shape layer's own outline is enough; no raster is needed.
        let mut doc = Document::new(
            8,
            8,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers = vec![shape_layer([1, 2, 3])];
        assert!(matches!(
            layer_content(&doc.layers[0], 8, 8, None),
            SvgContent::Shape { .. }
        ));
    }
}
