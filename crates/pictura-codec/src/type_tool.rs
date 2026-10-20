//! Decode a layer's preserved `TySh` block into the derived
//! [`pictura_core::TypeTool`] view.
//!
//! The raw block stays in `Layer.extra_blocks` and is the source of truth for
//! re-emission; this module only parses the framing. The text and warp
//! descriptors are kept as their full version-16 bytes so an edited view can
//! re-encode framing-only (EngineData stays opaque inside those bytes). A
//! malformed block leaves the view unset rather than failing the document.

use pictura_core::{AntiAlias, LayerStack, TextStyle, TypeTool};

use crate::common::Reader;
use crate::descriptor::{self, get_object_item, DescValue};
use crate::engine_data::{extract_fonts, extract_style, parse_engine_data};

/// Resolve `TySh` for `layers` (and their children). Never fails: an absent or
/// unparseable block leaves `type_tool` unset.
pub(crate) fn resolve_type_tools(layers: &mut LayerStack) {
    for layer in layers.iter_mut() {
        layer.type_tool = layer
            .extra_block(b"TySh")
            .and_then(|block| decode_type_tool(&block.data));
        resolve_type_tools(&mut layer.children);
    }
}

/// Parse the `TySh` payload: version-1 framing, transform, text and warp
/// descriptors, then four bounds. `ponytail:` bounds are `i32` per psd-tools;
/// The reference's table says "4 * 8" — switch if a real CS6 file disagrees.
pub(crate) fn decode_type_tool(data: &[u8]) -> Option<TypeTool> {
    let mut r = Reader::new(data);
    if r.u16().ok()? != 1 {
        return None;
    }
    let mut transform = [0f64; 6];
    for slot in &mut transform {
        *slot = f64::from_bits(r.u64().ok()?);
    }
    if r.u16().ok()? != 50 {
        return None;
    }
    let text_start = r.pos;
    let text_value = descriptor::read_descriptor(&mut r).ok()?;
    let text_desc = data.get(text_start..r.pos)?.to_vec();
    if r.u16().ok()? != 1 {
        return None;
    }
    let warp_start = r.pos;
    descriptor::read_descriptor(&mut r).ok()?;
    let warp_desc = data.get(warp_start..r.pos)?.to_vec();
    let bounds = [r.i32().ok()?, r.i32().ok()?, r.i32().ok()?, r.i32().ok()?];
    let (fonts, style, box_bounds) = engine_data_view(&text_value);
    Some(TypeTool {
        transform,
        text: extract_text(&text_value),
        bounds,
        text_desc,
        warp_desc,
        fonts,
        style,
        vertical: is_vertical(&text_value),
        box_bounds,
    })
}

/// The text descriptor's `Ornt` enum is `Vrtc` for vertical type.
fn is_vertical(value: &DescValue) -> bool {
    let DescValue::Object { items, .. } = value else {
        return false;
    };
    matches!(get_object_item(items, b"Ornt"), Some(DescValue::Enum { value, .. }) if value == b"Vrtc")
}

/// Best-effort view of the `Txt ` descriptor's opaque `EngineData` blob; a
/// missing or malformed blob leaves the style unset rather than failing. The
/// anti-aliasing method comes from the descriptor's `AntA` enum when present,
/// overriding the EngineData `/AntiAlias` flag. The third value is paragraph
/// type's box, `None` for point type.
fn engine_data_view(text: &DescValue) -> (Vec<String>, Option<TextStyle>, Option<[f64; 4]>) {
    let DescValue::Object { items, .. } = text else {
        return (Vec::new(), None, None);
    };
    let Some(DescValue::Raw(raw)) = get_object_item(items, b"EngineData") else {
        return (Vec::new(), None, None);
    };
    let Ok(root) = parse_engine_data(engine_data_payload(raw)) else {
        return (Vec::new(), None, None);
    };
    let fonts = extract_fonts(&root);
    let mut style = extract_style(&root, &fonts);
    if let Some(anti_alias) = descriptor_ant_alias(items) {
        style.character.anti_alias = anti_alias;
    }
    (fonts, Some(style), crate::type_shape::extract_box(&root))
}

/// The `AntA` enum value as an [`AntiAlias`], if the descriptor carries one.
fn descriptor_ant_alias(items: &[(Vec<u8>, DescValue)]) -> Option<AntiAlias> {
    match get_object_item(items, b"AntA") {
        Some(DescValue::Enum { value, .. }) => ant_alias_value(value),
        _ => None,
    }
}

/// The CS6 `AntA` spelling of an anti-aliasing method.
pub(crate) fn ant_alias_spelling(method: AntiAlias) -> &'static str {
    match method {
        AntiAlias::None => "antiAliasNone",
        AntiAlias::Sharp => "antiAliasSharp",
        AntiAlias::Crisp => "antiAliasCrisp",
        AntiAlias::Strong => "antiAliasStrong",
        AntiAlias::Smooth => "antiAliasSmooth",
    }
}

/// The method an `AntA` enum value names, `None` for an unknown spelling.
pub(crate) fn ant_alias_value(value: &[u8]) -> Option<AntiAlias> {
    // ponytail: Crisp/Strong/Smooth spellings are inferred; validated against a
    // CS6-authored PSD in the type-style-model follow-up.
    match value {
        b"antiAliasNone" => Some(AntiAlias::None),
        b"antiAliasSharp" => Some(AntiAlias::Sharp),
        b"antiAliasCrisp" => Some(AntiAlias::Crisp),
        b"antiAliasStrong" => Some(AntiAlias::Strong),
        b"antiAliasSmooth" => Some(AntiAlias::Smooth),
        _ => None,
    }
}

/// `tdta` raw values are `ostype + u32 length + bytes`; other shapes pass
/// through so the parser can reject them.
pub(crate) fn engine_data_payload(raw: &[u8]) -> &[u8] {
    if raw.len() >= 8 && &raw[..4] == b"tdta" {
        let len = u32::from_be_bytes(raw[4..8].try_into().unwrap()) as usize;
        let end = (8 + len).min(raw.len());
        &raw[8..end]
    } else {
        raw
    }
}

/// Rebuild the `TySh` payload from a view: framing plus the preserved
/// descriptor bytes. Block padding is the tagged-block writer's job.
pub fn encode_type_tool(tool: &TypeTool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1u16.to_be_bytes());
    for value in tool.transform {
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }
    out.extend_from_slice(&50u16.to_be_bytes());
    out.extend_from_slice(&tool.text_desc);
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&tool.warp_desc);
    for value in tool.bounds {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn extract_text(value: &DescValue) -> String {
    let DescValue::Object { items, .. } = value else {
        return String::new();
    };
    match get_object_item(items, b"Txt ") {
        // Photoshop null-terminates `Txt `.
        Some(DescValue::Text(text)) => text.trim_end_matches('\0').to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::write_descriptor;
    use pictura_core::Layer;

    const TEXT: &str = "Hi\rThere";

    fn text_desc() -> Vec<u8> {
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"TxtX".to_vec(),
            items: vec![(b"Txt ".to_vec(), DescValue::Text(TEXT.into()))],
        })
    }

    fn empty_desc() -> Vec<u8> {
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: Vec::new(),
        })
    }

    fn tysh_from(text_desc: Vec<u8>) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_be_bytes());
        for value in [1.0f64, 0.0, 0.0, 1.0, 10.0, 20.0] {
            data.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        data.extend_from_slice(&50u16.to_be_bytes());
        data.extend_from_slice(&text_desc);
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&empty_desc());
        for value in [0i32, 0, 100, 50] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data
    }

    fn synthetic_tysh() -> Vec<u8> {
        tysh_from(text_desc())
    }

    /// A `Txt ` descriptor whose `EngineData` is a `tdta` raw blob with a
    /// first run that omits `Font`, so the default font index 1 resolves.
    fn engine_data_desc() -> Vec<u8> {
        let payload = b"<< /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << \
            /StyleSheetData << /FontSize 150.0 /FillColor << /Values [ 1.0 1.0 1.0 1.0 ] >> \
            >> >> >> ] >> /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << \
            /Justification 0 >> >> >> ] >> >> /ResourceDict << \
            /FontSet [ << /Name (AdobeInvisFont) >> << /Name (MyriadPro-Regular) >> ] \
            /StyleSheetSet [ << /StyleSheetData << /Font 1 /FontSize 12.0 >> >> ] >> >>";
        let mut raw = b"tdta".to_vec();
        raw.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        raw.extend_from_slice(payload);
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"TxtX".to_vec(),
            items: vec![
                (b"Txt ".to_vec(), DescValue::Text(TEXT.into())),
                (b"EngineData".to_vec(), DescValue::Raw(raw)),
            ],
        })
    }

    /// A `Txt ` descriptor carrying `engine` as a `tdta` EngineData blob and an
    /// optional `AntA` enum value.
    fn text_desc_with(ant: Option<&str>, engine: &[u8]) -> Vec<u8> {
        let mut raw = b"tdta".to_vec();
        raw.extend_from_slice(&(engine.len() as u32).to_be_bytes());
        raw.extend_from_slice(engine);
        let mut items = vec![(b"Txt ".to_vec(), DescValue::Text(TEXT.into()))];
        if let Some(ant) = ant {
            items.push((
                b"AntA".to_vec(),
                DescValue::Enum {
                    kind: b"Annt".to_vec(),
                    value: ant.as_bytes().to_vec(),
                },
            ));
        }
        items.push((b"EngineData".to_vec(), DescValue::Raw(raw)));
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"TxtX".to_vec(),
            items,
        })
    }

    const ENGINE: &[u8] = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00a\x00\r) >> \
        /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /FontSize 12.0 >> >> >> ] >> \
        /AntiAlias 0 >> /ResourceDict << /FontSet [] >> >>";

    #[test]
    fn descriptor_ant_alias_is_reported_over_the_flag() {
        let desc = text_desc_with(Some("antiAliasCrisp"), ENGINE);
        let tool = decode_type_tool(&tysh_from(desc)).expect("decodes");
        assert_eq!(
            tool.style.expect("style").character.anti_alias,
            AntiAlias::Crisp
        );
    }

    #[test]
    fn engine_data_flag_is_the_anti_alias_fallback() {
        let desc = text_desc_with(None, ENGINE);
        let tool = decode_type_tool(&tysh_from(desc)).expect("decodes");
        assert_eq!(
            tool.style.expect("style").character.anti_alias,
            AntiAlias::None
        );
    }

    #[test]
    fn malformed_engine_data_leaves_the_style_unset() {
        let desc = text_desc_with(None, b"<< /EngineDict /unterminated");
        let tool = decode_type_tool(&tysh_from(desc)).expect("decodes");
        assert_eq!(tool.text, TEXT);
        assert!(tool.style.is_none());
    }

    #[test]
    fn type_tool_decodes_transform_text_bounds() {
        let tool = decode_type_tool(&synthetic_tysh()).expect("decodes");
        assert_eq!(tool.transform, [1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        assert_eq!(tool.text, TEXT);
        assert_eq!(tool.bounds, [0, 0, 100, 50]);
        assert_eq!(tool.text_desc, text_desc());
        assert_eq!(tool.warp_desc, empty_desc());
        assert!(tool.fonts.is_empty());
        assert!(tool.style.is_none(), "no EngineData leaves the style unset");
    }

    #[test]
    fn type_tool_reads_engine_data_style() {
        let tool = decode_type_tool(&tysh_from(engine_data_desc())).expect("decodes");
        assert_eq!(tool.fonts, ["AdobeInvisFont", "MyriadPro-Regular"]);
        let style = tool.style.expect("style decodes");
        assert_eq!(style.font.as_deref(), Some("MyriadPro-Regular"));
        assert_eq!(style.character.size, 150.0);
        assert_eq!(style.character.fill_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(style.paragraph.justify, pictura_core::Justify::Left);
    }

    #[test]
    fn type_tool_encode_decode_roundtrip() {
        let original = decode_type_tool(&synthetic_tysh()).expect("decodes");
        let reencoded = encode_type_tool(&original);
        assert_eq!(&reencoded[..2], &1u16.to_be_bytes(), "version 1");
        let text_version = u16::from_be_bytes(reencoded[50..52].try_into().unwrap());
        assert_eq!(text_version, 50, "u16 after the transform is 50");
        let back = decode_type_tool(&reencoded).expect("re-decodes");
        assert_eq!(back.transform, original.transform);
        assert_eq!(back.text, original.text);
        assert_eq!(back.bounds, original.bounds);
        assert_eq!(back.text_desc, original.text_desc);
        assert_eq!(back.warp_desc, original.warp_desc);
    }

    #[test]
    fn type_tool_missing_txt_yields_empty_text() {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_be_bytes());
        for value in [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0] {
            data.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        data.extend_from_slice(&50u16.to_be_bytes());
        data.extend_from_slice(&empty_desc());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&empty_desc());
        for value in [1i32, 2, 3, 4] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        let tool = decode_type_tool(&data).expect("decodes");
        assert_eq!(tool.text, "");
        assert_eq!(tool.bounds, [1, 2, 3, 4]);
    }

    #[test]
    fn type_tool_malformed_is_none() {
        assert!(decode_type_tool(&[]).is_none(), "empty");
        assert!(
            decode_type_tool(&synthetic_tysh()[..20]).is_none(),
            "truncated"
        );

        let mut bad_version = synthetic_tysh();
        bad_version[..2].copy_from_slice(&2u16.to_be_bytes());
        assert!(decode_type_tool(&bad_version).is_none(), "version != 1");

        let mut bytes = synthetic_tysh();
        let text_version_at = 2 + 6 * 8;
        bytes[text_version_at..text_version_at + 2].copy_from_slice(&49u16.to_be_bytes());
        assert!(decode_type_tool(&bytes).is_none(), "text version != 50");

        let mut layers = LayerStack::from_layers(vec![Layer {
            extra_blocks: vec![pictura_core::LayerBlock {
                key: *b"TySh",
                data: bad_version,
            }],
            ..Default::default()
        }]);
        resolve_type_tools(&mut layers);
        assert!(layers[0].type_tool.is_none(), "view stays unset");
        assert!(layers[0].is_type(), "kind detection stays presence-only");
    }
}
