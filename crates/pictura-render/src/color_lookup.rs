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

use pictura_adjust::{Adjustment, ColorLookupKind, ColorLookupParams};
use pictura_codec::DescValue;
use pictura_core::AdjustmentData;

use crate::color_lookup_presets::{preset_cube, COLOR_LOOKUP_PRESETS};
use crate::composite::{be_u16, desc_item};

/// `clrL`: a payload whose 2-byte version is not 1, whose descriptor does not
/// parse, or whose `lookupType` is absent is `None`, never a panic.
pub(crate) fn decode_color_lookup(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 1 {
        return None;
    }
    let body = d.get(2..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let kind = match desc_item(&obj, b"lookupType") {
        Some(DescValue::Enum { value, .. }) => decode_kind(value)?,
        _ => return None,
    };
    let lookup = if kind == ColorLookupKind::ThreeDLut {
        match desc_item(&obj, b"LUTFormat") {
            Some(DescValue::Enum { value, .. }) if value.as_slice() == b"LUTFormatCUBE" => {
                tdta_payload(desc_item(&obj, b"LUT3DFileData")?)
                    .and_then(pictura_adjust::parse_cube)
            }
            _ => None,
        }
    } else {
        None
    };
    Some(Adjustment::ColorLookup(ColorLookupParams { kind, lookup }))
}

/// A `lookupType` enum value, or `None` when it is not one this engine knows.
fn decode_kind(value: &[u8]) -> Option<ColorLookupKind> {
    Some(match value {
        b"3DLUT" => ColorLookupKind::ThreeDLut,
        b"abstractProfile" => ColorLookupKind::AbstractProfile,
        b"deviceLinkProfile" => ColorLookupKind::DeviceLinkProfile,
        _ => return None,
    })
}

/// The payload of a `tdta` raw value: `[4-byte ostype][u32 length][bytes]`.
fn tdta_payload(value: &DescValue) -> Option<&[u8]> {
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
    data.extend_from_slice(&pictura_codec::write_descriptor(&desc));
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

/// The [`COLOR_LOOKUP_PRESETS`] index of `d`'s stored `Nm  ` name, or `None`
/// when `d` does not carry a preset name (a hand-authored file, or a block that
/// is not a Color Lookup).
pub fn color_lookup_preset_index(d: &AdjustmentData) -> Option<usize> {
    let obj = pictura_codec::read_descriptor(d.data.get(2..)?).ok()?;
    let name = match desc_item(&obj, b"Nm  ")? {
        DescValue::Text(s) => s.as_str(),
        _ => return None,
    };
    COLOR_LOOKUP_PRESETS.iter().position(|p| *p == name)
}

/// A Color Lookup block `d` with its lookup replaced by preset `index`'s
/// generated cube. `None` when `d` does not decode to a `3DLUT` Color Lookup or
/// `index` is out of range. An abstract-profile or device-link lookup is left
/// untouched rather than flattened into a 3-D LUT.
pub fn set_color_lookup_preset(d: &AdjustmentData, index: usize) -> Option<AdjustmentData> {
    match crate::decode_adjustment(d) {
        Some(Adjustment::ColorLookup(params)) if params.kind == ColorLookupKind::ThreeDLut => {}
        _ => return None,
    }
    let name = *COLOR_LOOKUP_PRESETS.get(index)?;
    let out = encode_color_lookup(&preset_cube(name)?, name);
    crate::decode_adjustment(&out).map(|_| out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(data: &[u8]) -> Option<ColorLookupParams> {
        match decode_color_lookup(data)? {
            Adjustment::ColorLookup(params) => Some(params),
            _ => panic!("clrL must decode to ColorLookup"),
        }
    }

    #[test]
    fn encoder_round_trips_to_a_parsed_cube() {
        let encoded = encode_color_lookup(&identity_cube(), "Identity");
        assert_eq!(encoded.key, *b"clrL");
        assert_eq!(&encoded.data[..2], &1u16.to_be_bytes());
        let params = decode(&encoded.data).expect("decodes");
        assert_eq!(params.kind, ColorLookupKind::ThreeDLut);
        let lut = params.lookup.expect("has a lookup");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.points.len(), 8);
    }

    #[test]
    fn rejects_bad_version_missing_key_and_truncation() {
        let mut encoded = encode_color_lookup(&identity_cube(), "x");
        encoded.data[1] = 2;
        assert_eq!(decode_color_lookup(&encoded.data), None);
        assert_eq!(decode_color_lookup(&[0, 1]), None);
        let mut truncated = encode_color_lookup(&identity_cube(), "x");
        truncated.data.truncate(3);
        assert_eq!(decode_color_lookup(&truncated.data), None);
    }

    #[test]
    fn non_cube_format_decodes_without_a_lookup() {
        // Flip LUTFormat to 3DL by rewriting the enum value bytes in place.
        let mut encoded = encode_color_lookup(&identity_cube(), "x");
        let needle = b"LUTFormatCUBE";
        let at = encoded
            .data
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("has the format enum");
        encoded.data[at..at + needle.len()].copy_from_slice(b"LUTFormat3DL\0");
        let params = decode(&encoded.data).expect("still decodes the block");
        assert_eq!(params.kind, ColorLookupKind::ThreeDLut);
        assert_eq!(params.lookup, None);
    }

    /// A minimal `clrL` block whose descriptor carries only `lookupType`.
    fn block_with_lookup_type(value: &[u8]) -> Vec<u8> {
        let desc = DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: vec![(
                b"lookupType".to_vec(),
                DescValue::Enum {
                    kind: value.to_vec(),
                    value: value.to_vec(),
                },
            )],
        };
        let mut data = 1u16.to_be_bytes().to_vec();
        data.extend_from_slice(&pictura_codec::write_descriptor(&desc));
        data
    }

    #[test]
    fn abstract_and_device_link_decode_without_a_lookup() {
        let abstract_profile =
            decode(&block_with_lookup_type(b"abstractProfile")).expect("decodes");
        assert_eq!(abstract_profile.kind, ColorLookupKind::AbstractProfile);
        assert_eq!(abstract_profile.lookup, None);
        let device_link = decode(&block_with_lookup_type(b"deviceLinkProfile")).expect("decodes");
        assert_eq!(device_link.kind, ColorLookupKind::DeviceLinkProfile);
        assert_eq!(device_link.lookup, None);
        assert_eq!(decode_color_lookup(&block_with_lookup_type(b"bogus")), None);
    }

    #[test]
    fn encoder_block_is_a_typed_descriptor() {
        let encoded = encode_color_lookup(&identity_cube(), "Identity.CUBE");
        let obj = pictura_codec::read_descriptor(&encoded.data[2..]).expect("descriptor parses");
        assert!(
            matches!(desc_item(&obj, b"lookupType"), Some(DescValue::Enum { value, .. })
                if value.as_slice() == b"3DLUT")
        );
        assert!(
            matches!(desc_item(&obj, b"Nm  "), Some(DescValue::Text(s)) if s == "Identity.CUBE")
        );
        match desc_item(&obj, b"LUT3DFileData") {
            Some(DescValue::Raw(bytes)) => {
                assert_eq!(&bytes[..4], b"tdta");
                assert_eq!(&bytes[8..], &identity_cube()[..]);
            }
            other => panic!("expected a tdta raw value, got {other:?}"),
        }
    }

    #[test]
    fn set_preset_rebuilds_and_rejects_bad_input() {
        let encoded = encode_color_lookup(&identity_cube(), "None");
        assert_eq!(color_lookup_preset_index(&encoded), Some(0));
        let changed = set_color_lookup_preset(&encoded, 1).expect("preset 1 applies");
        assert_eq!(color_lookup_preset_index(&changed), Some(1));
        assert_ne!(changed.data, encoded.data);
        assert!(set_color_lookup_preset(&encoded, COLOR_LOOKUP_PRESETS.len()).is_none());
        let mut other = encoded.clone();
        other.key = *b"levl";
        assert!(set_color_lookup_preset(&other, 0).is_none());
    }

    #[test]
    fn set_preset_refuses_non_three_d_lut_blocks() {
        for value in [
            b"abstractProfile".as_slice(),
            b"deviceLinkProfile".as_slice(),
        ] {
            let block = AdjustmentData {
                key: *b"clrL",
                data: block_with_lookup_type(value).into(),
            };
            assert!(
                set_color_lookup_preset(&block, 1).is_none(),
                "a {} block must be left unchanged",
                String::from_utf8_lossy(value)
            );
        }
        // The 3-D LUT path still rebuilds.
        let encoded = encode_color_lookup(&identity_cube(), "None");
        assert!(set_color_lookup_preset(&encoded, 1).is_some());
    }
}
