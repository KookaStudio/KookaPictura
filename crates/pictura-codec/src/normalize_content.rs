//! Resolve raw candidates once, retaining their unchanged-save representation.
use std::sync::Arc;
use pictura_core::{Layer, LayerContent, LayerParts};
use crate::{budget::DecodeBudget, PsdError};

pub fn layer_from_parts(mut parts: LayerParts) -> Layer {
    if parts.vector_mask.is_none() {
        parts.vector_mask = parts.extra_blocks.iter().find(|b| b.key == *b"vmsk")
            .or_else(|| parts.extra_blocks.iter().find(|b| b.key == *b"vsms"))
            .and_then(|b| crate::decode_vector_mask(&b.data, parts.rect.right.max(1) as u32, parts.rect.bottom.max(1) as u32));
    }
    layer_from_parts_within(parts, &mut DecodeBudget::default())
        .expect("authored layer content fits the default decode budget")
}

pub(crate) fn layer_from_parts_within(mut parts: LayerParts, budget: &mut DecodeBudget) -> Result<Layer, PsdError> {
    let content = if parts.is_group {
        LayerContent::Group(std::mem::take(&mut parts.children))
    } else if let Some(raw) = parts.adjustment.as_ref() {
        if let Some(fill) = crate::layer_content::decode_fill_within(raw, budget)? {
            fill_content(&parts, fill, budget)?
        } else {
            let adjustment = crate::layer_content::decode_adjustment_within(raw, budget)?;
            if adjustment.is_some() { crate::stage_content::admit_arc::<pictura_core::Adjustment>(budget, "adjustment content")?; }
            LayerContent::Adjustment(adjustment.map(Arc::new))
        }
    } else if let Some(fill) = match parts.extra_blocks.iter().find(|b| b.key == *b"vscg") {
        Some(raw) => crate::layer_content::decode_vector_fill_within(&raw.data, budget)?,
        None => None,
    } {
        fill_content(&parts, fill, budget)?
    } else if let Some(mut smart) = parts.smart_object.take() {
        let original_mode = crate::stack_marker::take_stack_marker_from_parts(&mut parts);
        if smart.stack_mode.is_none() { smart.stack_mode = original_mode; }
        crate::stage_content::admit_arc::<pictura_core::SmartObject>(budget, "smart object content")?;
        LayerContent::SmartObject(Arc::new(smart))
    } else if parts.type_tool.is_some() || parts.extra_blocks.iter().any(|b| b.key == *b"TySh") {
        if parts.type_tool.is_some() { crate::stage_content::admit_arc::<pictura_core::TypeTool>(budget, "text content")?; }
        LayerContent::Text(parts.type_tool.take().map(Arc::new))
    } else {
        LayerContent::Pixel
    };
    if parts.type_tool.is_some() { crate::stage_content::admit_arc::<pictura_core::TypeTool>(budget, "fallback text content")?; }
    Ok(Layer::from_parts(parts, content))
}
fn fill_content(parts: &LayerParts, fill: pictura_core::FillContent, budget: &mut DecodeBudget) -> Result<LayerContent, PsdError> {
    crate::stage_content::admit_arc::<pictura_core::FillContent>(budget, "fill content")?;
    Ok(
    if parts.extra_blocks.iter().any(|b| matches!(&b.key, b"vmsk" | b"vsms")) {
        LayerContent::Shape(Arc::new(fill))
    } else {
        LayerContent::Fill(Arc::new(fill))
    })
}

/// Stage source edits and install the resolved value with its source bytes.
pub fn edit_layer<T>(layer: &mut Layer, edit: impl FnOnce(&mut LayerParts) -> T) -> T {
    let mut parts = crate::stage_content::parts_for_edit(layer);
    let result = edit(&mut parts);
    let staged = crate::stage_content::finish_parts(layer, parts);
    *layer = staged;
    result
}

/// Finish every candidate, including hidden descendants, under the import budget.
pub(crate) fn normalize_tree_within(layers: &mut pictura_core::LayerStack, budget: &mut DecodeBudget) -> Result<(), PsdError> {
    for layer in layers.iter_mut() {
        if let Some(children) = layer.children_mut() { normalize_tree_within(children, budget)?; }
        let parts = LayerParts::from(std::mem::take(layer));
        *layer = layer_from_parts_within(parts, budget)?;
    }
    Ok(())
}

/// Retire every definition that could become active after a pixel bake.
/// Masks and appearance metadata are retained by the command that owns them.
pub fn retire_layer_content(layer: &mut Layer) {
    let mut parts = LayerParts::from(std::mem::take(layer));
    parts.adjustment = None;
    parts.smart_object = None;
    parts.type_tool = None;
    parts.extra_blocks.retain(|block| !matches!(&block.key,
        b"TySh" | b"SoLd" | b"SoLE" | b"PlLd" | b"plLd" | b"vscg" |
        b"nvrt" | b"invr" | b"post" | b"thrs" | b"brit" | b"levl" |
        b"hue2" | b"hue " | b"expA" | b"phfl" | b"vibA" | b"shdH" |
        b"blwh" | b"gdrm" | b"grdm" | b"blnc" | b"mixr" | b"curv" |
        b"selc" | b"clrL" | b"SoCo" | b"GdFl" | b"PtFl"));
    *layer = Layer::from_parts(parts, LayerContent::Pixel);
}
