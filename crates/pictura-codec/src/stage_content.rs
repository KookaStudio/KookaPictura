//! Internal import candidates and synchronized authoring edits.
use crate::{budget::DecodeBudget, PsdError};
use pictura_core::{AdjustmentData, FillContent, Layer, LayerBlock, LayerContent, LayerParts};
use std::sync::Arc;

pub(crate) fn admit_arc<T>(budget: &mut DecodeBudget, what: &str) -> Result<(), PsdError> {
    budget.charge(
        what,
        &[std::mem::size_of::<T>() + 2 * std::mem::size_of::<usize>()],
    )?;
    Ok(())
}

/// Import-only staging: primary definitions are resolved by the final pass.
pub(crate) fn stage_layer_from_parts(
    mut parts: LayerParts,
    budget: &mut DecodeBudget,
) -> Result<Layer, PsdError> {
    let content = if parts.is_group {
        LayerContent::Group(std::mem::take(&mut parts.children))
    } else if let Some(smart) = parts.smart_object.take() {
        admit_arc::<pictura_core::SmartObject>(budget, "smart object content")?;
        LayerContent::SmartObject(Arc::new(smart))
    } else if parts.type_tool.is_some() || parts.extra_block(b"TySh").is_some() {
        let tool = parts.type_tool.take();
        if tool.is_some() {
            admit_arc::<pictura_core::TypeTool>(budget, "text content")?;
        }
        LayerContent::Text(tool.map(Arc::new))
    } else {
        LayerContent::Pixel
    };
    if parts.type_tool.is_some() {
        admit_arc::<pictura_core::TypeTool>(budget, "fallback text content")?;
    }
    Ok(Layer::from_parts(parts, content))
}

pub(crate) fn edit_layer_staged<T>(
    layer: &mut Layer,
    budget: &mut DecodeBudget,
    edit: impl FnOnce(&mut LayerParts) -> Result<T, PsdError>,
) -> Result<T, PsdError> {
    let mut parts = LayerParts::from(std::mem::take(layer));
    let result = edit(&mut parts)?;
    *layer = stage_layer_from_parts(parts, budget)?;
    Ok(result)
}

/// Encode a fill for a layer's content payload. An opaque solid takes the
/// standard version-16 `SoCo` descriptor; any other alpha takes the 4-byte
/// `[r, g, b, a]` tuple, which the decoder, compositor, and writer all read.
/// Gradient and pattern fills delegate to their own encoders.
pub fn encoded_fill(fill: &FillContent) -> AdjustmentData {
    match fill {
        FillContent::Solid(color) if color[3] != 255 => AdjustmentData {
            key: *b"SoCo",
            data: color.to_vec().into(),
        },
        FillContent::Solid(color) => crate::encode_solid_color_fill([color[0], color[1], color[2]]),
        FillContent::Gradient(params) => crate::encode_gradient_fill_params(params),
        FillContent::Pattern(params) => crate::encode_pattern_fill(params),
    }
}

fn install_fill(parts: &mut LayerParts, fill: &FillContent) {
    let encoded = encoded_fill(fill);
    if parts.adjustment.is_some() || (encoded.key == *b"SoCo" && encoded.data.len() == 4) {
        parts.adjustment = Some(encoded);
    } else if let Some(block) = parts
        .extra_blocks
        .iter_mut()
        .find(|block| block.key == *b"vscg")
    {
        let mut data = encoded.key.to_vec();
        data.extend_from_slice(&encoded.data);
        block.data = data.into();
    } else {
        parts.adjustment = Some(encoded);
    }
}

pub(crate) fn parts_for_edit(layer: &Layer) -> LayerParts {
    let mut parts = LayerParts::from(layer.clone());
    // The dirty flag belongs to the installed value; materialize it before
    // extracting preservation so unrelated edits cannot resurrect old bytes.
    if layer.fill_was_edited() {
        if let Some(fill) = layer.fill_content() {
            install_fill(&mut parts, fill);
        }
    }
    parts
}

pub(crate) fn finish_parts(before: &Layer, mut parts: LayerParts) -> Layer {
    let original_text = before.type_tool().or_else(|| before.fallback_type_tool());
    if original_text != parts.type_tool.as_ref()
        && before.extra_block(b"TySh") == parts.extra_block(b"TySh")
    {
        match parts.type_tool.as_ref() {
            Some(tool) => {
                let data = crate::encode_type_tool(tool).into();
                if let Some(block) = parts.extra_blocks.iter_mut().find(|b| b.key == *b"TySh") {
                    block.data = data;
                } else {
                    parts.extra_blocks.push(LayerBlock {
                        key: *b"TySh",
                        data,
                    });
                }
            }
            None => parts.extra_blocks.retain(|b| b.key != *b"TySh"),
        }
    }
    let source_unchanged = before.adjustment() == parts.adjustment.as_ref()
        && before.extra_block(b"vscg") == parts.extra_block(b"vscg");
    if source_unchanged && !before.fill_was_edited() {
        let content = match before.content() {
            LayerContent::Adjustment(_) if !parts.is_group => Some(before.content().clone()),
            LayerContent::Fill(fill) | LayerContent::Shape(fill) if !parts.is_group => {
                let shape = parts
                    .extra_blocks
                    .iter()
                    .any(|b| matches!(&b.key, b"vmsk" | b"vsms"));
                Some(if shape {
                    LayerContent::Shape(fill.clone())
                } else {
                    LayerContent::Fill(fill.clone())
                })
            }
            _ => None,
        };
        if let Some(content) = content {
            return Layer::from_parts(parts, content);
        }
    }
    crate::layer_from_parts(parts)
}

/// Commit a fallible source edit only after its validation succeeds.
pub(crate) fn try_edit_layer<T>(
    layer: &mut Layer,
    edit: impl FnOnce(&mut LayerParts) -> Result<T, PsdError>,
) -> Result<T, PsdError> {
    let mut parts = parts_for_edit(layer);
    let result = edit(&mut parts)?;
    *layer = finish_parts(layer, parts);
    Ok(result)
}
