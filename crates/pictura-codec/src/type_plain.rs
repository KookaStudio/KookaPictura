//! Whether a type layer is redrawn faithfully from its first style alone: one
//! style run and no warp. Live text in a Photoshop PDF (`interop/pdf-export`)
//! sets only such layers; any other keeps its rendered pixels.

use pictura_core::TypeTool;

use crate::common::Reader;
use crate::descriptor::{get_object_item, read_descriptor, DescValue};
use crate::engine_data::{as_list, path};

/// True when `tool` sets its text in one style and is not warped. A layer
/// without descriptors (authored in memory) is plain.
pub fn type_is_plain(tool: &TypeTool) -> bool {
    unwarped(&tool.warp_desc) && one_style(&tool.text_desc)
}

fn unwarped(warp_desc: &[u8]) -> bool {
    if warp_desc.is_empty() {
        return true;
    }
    let Ok(DescValue::Object { items, .. }) = read_descriptor(&mut Reader::new(warp_desc)) else {
        return false;
    };
    match get_object_item(&items, b"warpStyle") {
        None => true,
        Some(DescValue::Enum { value, .. }) => value == b"warpNone",
        Some(_) => false,
    }
}

fn one_style(text_desc: &[u8]) -> bool {
    let Some(root) = crate::type_write::existing_engine_tree(text_desc) else {
        return true;
    };
    let runs = path(&root, &["EngineDict", "StyleRun", "RunArray"]).map_or(&[][..], as_list);
    let sheets: Vec<_> = runs
        .iter()
        .map(|run| path(run, &["StyleSheet", "StyleSheetData"]))
        .collect();
    sheets.windows(2).all(|pair| pair[0] == pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::write_descriptor;

    fn descriptor(items: Vec<(&str, DescValue)>) -> Vec<u8> {
        write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: items
                .into_iter()
                .map(|(k, v)| (k.as_bytes().to_vec(), v))
                .collect(),
        })
    }

    fn warp(style: &str) -> Vec<u8> {
        descriptor(vec![(
            "warpStyle",
            DescValue::Enum {
                kind: b"warpStyle".to_vec(),
                value: style.as_bytes().to_vec(),
            },
        )])
    }

    fn engine(runs: &[&str]) -> Vec<u8> {
        let runs: String = runs
            .iter()
            .map(|font| format!("<< /StyleSheet << /StyleSheetData << /Font {font} >> >> >> "))
            .collect();
        let data = format!("<< /EngineDict << /StyleRun << /RunArray [ {runs}] >> >> >>");
        let mut raw = b"tdta".to_vec();
        raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
        raw.extend_from_slice(data.as_bytes());
        descriptor(vec![("EngineData", DescValue::Raw(raw))])
    }

    fn tool(text_desc: Vec<u8>, warp_desc: Vec<u8>) -> TypeTool {
        TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: "Hi".into(),
            bounds: [0; 4],
            text_desc: text_desc.into(),
            warp_desc: warp_desc.into(),
            fonts: Vec::new(),
            style: None,
            vertical: false,
            box_bounds: None,
            path: None,
        }
    }

    #[test]
    fn plain_type_is_one_unwarped_style() {
        assert!(type_is_plain(&tool(Vec::new(), Vec::new())));
        assert!(type_is_plain(&tool(engine(&["0", "0"]), warp("warpNone"))));
    }

    #[test]
    fn a_warp_or_a_second_style_is_not_plain() {
        assert!(!type_is_plain(&tool(Vec::new(), warp("warpArc"))));
        assert!(!type_is_plain(&tool(engine(&["0", "1"]), Vec::new())));
    }
}
