//! The document-level `Txt2` text-frame data: which text layers sit on a path
//! or fill a closed path's area.
//!
//! Photoshop keeps this geometry outside `TySh`, in `Txt2`'s
//! `ResourceDict.TextFrameSet`, one entry per text layer in text-layer order.
//! The shape is established from public `ag-psd` fixtures:
//!
//! ```text
//! /8 << /0 [ << /0 << /1 << /0 [8 numbers per cubic segment] >>
//!             /2 << /0 2 /6 [start_fraction length_fraction]
//!                   /11 << /3 0 /5 0 /7 false >> >> >> >> ] >>
//! ```
//!
//! `Type` 1 is area type (`TextRange [-2, -2]`, `PathData 18 -2 / spacing -2`);
//! `Type` 2 is on-path type. Every other key in an existing `Txt2` survives.

use pictura_core::path::{PathPoint, Subpath};
use pictura_core::{LayerStack, TextPath, TextPathKind, TypeTool};

use crate::common::{is_psb_big_key, Reader};
use crate::engine_data::{as_list, get, parse_engine_data, EngineValue};
use crate::engine_data_encode::encode_engine_data;

/// Every type tool in document order (bottom-first, depth-first), which is the
/// order `TextFrameSet` entries use.
fn text_tools<'a>(layers: &'a LayerStack, out: &mut Vec<&'a TypeTool>) {
    for layer in layers.iter() {
        if let Some(tool) = layer.type_tool.as_ref() {
            out.push(tool);
        }
        if !layer.children.is_empty() {
            text_tools(&layer.children, out);
        }
    }
}

fn assign_paths(layers: &mut LayerStack, paths: &mut impl Iterator<Item = Option<TextPath>>) {
    for layer in layers.iter_mut() {
        if let Some(tool) = layer.type_tool.as_mut() {
            tool.path = paths.next().flatten();
        }
        assign_paths(&mut layer.children, paths);
    }
}

/// Apply the `Txt2` paths to the document's text layers, in text-layer order.
pub(crate) fn resolve_text_paths(layers: &mut LayerStack, extra: &[u8], is_psb: bool) {
    let Some(data) = find_block(extra, b"Txt2", is_psb) else {
        return;
    };
    let Ok(root) = parse_engine_data(data) else {
        return;
    };
    let Some(resource) = resource_dict(&root) else {
        return;
    };
    let Some(frame_set) = get(resource, "8").or_else(|| get(resource, "TextFrameSet")) else {
        return;
    };
    let entries = frame_set_entries(frame_set);
    let mut paths = entries.iter().map(entry_path);
    assign_paths(layers, &mut paths);
}

/// Rewrite the `Txt2` block's `TextFrameSet` from the layers' paths, preserving
/// every other global block and every other `Txt2` key.
pub(crate) fn encode_text_paths(layers: &LayerStack, existing: &[u8], is_psb: bool) -> Vec<u8> {
    let mut tools = Vec::new();
    text_tools(layers, &mut tools);
    let has_path = tools.iter().any(|tool| tool.path.is_some());
    let block = find_block(existing, b"Txt2", is_psb);
    let existing_has_path = block
        .and_then(|data| parse_engine_data(data).ok())
        .and_then(|root| {
            let resource = resource_dict(&root)?;
            let frame_set = get(resource, "8").or_else(|| get(resource, "TextFrameSet"))?;
            Some(frame_set_entries(frame_set).iter().any(entry_has_path))
        })
        .unwrap_or(false);
    if !has_path && !existing_has_path {
        return existing.to_vec();
    }
    let mut root = block
        .and_then(|data| parse_engine_data(data).ok())
        .unwrap_or_else(|| {
            EngineValue::Dict(vec![(
                "0".to_string(),
                EngineValue::Dict(vec![("8".to_string(), frame_set(vec![]))]),
            )])
        });
    let numeric = get(&root, "ResourceDict").is_none();
    let resource_key = if numeric { "0" } else { "ResourceDict" };
    if !matches!(get(&root, resource_key), Some(EngineValue::Dict(_))) {
        root = EngineValue::Dict(vec![(resource_key.to_string(), EngineValue::Dict(vec![]))]);
    }
    let resource = match root {
        EngineValue::Dict(ref mut items) => items
            .iter_mut()
            .find(|(key, _)| key == resource_key)
            .map(|(_, value)| value)
            .expect("resource dict"),
        _ => return existing.to_vec(),
    };
    let EngineValue::Dict(resource_items) = resource else {
        return existing.to_vec();
    };
    let set_key = if numeric { "8" } else { "TextFrameSet" };
    let entries = tools.into_iter().map(entry).collect();
    let value = frame_set(entries);
    if let Some((_, slot)) = resource_items.iter_mut().find(|(key, _)| key == set_key) {
        *slot = value;
    } else {
        resource_items.push((set_key.to_string(), value));
    }
    let Ok(encoded) = encode_engine_data(&root) else {
        return existing.to_vec();
    };
    replace_block(existing, b"Txt2", &encoded, is_psb)
}

fn entry(tool: &TypeTool) -> EngineValue {
    let Some(path) = tool.path.as_ref() else {
        return EngineValue::Dict(Vec::new());
    };
    let points: Vec<EngineValue> = (0..path.subpath.segment_count())
        .filter_map(|i| path.subpath.segment(i))
        .flat_map(|segment| {
            segment
                .into_iter()
                .flat_map(|(x, y)| [EngineValue::Double(x), EngineValue::Double(y)])
        })
        .collect();
    let (kind, range, path_data) = match path.kind {
        TextPathKind::OnPath => (
            2,
            vec![
                EngineValue::Double(start_fraction(path)),
                EngineValue::Double(1.0),
            ],
            vec![
                ("3".to_string(), EngineValue::Int(0)),
                ("5".to_string(), EngineValue::Int(0)),
                ("7".to_string(), EngineValue::Bool(false)),
            ],
        ),
        TextPathKind::Area => (
            1,
            vec![EngineValue::Double(-2.0), EngineValue::Double(-2.0)],
            vec![
                ("18".to_string(), EngineValue::Int(-2)),
                ("spacing".to_string(), EngineValue::Int(-2)),
            ],
        ),
    };
    let curve = EngineValue::Dict(vec![("0".to_string(), EngineValue::List(points))]);
    let data = EngineValue::Dict(vec![
        ("0".to_string(), EngineValue::Int(kind)),
        ("6".to_string(), EngineValue::List(range)),
        ("11".to_string(), EngineValue::Dict(path_data)),
    ]);
    let text_path = EngineValue::Dict(vec![("1".to_string(), curve), ("2".to_string(), data)]);
    EngineValue::Dict(vec![("0".to_string(), text_path)])
}

fn start_fraction(path: &TextPath) -> f64 {
    let total = path.subpath.flattened(0.25).total;
    if total > 0.0 {
        (path.start / total).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn frame_set(entries: Vec<EngineValue>) -> EngineValue {
    EngineValue::Dict(vec![("0".to_string(), EngineValue::List(entries))])
}

fn frame_set_entries(value: &EngineValue) -> Vec<EngineValue> {
    match value {
        EngineValue::List(entries) => entries.clone(),
        EngineValue::Dict(items) => items
            .iter()
            .find(|(key, _)| key == "0")
            .map(|(_, value)| as_list(value).to_vec())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn entry_has_path(entry: &EngineValue) -> bool {
    get(entry, "0")
        .and_then(|text_path| get(text_path, "1"))
        .is_some()
}

fn entry_path(entry: &EngineValue) -> Option<TextPath> {
    let text_path = get(entry, "0")?;
    let data = get(text_path, "2")?;
    let kind = get(data, "0").and_then(as_i64)?;
    let points = get(text_path, "1")
        .and_then(|curve| get(curve, "0"))
        .map(as_list)?;
    let mut values = Vec::with_capacity(points.len());
    for value in points {
        values.push(as_f64(value)?);
    }
    if values.len() < 8 || values.len() % 8 != 0 {
        return None;
    }
    let segments: Vec<[(f64, f64); 4]> = values
        .as_chunks::<8>()
        .0
        .iter()
        .map(|chunk| {
            [
                (chunk[0], chunk[1]),
                (chunk[2], chunk[3]),
                (chunk[4], chunk[5]),
                (chunk[6], chunk[7]),
            ]
        })
        .collect();
    let (kind, closed) = match kind {
        1 => (TextPathKind::Area, true),
        2 => (TextPathKind::OnPath, false),
        _ => return None,
    };
    let subpath = subpath_from_segments(&segments, closed);
    // ponytail: `FrameMatrix` and the second `TextRange` value are not applied;
    // Pictura authors its path points in document coordinates and reads back
    // the start fraction it wrote.
    let start = match kind {
        TextPathKind::OnPath => {
            let fraction = get(data, "6")
                .map(as_list)
                .and_then(|values| values.first())
                .and_then(as_f64)
                .unwrap_or(0.0);
            let total = subpath.flattened(0.25).total;
            (fraction * total).max(0.0)
        }
        TextPathKind::Area => 0.0,
    };
    Some(TextPath {
        kind,
        subpath,
        start,
    })
}

fn subpath_from_segments(segments: &[[(f64, f64); 4]], closed: bool) -> Subpath {
    let mut points = Vec::with_capacity(segments.len() + usize::from(!closed));
    for (index, segment) in segments.iter().enumerate() {
        let in_handle = if index == 0 {
            closed
                .then(|| segments.last().map(|last| last[2]))
                .flatten()
        } else {
            Some(segments[index - 1][2])
        };
        points.push(PathPoint {
            anchor: segment[0],
            in_handle,
            out_handle: Some(segment[1]),
            smooth: true,
        });
    }
    if !closed {
        if let Some(last) = segments.last() {
            points.push(PathPoint {
                anchor: last[3],
                in_handle: Some(last[2]),
                out_handle: None,
                smooth: false,
            });
        }
    }
    Subpath { points, closed }
}

fn resource_dict(root: &EngineValue) -> Option<&EngineValue> {
    get(root, "ResourceDict").or_else(|| get(root, "0"))
}

fn as_i64(value: &EngineValue) -> Option<i64> {
    match value {
        EngineValue::Int(value) => Some(*value),
        EngineValue::Double(value) => Some(*value as i64),
        _ => None,
    }
}

fn as_f64(value: &EngineValue) -> Option<f64> {
    match value {
        EngineValue::Int(value) => Some(*value as f64),
        EngineValue::Double(value) => Some(*value),
        _ => None,
    }
}

/// The data of the first global `key` block in `extra`.
fn find_block<'a>(extra: &'a [u8], key: &[u8; 4], is_psb: bool) -> Option<&'a [u8]> {
    let mut reader = Reader::new(extra);
    loop {
        let Ok(sig) = reader.take(4) else {
            return None;
        };
        if sig != b"8BIM" && sig != b"8B64" {
            return None;
        }
        let Ok(block_key) = reader.take(4) else {
            return None;
        };
        let block_key: [u8; 4] = block_key.try_into().ok()?;
        let len = if is_psb && is_psb_big_key(&block_key) {
            usize::try_from(reader.u64().ok()?).ok()?
        } else {
            reader.u32().ok()? as usize
        };
        let Ok(data) = reader.take(len) else {
            return None;
        };
        let pad = (4 - len % 4) % 4;
        if reader.skip(pad).is_err() {
            return None;
        }
        if &block_key == key {
            return Some(data);
        }
    }
}

/// Replace the first global `key` block, preserving the other bytes.
fn replace_block(existing: &[u8], key: &[u8; 4], data: &[u8], is_psb: bool) -> Vec<u8> {
    let mut reader = Reader::new(existing);
    let mut out = Vec::with_capacity(existing.len() + data.len());
    let mut found = false;
    loop {
        let start = reader.pos;
        let Ok(sig) = reader.take(4) else {
            out.extend_from_slice(&existing[start..]);
            if !found {
                write_block(&mut out, key, data, is_psb);
            }
            return out;
        };
        if sig != b"8BIM" && sig != b"8B64" {
            out.extend_from_slice(&existing[start..]);
            if !found {
                write_block(&mut out, key, data, is_psb);
            }
            return out;
        }
        let Ok(block_key) = reader.take(4) else {
            out.extend_from_slice(&existing[start..]);
            return out;
        };
        let block_key: [u8; 4] = block_key.try_into().expect("4 bytes");
        let big = is_psb && is_psb_big_key(&block_key);
        let len = if big {
            match reader.u64() {
                Ok(len) => usize::try_from(len).unwrap_or(usize::MAX),
                Err(_) => {
                    out.extend_from_slice(&existing[start..]);
                    return out;
                }
            }
        } else {
            match reader.u32() {
                Ok(len) => len as usize,
                Err(_) => {
                    out.extend_from_slice(&existing[start..]);
                    return out;
                }
            }
        };
        if reader.take(len).is_err() {
            out.extend_from_slice(&existing[start..]);
            return out;
        }
        let pad = (4 - len % 4) % 4;
        if reader.skip(pad).is_err() {
            out.extend_from_slice(&existing[start..]);
            return out;
        }
        if &block_key == key {
            write_block(&mut out, key, data, is_psb);
            found = true;
        } else {
            out.extend_from_slice(&existing[start..reader.pos]);
        }
    }
}

fn write_block(out: &mut Vec<u8>, key: &[u8; 4], data: &[u8], is_psb: bool) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    if is_psb && is_psb_big_key(key) {
        out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    } else {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(data);
    let pad = (4 - data.len() % 4) % 4;
    out.extend(std::iter::repeat_n(0u8, pad));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::PathPoint;
    use pictura_core::Layer;

    fn rect_subpath(w: f64, h: f64, closed: bool) -> Subpath {
        Subpath {
            points: vec![
                PathPoint {
                    anchor: (0.0, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (w, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (w, h),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (0.0, h),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed,
        }
    }

    fn tool(path: Option<TextPath>) -> TypeTool {
        TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 10.0, 20.0],
            text: "Hi".into(),
            bounds: [0; 4],
            text_desc: Vec::new(),
            warp_desc: Vec::new(),
            fonts: Vec::new(),
            style: None,
            vertical: false,
            box_bounds: None,
            path,
        }
    }

    fn doc_with(layers: Vec<Layer>) -> LayerStack {
        LayerStack::from_layers(layers)
    }

    #[test]
    fn text_path_round_trips() {
        let on_path = TextPath {
            kind: TextPathKind::OnPath,
            subpath: rect_subpath(200.0, 0.0, false),
            start: 50.0,
        };
        let area = TextPath {
            kind: TextPathKind::Area,
            subpath: rect_subpath(100.0, 80.0, true),
            start: 0.0,
        };
        let layers = doc_with(vec![
            Layer {
                type_tool: Some(tool(Some(on_path.clone()))),
                ..Default::default()
            },
            Layer {
                type_tool: Some(tool(Some(area.clone()))),
                ..Default::default()
            },
        ]);
        let extra = encode_text_paths(&layers, &[], false);
        let mut read = doc_with(vec![
            Layer {
                type_tool: Some(tool(None)),
                ..Default::default()
            },
            Layer {
                type_tool: Some(tool(None)),
                ..Default::default()
            },
        ]);
        resolve_text_paths(&mut read, &extra, false);
        let first = read[0].type_tool.as_ref().unwrap().path.as_ref().unwrap();
        let second = read[1].type_tool.as_ref().unwrap().path.as_ref().unwrap();
        assert_eq!(first.kind, TextPathKind::OnPath);
        assert!((first.start - 50.0).abs() < 0.5);
        assert_eq!(second.kind, TextPathKind::Area);
        assert!(second.subpath.closed);
        assert_eq!(second.subpath.points.len(), 4);
    }

    #[test]
    fn text_path_survives_psd() {
        use pictura_core::{BitDepth, Channel, ColorMode, Document, LayerBlock, PsdRect, TypeSpec};

        let path = TextPath {
            kind: TextPathKind::OnPath,
            subpath: rect_subpath(200.0, 0.0, false),
            start: 40.0,
        };
        let mut spec = TypeSpec::new("Hi", "Liberation Sans", 20.0);
        spec.path = Some(path.clone());
        let tool = crate::type_write::author_type_tool(&spec, [0.0, -20.0, 40.0, 10.0], None, None);
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 40,
            right: 200,
        };
        let mut doc = Document::new(200, 40, ColorMode::Rgb, BitDepth::Eight);
        doc.layers
            .push(Layer {
                name: "Hi".into(),
                rect,
                channels: [0, 1, 2, -1]
                    .into_iter()
                    .map(|id| Channel {
                        id,
                        data: vec![0; 200 * 40].into(),
                    })
                    .collect(),
                extra_blocks: vec![LayerBlock {
                    key: *b"TySh",
                    data: crate::type_tool::encode_type_tool(&tool),
                }],
                type_tool: Some(tool),
                ..Default::default()
            })
            .expect("push the type layer");
        let bytes = crate::write_psd(&doc).expect("writes");
        let back = crate::read_psd(&bytes).expect("reads");
        let restored = back.layers[0]
            .type_tool
            .as_ref()
            .and_then(|tool| tool.path.as_ref())
            .expect("the path survives");
        assert_eq!(restored.kind, TextPathKind::OnPath);
        assert!((restored.start - 40.0).abs() < 0.5);
        assert_eq!(restored.subpath.points.len(), path.subpath.points.len());
    }

    #[test]
    fn reads_text_frame_set_entries() {
        let engine = b"<< /0 << /8 << /0 [ \
            << /0 << /1 << /0 [ 0 0 10 0 20 0 30 0 ] >> \
                  /2 << /0 2 /6 [ 0.5 1.0 ] /11 << /3 0 /5 0 /7 false >> >> >> >> \
            << /0 << /1 << /0 [ 0 0 10 0 10 0 10 10 10 10 0 10 0 10 0 0 ] >> \
                  /2 << /0 1 /6 [ -2 -2 ] /11 << /18 -2 /spacing -2 >> >> >> >> \
        ] >> >> >>";
        let mut block = Vec::new();
        write_block(&mut block, b"Txt2", engine, false);
        let mut layers = doc_with(vec![
            Layer {
                type_tool: Some(tool(None)),
                ..Default::default()
            },
            Layer {
                type_tool: Some(tool(None)),
                ..Default::default()
            },
        ]);
        resolve_text_paths(&mut layers, &block, false);
        let first = layers[0].type_tool.as_ref().unwrap().path.as_ref().unwrap();
        assert_eq!(first.kind, TextPathKind::OnPath);
        assert!((first.start - 15.0).abs() < 0.5);
        let second = layers[1].type_tool.as_ref().unwrap().path.as_ref().unwrap();
        assert_eq!(second.kind, TextPathKind::Area);
        assert!(second.subpath.closed);
    }
}
