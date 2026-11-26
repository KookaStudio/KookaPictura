//! Encoding for active content that has no preserved PSD definition.
use pictura_core::{Adjustment, AdjustmentData, Layer, LayerContent};

use crate::{layer_content, PsdError};

fn unsupported() -> PsdError {
    PsdError::Unsupported(
        "content has no exact supported PSD representation without its original data".into(),
    )
}

pub(crate) fn adjustment(layer: &Layer) -> Result<Option<AdjustmentData>, PsdError> {
    let LayerContent::Adjustment(value) = layer.content() else {
        return Ok(None);
    };
    if layer.adjustment().is_some() {
        return Ok(None);
    }
    let value = value.as_deref().ok_or_else(unsupported)?;
    let encoded = match value {
        Adjustment::Invert => crate::encode_invert(),
        Adjustment::Posterize(levels) => crate::encode_posterize(*levels),
        Adjustment::Threshold(level) => crate::encode_threshold(*level),
        Adjustment::BrightnessContrast(p) => {
            crate::encode_brightness_contrast(p.brightness, p.contrast)
        }
        Adjustment::HueSaturation(p) => {
            crate::encode_hue_saturation(p.hue, p.saturation, p.lightness)
        }
        Adjustment::PhotoFilter(p) => {
            crate::encode_photo_filter(p.color, p.density, p.preserve_luminosity)
        }
        Adjustment::ColorBalance(p) => {
            crate::encode_color_balance(p.shadows, p.midtones, p.highlights, p.preserve_luminosity)
        }
        Adjustment::ChannelMixer(p) => {
            let rgb = crate::encode_channel_mixer(p.monochrome, p.red, p.green, p.blue, p.constant);
            if layer_content::decode_adjustment(&rgb).as_ref() == Some(value) {
                rgb
            } else {
                crate::encode_channel_mixer_ink(p.ink, p.ink_constant)
            }
        }
        Adjustment::Curves(p) => crate::encode_curves(
            &p.points,
            p.red.as_deref(),
            p.green.as_deref(),
            p.blue.as_deref(),
        ),
        Adjustment::SelectiveColor(p) => crate::encode_selective_color(p.method, &p.ranges),
        Adjustment::GradientMap(p) => crate::encode_gradient_map(p, false),
        Adjustment::ShadowsHighlights(p) => {
            if p.local.is_some() {
                crate::encode_shadows_highlights_local(*p).ok_or_else(unsupported)?
            } else {
                crate::encode_shadows_highlights(p.shadows_amount as u16, p.highlights_amount as u16)
            }
        }
        Adjustment::Levels(_)
        | Adjustment::Exposure(_)
        | Adjustment::BlackWhite(_)
        | Adjustment::Vibrance(_)
        | Adjustment::ColorLookup(_)
        | Adjustment::Auto(_)
        | Adjustment::Desaturate
        | Adjustment::Equalize
        | Adjustment::ReplaceColor(_)
        | Adjustment::MatchColor(_) => return Err(unsupported()),
    };
    // Existing encoders may clamp values or cover only a subset of a model.
    if layer_content::decode_adjustment(&encoded).as_ref() != Some(value) {
        return Err(unsupported());
    }
    Ok(Some(encoded))
}

pub(crate) fn text(layer: &Layer) -> Result<Option<Vec<u8>>, PsdError> {
    let LayerContent::Text(value) = layer.content() else {
        return Ok(None);
    };
    if layer.extra_block(b"TySh").is_some() {
        return Ok(None);
    }
    let value = value.as_deref().ok_or_else(unsupported)?;
    let bytes = crate::encode_type_tool(value);
    let mut decoded = crate::type_tool::decode_type_tool(&bytes).ok_or_else(unsupported)?;
    // Text paths are serialized by the document-level Txt2 writer.
    decoded.path = value.path.clone();
    if decoded != *value {
        return Err(unsupported());
    }
    Ok(Some(bytes))
}

pub(crate) fn require_definition(layer: &Layer) -> Result<(), PsdError> {
    match layer.content() {
        LayerContent::Shape(_)
            if layer.extra_block(b"vmsk").is_none() && layer.extra_block(b"vsms").is_none() =>
        {
            Err(unsupported())
        }
        LayerContent::SmartObject(_)
            if !layer
                .extra_blocks()
                .iter()
                .any(|block| matches!(&block.key, b"SoLd" | b"SoLE" | b"plLd" | b"PlLd"))
                && crate::smart_writer::should_author(layer).is_none() =>
        {
            Err(unsupported())
        }
        _ => Ok(()),
    }
}
