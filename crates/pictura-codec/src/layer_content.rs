//! PSD content parameters and their lossless raw encodings.

pub mod basic;
pub mod channel_mixer;
pub mod color_balance;
pub mod color_lookup;
pub mod curves;
pub mod defaults;
pub mod fill;
pub mod gradient_map;
pub mod selective_color;

use pictura_core::{Adjustment, AdjustmentData, FillContent};

use crate::budget::DecodeBudget;
use crate::layer_content_budget::{self, ContentAlloc};
use crate::{DescValue, PsdError};

pub use basic::{
    decode_solid_fill, encode_brightness_contrast, encode_hue_saturation, encode_invert,
    encode_photo_filter, encode_posterize, encode_solid_color_fill, encode_threshold,
};
pub use channel_mixer::{encode_channel_mixer, encode_channel_mixer_ink};
pub use color_balance::encode_color_balance;
pub use color_lookup::{encode_color_lookup, identity_cube};
pub use curves::encode_curves;
pub use defaults::{encode_shadows_highlights, Defaults};
pub use fill::{
    decode_gradient_fill, decode_pattern_fill, encode_gradient_fill, encode_gradient_fill_params,
    encode_pattern_fill, gradient_params_from_desc, pattern_params_from_desc,
    pattern_params_from_desc_with_link,
};
pub use gradient_map::{encode_gradient_map, gradient_map_dither};
pub use selective_color::encode_selective_color;

pub fn decode_adjustment(data: &AdjustmentData) -> Option<Adjustment> {
    let d = &data.data;
    match &data.key {
        b"nvrt" | b"invr" => Some(Adjustment::Invert),
        b"post" => basic::be_u16(d, 0)
            .filter(|v| (2..=255).contains(v))
            .map(|v| Adjustment::Posterize(v as u8)),
        b"thrs" => basic::be_u16(d, 0)
            .filter(|v| (1..=255).contains(v))
            .map(|v| Adjustment::Threshold(v as u8)),
        b"brit" => basic::decode_brightness_contrast(d),
        b"levl" => basic::decode_levels(d),
        b"hue2" | b"hue " => basic::decode_hue_saturation(d),
        b"expA" => basic::decode_exposure(d),
        b"phfl" => basic::decode_photo_filter(d),
        b"vibA" => basic::decode_vibrance(d),
        b"shdH" => basic::decode_shadows_highlights(d),
        b"blwh" => basic::decode_black_white(d),
        b"gdrm" | b"grdm" => gradient_map::decode_gradient_map(d),
        b"blnc" => color_balance::decode_color_balance(d),
        b"mixr" => channel_mixer::decode_channel_mixer(d),
        b"curv" => curves::decode_curves(d),
        b"selc" => selective_color::decode_selective_color(d),
        b"clrL" => color_lookup::decode_color_lookup(d),
        _ => None,
    }
}

pub fn decode_fill(data: &AdjustmentData) -> Option<FillContent> {
    match &data.key {
        b"SoCo" => match data.data.as_slice() {
            [r, g, b, a] => Some(FillContent::Solid([*r, *g, *b, *a])),
            d => basic::decode_solid_fill(d),
        },
        b"GdFl" => fill::decode_gradient_fill(&data.data),
        b"PtFl" => fill::decode_pattern_fill(&data.data),
        _ => None,
    }
}

fn vector_fill_key(obj: &DescValue) -> Option<&'static [u8; 4]> {
    if basic::desc_item(obj, b"Grad").is_some() {
        Some(b"GdFl")
    } else if basic::desc_item(obj, b"Ptrn").is_some() {
        Some(b"PtFl")
    } else if basic::desc_item(obj, b"Clr ").is_some() {
        Some(b"SoCo")
    } else {
        None
    }
}

pub fn decode_vector_fill(d: &[u8]) -> Option<FillContent> {
    let obj = crate::read_descriptor(d.get(4..)?).ok()?;
    decode_fill_from_desc(vector_fill_key(&obj)?, &obj)
}

pub fn decode_adjustment_from_desc(key: &[u8; 4], obj: &DescValue) -> Option<Adjustment> {
    match key {
        b"vibA" => basic::vibrance_from_desc(obj),
        b"blwh" => basic::black_white_from_desc(obj),
        b"clrL" => color_lookup::color_lookup_from_desc(obj),
        _ => None,
    }
}

pub fn decode_fill_from_desc(key: &[u8; 4], obj: &DescValue) -> Option<FillContent> {
    match key {
        b"SoCo" => basic::solid_fill_from_desc(obj),
        b"GdFl" => fill::gradient_params_from_desc(obj).map(FillContent::Gradient),
        b"PtFl" => fill::pattern_params_from_desc(obj).map(FillContent::Pattern),
        _ => None,
    }
}

pub(crate) fn decode_adjustment_within(
    data: &AdjustmentData,
    budget: &mut DecodeBudget,
) -> Result<Option<Adjustment>, PsdError> {
    let d = &data.data;
    match &data.key {
        b"vibA" | b"blwh" | b"clrL" => {
            let bytes: &[u8] = if data.key == *b"clrL" {
                if basic::be_u16(d, 0) != Some(1) {
                    return Ok(None);
                }
                let Some(bytes) = d.get(2..) else {
                    return Ok(None);
                };
                bytes
            } else {
                d
            };
            let Some(obj) = layer_content_budget::read_descriptor(bytes, budget)? else {
                return Ok(None);
            };
            decode_adjustment_from_desc_within(&data.key, &obj, budget)
        }
        b"hue2" | b"hue " | b"curv" | b"gdrm" | b"grdm" => {
            let mut alloc = ContentAlloc::new(Some(budget));
            let value = match &data.key {
                b"hue2" | b"hue " => basic::decode_hue_saturation_using(d, &mut alloc),
                b"curv" => curves::decode_curves_using(d, &mut alloc),
                _ => gradient_map::decode_gradient_map_using(d, &mut alloc),
            };
            alloc.finish(value)
        }
        _ => Ok(decode_adjustment(data)),
    }
}

pub(crate) fn decode_adjustment_from_desc_within(
    key: &[u8; 4],
    obj: &DescValue,
    budget: &mut DecodeBudget,
) -> Result<Option<Adjustment>, PsdError> {
    let mut alloc = ContentAlloc::new(Some(budget));
    let value = match key {
        b"vibA" => basic::vibrance_from_desc(obj),
        b"blwh" => basic::black_white_from_desc(obj),
        b"clrL" => color_lookup::color_lookup_from_desc_using(obj, &mut alloc),
        _ => None,
    };
    alloc.finish(value)
}

pub(crate) fn decode_fill_within(
    data: &AdjustmentData,
    budget: &mut DecodeBudget,
) -> Result<Option<FillContent>, PsdError> {
    if !matches!(&data.key, b"SoCo" | b"GdFl" | b"PtFl") {
        return Ok(None);
    }
    if data.key == *b"SoCo" {
        if let [r, g, b, a] = data.data.as_slice() {
            return Ok(Some(FillContent::Solid([*r, *g, *b, *a])));
        }
    }
    let Some(obj) = layer_content_budget::read_descriptor(&data.data, budget)? else {
        return Ok(None);
    };
    decode_fill_from_desc_within(&data.key, &obj, budget)
}

pub(crate) fn decode_fill_from_desc_within(
    key: &[u8; 4],
    obj: &DescValue,
    budget: &mut DecodeBudget,
) -> Result<Option<FillContent>, PsdError> {
    let mut alloc = ContentAlloc::new(Some(budget));
    let value = match key {
        b"SoCo" => basic::solid_fill_from_desc(obj),
        b"GdFl" => {
            fill::gradient_params_from_desc_using(obj, &mut alloc).map(FillContent::Gradient)
        }
        b"PtFl" => {
            fill::pattern_params_from_desc_using(obj, None, &mut alloc).map(FillContent::Pattern)
        }
        _ => None,
    };
    alloc.finish(value)
}

pub(crate) fn decode_vector_fill_within(
    d: &[u8],
    budget: &mut DecodeBudget,
) -> Result<Option<FillContent>, PsdError> {
    let Some(bytes) = d.get(4..) else {
        return Ok(None);
    };
    let Some(obj) = layer_content_budget::read_descriptor(bytes, budget)? else {
        return Ok(None);
    };
    let Some(key) = vector_fill_key(&obj) else {
        return Ok(None);
    };
    decode_fill_from_desc_within(key, &obj, budget)
}
