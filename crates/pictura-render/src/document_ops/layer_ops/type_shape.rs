//! `Type > Convert to Shape`: replace a live type layer with a shape layer in
//! place, keeping its name, stack position, raster mask, blending, and effects.

use pictura_core::{Document, LayerBlock, LayerParts, PsdRect, VectorFillRule};

use crate::type_layer::type_layer_spec;
use crate::type_outline::outline_paths;
use crate::{encode_solid_color_fill, resolve_path, resolve_path_mut};

/// Replace the type layer at `path` with a shape layer cut to its glyph
/// outlines. The glyph outline becomes the vector mask; the raster layer mask
/// and the layer's effects are retained. Returns false for a non-type layer,
/// an outline-less face, empty text, or a zero-dimension document.
pub fn convert_type_to_shape(doc: &mut Document, path: &str) -> bool {
    let Some(spec) = resolve_path(doc, path).and_then(type_layer_spec) else {
        return false;
    };
    let Some(paths) = outline_paths(&spec) else {
        return false;
    };
    if paths.is_empty() || doc.width == 0 || doc.height == 0 {
        return false;
    }
    let rgba = spec.rgba();
    let (width, height) = (doc.width, doc.height);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if !layer.is_type() {
        return false;
    }
    let mut parts = LayerParts::from(std::mem::take(layer));
    parts.adjustment = Some(encode_solid_color_fill([rgba[0], rgba[1], rgba[2]]));
    parts.type_tool = None;
    parts
        .extra_blocks
        .retain(|block| !matches!(&block.key, b"TySh" | b"vmsk" | b"vsms" | b"vmgm"));
    parts.applied_character_style = None;
    parts.applied_paragraph_style = None;
    parts.type_overrides = Default::default();
    parts.channels.clear();
    parts.raw_channels.clear();
    parts.source_channels = None;
    parts.pixel_data_irrelevant = true;
    parts.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: height as i32,
        right: width as i32,
    };
    let mask =
        pictura_codec::encode_vector_mask_with_rule(&paths, width, height, VectorFillRule::NonZero);
    parts.vector_mask = pictura_codec::decode_vector_mask(&mask, width, height);
    parts.vector_mask_hides_effects = false;
    parts.extra_blocks.push(LayerBlock {
        key: *b"vmsk",
        data: mask.into(),
    });
    *layer = pictura_codec::layer_from_parts(parts);
    true
}
