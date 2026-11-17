//! Copy, paste and clear a Layer Style: the effect blocks plus the Blending
//! Options, which CS6's Copy Layer Style carries with it.

use super::*;

/// A copied layer style: the effect blocks plus the Blending Options, which
/// CS6's Copy Layer Style carries with it.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerStyle {
    blocks: Vec<LayerBlock>,
    blend: BlendMode,
    opacity: u8,
    fill: u8,
    knockout: Knockout,
    blend_interior: bool,
    blend_clipping: bool,
    transparency_shapes: bool,
    layer_mask_hides_effects: bool,
    vector_mask_hides_effects: bool,
    restricted_channels: Vec<u32>,
}

/// Copy `layer`'s style, or `None` when it cannot carry one.
pub fn copy_layer_style(layer: &Layer) -> Option<LayerStyle> {
    styleable(layer).then(|| LayerStyle {
        blocks: layer
            .extra_blocks()
            .iter()
            .filter(|b| matches!(&b.key, b"lfx2" | b"lrFX"))
            .cloned()
            .collect(),
        blend: layer.blend,
        opacity: layer.opacity,
        fill: layer.fill,
        knockout: layer.knockout,
        blend_interior: layer.blend_interior,
        blend_clipping: layer.blend_clipping,
        transparency_shapes: layer.transparency_shapes,
        layer_mask_hides_effects: layer.layer_mask_hides_effects,
        vector_mask_hides_effects: layer.vector_mask_hides_effects,
        restricted_channels: layer.restricted_channels.clone(),
    })
}

/// Replace the style on every styleable layer at `paths` with `style`.
/// Returns how many layers changed.
pub fn paste_layer_style(doc: &mut Document, paths: &[&str], style: &LayerStyle) -> usize {
    edit_styles(doc, paths, |layer| {
        let before = layer.clone();
        pictura_codec::edit_layer(layer.as_mut(), |parts| {
            parts
                .extra_blocks
                .retain(|b| !matches!(&b.key, b"lfx2" | b"lrFX"));
        });
        pictura_codec::edit_layer(layer.as_mut(), |parts| {
            parts.extra_blocks.extend(style.blocks.iter().cloned());
        });
        layer.blend = style.blend;
        layer.opacity = style.opacity;
        layer.fill = style.fill;
        layer.knockout = style.knockout;
        layer.blend_interior = style.blend_interior;
        layer.blend_clipping = style.blend_clipping;
        layer.transparency_shapes = style.transparency_shapes;
        layer.layer_mask_hides_effects = style.layer_mask_hides_effects;
        layer.vector_mask_hides_effects = style.vector_mask_hides_effects;
        layer.restricted_channels = style.restricted_channels.clone();
        *layer != before
    })
}

/// Remove every effect from the layers at `paths`. Returns how many changed.
///
/// ponytail: the Blending Options are left as they are; whether CS6's Clear
/// Layer Style also resets them is unverified.
pub fn clear_layer_style(doc: &mut Document, paths: &[&str]) -> usize {
    edit_styles(doc, paths, |layer| {
        let before = layer.extra_blocks().len();
        pictura_codec::edit_layer(layer.as_mut(), |parts| {
            parts
                .extra_blocks
                .retain(|b| !matches!(&b.key, b"lfx2" | b"lrFX"));
        });
        layer.extra_blocks().len() != before
    })
}
