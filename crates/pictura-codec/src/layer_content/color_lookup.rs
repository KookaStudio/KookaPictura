//! Color Lookup (`clrL`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring `selective_color.rs` and `color_balance.rs`. The block is a
//! `u16` version (`1`) followed by a version-16 descriptor; the descriptor
//! carries `lookupType`, `LUTFormat`, `dataOrder`/`tableOrder`, `Dthr`, and the
//! embedded LUT bytes in `LUT3DFileData`.
//!
//! ponytail: `dataOrder`/`tableOrder` are read as metadata only (a `.CUBE` is
//! self-describing), and an abstract-profile or device-link lookup decodes to a
//! no-op. Raise either when a real fixture disagrees.

use pictura_core::{Adjustment, ColorLookupKind, ColorLookupParams, Lut3d};
use crate::DescValue;
use crate::layer_content_budget::ContentAlloc;
use pictura_core::AdjustmentData;

use super::basic::{be_u16, desc_item};

/// `clrL`: a payload whose 2-byte version is not 1, whose descriptor does not
/// parse, or whose `lookupType` is absent is `None`, never a panic.
pub fn decode_color_lookup(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 1 {
        return None;
    }
    let obj = crate::read_descriptor(d.get(2..)?).ok()?;
    color_lookup_from_desc(&obj)
}

pub fn color_lookup_from_desc(obj: &DescValue) -> Option<Adjustment> {
    color_lookup_from_desc_using(obj, &mut ContentAlloc::new(None))
}

pub(crate) fn color_lookup_from_desc_using(obj: &DescValue, alloc: &mut ContentAlloc) -> Option<Adjustment> {
    let kind = match desc_item(obj, b"lookupType") {
        Some(DescValue::Enum { value, .. }) => decode_kind(value)?,
        _ => return None,
    };
    let lookup = if kind == ColorLookupKind::ThreeDLut {
        match desc_item(obj, b"LUTFormat") {
            Some(DescValue::Enum { value, .. }) if value.as_slice() == b"LUTFormatCUBE" => {
                tdta_payload(desc_item(obj, b"LUT3DFileData")?)
                    .and_then(|bytes| alloc.cube(bytes))
                    .map(|(size, points)| Lut3d { size, points })
            }
            _ => None,
        }
    } else {
        None
    };
    Some(Adjustment::ColorLookup(ColorLookupParams { kind, lookup }))
}

/// A `lookupType` enum value, or `None` when it is not one this engine knows.
pub(crate) fn decode_kind(value: &[u8]) -> Option<ColorLookupKind> {
    Some(match value {
        b"3DLUT" => ColorLookupKind::ThreeDLut,
        b"abstractProfile" => ColorLookupKind::AbstractProfile,
        b"deviceLinkProfile" => ColorLookupKind::DeviceLinkProfile,
        _ => return None,
    })
}

/// The payload of a `tdta` raw value: `[4-byte ostype][u32 length][bytes]`.
pub(crate) fn tdta_payload(value: &DescValue) -> Option<&[u8]> {
    let DescValue::Raw(bytes) = value else {
        return None;
    };
    if bytes.get(..4)? != b"tdta" {
        return None;
    }
    let len = u32::from_be_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    bytes.get(8..8 + len)
}

/// Build a `clrL` block: version 1, a `3DLUT` descriptor embedding `file_data`
/// as a `.CUBE`, `rgbOrder` data/table order, dither off. `decode_adjustment` on
/// the output parses the `.CUBE` back when it is valid.
pub fn encode_color_lookup(file_data: &[u8], name: &str) -> AdjustmentData {
    let mut tdta = b"tdta".to_vec();
    tdta.extend_from_slice(&(file_data.len() as u32).to_be_bytes());
    tdta.extend_from_slice(file_data);
    let desc = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (
                b"lookupType".to_vec(),
                DescValue::Enum {
                    kind: b"3DLUT".to_vec(),
                    value: b"3DLUT".to_vec(),
                },
            ),
            (b"Nm  ".to_vec(), DescValue::Text(name.to_string())),
            (b"Dthr".to_vec(), DescValue::Bool(false)),
            (
                b"LUTFormat".to_vec(),
                DescValue::Enum {
                    kind: b"LUTFormat".to_vec(),
                    value: b"LUTFormatCUBE".to_vec(),
                },
            ),
            (
                b"dataOrder".to_vec(),
                DescValue::Enum {
                    kind: b"LUTOrder".to_vec(),
                    value: b"rgbOrder".to_vec(),
                },
            ),
            (
                b"tableOrder".to_vec(),
                DescValue::Enum {
                    kind: b"LUTOrder".to_vec(),
                    value: b"rgbOrder".to_vec(),
                },
            ),
            (b"LUT3DFileName".to_vec(), DescValue::Text(name.to_string())),
            (b"LUT3DFileData".to_vec(), DescValue::Raw(tdta)),
        ],
    };
    let mut data = 1u16.to_be_bytes().to_vec();
    data.extend_from_slice(&crate::write_descriptor(&desc));
    AdjustmentData {
        key: *b"clrL",
        data: data.into(),
    }
}

/// A `LUT_3D_SIZE 2` identity `.cube`, so an authored Color Lookup layer is an
/// exact no-op.
pub fn identity_cube() -> Vec<u8> {
    let mut s = String::from("TITLE \"Identity\"\nLUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                s.push_str(&format!("{r}.0 {g}.0 {b}.0\n"));
            }
        }
    }
    s.into_bytes()
}

