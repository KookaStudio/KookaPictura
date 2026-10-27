//! Content keys for the layer-source cache.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pictura_core::{Document, Layer, PsdRect, SmartObject, TypeTool};

use super::rect_key;

pub(super) fn content_key(layer: &Layer, doc: &Document) -> u64 {
    let mut hasher = DefaultHasher::new();
    rect_key(layer.rect).hash(&mut hasher);
    doc.width.hash(&mut hasher);
    doc.height.hash(&mut hasher);
    doc.mode.hash(&mut hasher);
    for channel in &layer.channels {
        channel.id.hash(&mut hasher);
        channel.data.ident().hash(&mut hasher);
    }
    if let Some(mask) = layer.mask.as_ref() {
        rect_key(mask.rect).hash(&mut hasher);
        mask.default_color.hash(&mut hasher);
        mask.disabled.hash(&mut hasher);
        mask.density.hash(&mut hasher);
        mask.feather_bits.hash(&mut hasher);
        mask.data
            .as_ref()
            .map(|data| data.ident())
            .hash(&mut hasher);
    }
    if let Some(mask) = layer.vector_mask.as_ref().filter(|mask| mask.has_fill()) {
        mask.ident().hash(&mut hasher);
        mask.invert.hash(&mut hasher);
        mask.disabled.hash(&mut hasher);
        mask.density.hash(&mut hasher);
        mask.feather_bits.hash(&mut hasher);
    }
    if let Some(adjustment) = layer.adjustment.as_ref() {
        adjustment.key.hash(&mut hasher);
        adjustment.data.hash(&mut hasher);
    }
    for block in &layer.extra_blocks {
        block.key.hash(&mut hasher);
        block.data.hash(&mut hasher);
    }
    if let Some(so) = layer.smart_object.as_ref() {
        so.payload_digest().hash(&mut hasher);
        chain_hash(so).hash(&mut hasher);
    }
    if let Some(type_tool) = layer.type_tool.as_ref() {
        text_key(type_tool, layer.rect).hash(&mut hasher);
    }
    text_state_hash().hash(&mut hasher);
    hasher.finish()
}

/// Global text rendering state that a composite of an embedded document can
/// read: the gamma curve, the font registry, and missing-glyph protection.
pub(super) fn text_state_hash() -> u64 {
    let mut hasher = DefaultHasher::new();
    crate::text_gamma::text_gamma()
        .map(f32::to_bits)
        .hash(&mut hasher);
    crate::fonts::generation().hash(&mut hasher);
    crate::fonts::missing_glyph_protection().hash(&mut hasher);
    hasher.finish()
}

pub(super) fn embedded_key(so: &SmartObject) -> u64 {
    let mut hasher = DefaultHasher::new();
    so.payload_digest().hash(&mut hasher);
    text_state_hash().hash(&mut hasher);
    hasher.finish()
}

pub(super) fn chain_hash(so: &SmartObject) -> u64 {
    let mut hasher = DefaultHasher::new();
    so.smart_filters_enabled.hash(&mut hasher);
    for filter in &so.smart_filters {
        filter.enabled.hash(&mut hasher);
        filter.filter_id.hash(&mut hasher);
        filter.options.hash(&mut hasher);
    }
    hasher.finish()
}

pub(super) fn text_key(type_tool: &TypeTool, rect: PsdRect) -> u64 {
    let mut hasher = DefaultHasher::new();
    // ponytail: Debug allocates the whole type model on every key; a structural
    // hash is the upgrade if text-heavy documents show up in a profile.
    format!("{type_tool:?}").hash(&mut hasher);
    rect_key(rect).hash(&mut hasher);
    crate::fonts::generation().hash(&mut hasher);
    crate::fonts::missing_glyph_protection().hash(&mut hasher);
    hasher.finish()
}

pub(super) fn effect_digest(layer: &Layer) -> u64 {
    let mut hasher = DefaultHasher::new();
    if let Some(block) = layer.extra_block(b"lfx2") {
        2u8.hash(&mut hasher);
        block.data.hash(&mut hasher);
    } else if let Some(block) = layer.extra_block(b"lrFX") {
        1u8.hash(&mut hasher);
        block.data.hash(&mut hasher);
    }
    hasher.finish()
}
