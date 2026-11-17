//! Layer comp persistence: image resource 1065 and per-layer `shmd`/`cmls`.

use std::collections::BTreeSet;

use pictura_core::layer_comp::{
    ensure_layer_ids, layer_anchor, layer_id, set_layer_anchor, BlendKey, CapturedState, CompEntry,
    CompRecord, LayerComp, LayerCompStore, PositionAnchor,
};
use pictura_core::{BlendMode, Document, Layer};

use crate::common::Reader;
use crate::descriptor::{get_object_item, read_descriptor, write_descriptor, DescValue};
use crate::image_resources::{
    decode_image_resources, decode_section, encode_image_resources, frame_image_resource,
};

pub(crate) const COMPS_RESOURCE: u16 = 1065;
pub(crate) const ID_SEED_RESOURCE: u16 = 1044;
const COMMENT_KEYS: [&[u8]; 4] = [b"comment", b"Cmnt", b"Cm  ", b"KkCm"];
const WARNING_IDS_KEY: &[u8] = b"KkId";
const WARNING_CLEARED_KEY: &[u8] = b"KkWc";

fn number(value: &DescValue) -> Option<f64> {
    match value {
        DescValue::Long(value) => Some(f64::from(*value)),
        DescValue::Double(value) => Some(*value),
        DescValue::UnitFloat { value, .. } => Some(*value),
        _ => None,
    }
}

fn anchor(value: &DescValue) -> Option<PositionAnchor> {
    let DescValue::Object { items, .. } = value else {
        return None;
    };
    Some(PositionAnchor {
        x: number(get_object_item(items, b"Hrzn")?)?,
        y: number(get_object_item(items, b"Vrtc")?)?,
    })
}

fn text(value: &DescValue) -> Option<String> {
    match value {
        DescValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn long(value: &DescValue) -> Option<i32> {
    match value {
        DescValue::Long(value) => Some(*value),
        _ => None,
    }
}

fn bool_value(value: &DescValue) -> Option<bool> {
    match value {
        DescValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn comp_comment(items: &[(Vec<u8>, DescValue)]) -> String {
    COMMENT_KEYS
        .iter()
        .find_map(|key| get_object_item(items, key).and_then(text))
        .unwrap_or_default()
}

fn read_comp_list(data: &[u8]) -> Option<Vec<LayerComp>> {
    let mut reader = Reader::new(data);
    let DescValue::Object { items, .. } = read_descriptor(&mut reader).ok()? else {
        return None;
    };
    let DescValue::List(list) = get_object_item(&items, b"list")? else {
        return None;
    };
    let mut comps = Vec::new();
    for value in list {
        let DescValue::Object { items, .. } = value else {
            continue;
        };
        let Some(id) = get_object_item(items, b"compID").and_then(long) else {
            continue;
        };
        let name = get_object_item(items, b"Nm  ")
            .and_then(text)
            .unwrap_or_default();
        let record = get_object_item(items, b"capturedInfo")
            .and_then(long)
            .map(|bits| CompRecord::from_bits(bits as u32))
            .unwrap_or_default();
        let mut comp = LayerComp::new(id as u32, name, comp_comment(items), record, Vec::new());
        if let Some(DescValue::List(ids)) = get_object_item(items, WARNING_IDS_KEY) {
            for id in ids.iter().filter_map(long) {
                if comp.entry(id as u32).is_none() {
                    comp.entries.push(CompEntry {
                        layer_id: id as u32,
                        state: CapturedState::default(),
                    });
                }
            }
        }
        if let Some(DescValue::List(ids)) = get_object_item(items, WARNING_CLEARED_KEY) {
            comp.warning_cleared = ids
                .iter()
                .filter_map(long)
                .map(|id| id as u32)
                .collect::<BTreeSet<_>>();
        }
        comps.push(comp);
    }
    Some(comps)
}

fn read_shmd_entries(data: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut reader = Reader::new(data);
    let Ok(count) = reader.u32() else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for _ in 0..count {
        let Ok(sig) = reader.take(4) else { break };
        if sig != b"8BIM" && sig != b"8B64" {
            break;
        }
        let Ok(key) = reader.take(4) else { break };
        let Ok(len) = reader.u64() else { break };
        let Ok(data) = reader.take(len as usize) else {
            break;
        };
        if !len.is_multiple_of(2) && reader.take(1).is_err() {
            break;
        }
        entries.push((key.to_vec(), data.to_vec()));
    }
    entries
}

fn entry_for(comp: &mut LayerComp, layer_id: u32) -> &mut CapturedState {
    if let Some(index) = comp.entries.iter().position(|e| e.layer_id == layer_id) {
        return &mut comp.entries[index].state;
    }
    comp.entries.push(CompEntry {
        layer_id,
        state: CapturedState::default(),
    });
    &mut comp.entries.last_mut().expect("just pushed").state
}

type CmlsVariant = (Vec<i32>, DescValue);
type Cmls = (u32, PositionAnchor, Vec<CmlsVariant>);

fn read_cmls(data: &[u8]) -> Option<Cmls> {
    let mut reader = Reader::new(data);
    let DescValue::Object { items, .. } = read_descriptor(&mut reader).ok()? else {
        return None;
    };
    let layer_id = get_object_item(&items, b"LyrI").and_then(long)? as u32;
    let base = get_object_item(&items, b"origFXRefPoint").and_then(anchor)?;
    let variants = match get_object_item(&items, b"layerSettings") {
        Some(DescValue::List(list)) => list
            .iter()
            .filter_map(|value| {
                let DescValue::Object { items, .. } = value else {
                    return None;
                };
                let ids = match get_object_item(items, b"compList") {
                    Some(DescValue::List(ids)) => ids.iter().filter_map(long).collect::<Vec<_>>(),
                    _ => return None,
                };
                Some((ids, value.clone()))
            })
            .collect(),
        _ => Vec::new(),
    };
    Some((layer_id, base, variants))
}

fn read_layer_state(
    doc: &mut Document,
    _layer: &Layer,
    cmls: &[(Vec<i32>, DescValue)],
    base: PositionAnchor,
    layer_id: u32,
) {
    for (ids, value) in cmls {
        if ids.contains(&0) {
            continue;
        }
        let DescValue::Object { items, .. } = value else {
            continue;
        };
        for id in ids.iter().filter(|id| **id != 0) {
            let Some(index) = doc
                .layer_comps
                .comps()
                .iter()
                .position(|c| c.id == *id as u32)
            else {
                continue;
            };
            let record = doc.layer_comps.comps()[index].record;
            let visible = record
                .visibility
                .then(|| get_object_item(items, b"enab").and_then(bool_value))
                .flatten();
            let blend = if record.appearance {
                get_object_item(items, b"blendOptions")
                    .and_then(|value| {
                        let DescValue::Object { items, .. } = value else {
                            return None;
                        };
                        get_object_item(items, b"Md  ")
                    })
                    .and_then(|value| {
                        let DescValue::Enum { value, .. } = value else {
                            return None;
                        };
                        let key: [u8; 4] = value.as_slice().try_into().ok()?;
                        Some(
                            BlendMode::from_psd_key(key).map_or(BlendKey::Raw(key), BlendKey::Mode),
                        )
                    })
            } else {
                None
            };
            let position = if record.position {
                get_object_item(items, b"FXRefPoint")
                    .and_then(anchor)
                    .or_else(|| {
                        get_object_item(items, b"vectorMask").and_then(|value| {
                            let DescValue::Object { items, .. } = value else {
                                return None;
                            };
                            get_object_item(items, b"Ofst").and_then(anchor)
                        })
                    })
                    .or(Some(base))
            } else {
                None
            };
            let style_visible = record
                .appearance
                .then(|| get_object_item(items, b"KkFx").and_then(bool_value))
                .flatten();
            let Some(comp) = doc.layer_comps.comp_mut(*id as u32) else {
                continue;
            };
            let state = entry_for(comp, layer_id);
            if visible.is_some() {
                state.visible = visible;
            }
            if blend.is_some() {
                state.blend = blend;
            }
            if position.is_some() {
                state.position = position;
            }
            if style_visible.is_some() {
                state.style_visible = style_visible;
            }
        }
    }
}

fn seed_from_resources(doc: &Document) -> u32 {
    decode_image_resources(doc)
        .into_iter()
        .find(|resource| resource.id == ID_SEED_RESOURCE)
        .and_then(|resource| {
            let bytes = resource.data.get(..4)?;
            Some(u32::from_be_bytes(bytes.try_into().ok()?))
        })
        .unwrap_or(1)
}

/// Derive `Document.layer_comps` from the preserved comp resource and each
/// layer's `shmd` block. A malformed resource leaves the store unparsed so the
/// writer keeps the original bytes.
pub(crate) fn resolve(doc: &mut Document) {
    let resource = decode_image_resources(doc)
        .into_iter()
        .find(|resource| resource.id == COMPS_RESOURCE);
    let seed = seed_from_resources(doc);
    doc.layer_comps.set_next_layer_id(seed);

    let Some(resource) = resource else {
        doc.layer_comps.mark_parsed();
        return;
    };
    let Some(comps) = read_comp_list(&resource.data) else {
        return;
    };
    for comp in comps {
        doc.layer_comps.push_comp(comp);
    }
    doc.layer_comps.dirty = false;
    doc.layer_comps.base_dirty = false;
    doc.layer_comps.mark_parsed();
    if !doc.layer_comps.is_empty() {
        ensure_layer_ids(doc, seed);
    }

    fn walk(
        doc: &mut Document,
        layers: &pictura_core::LayerStack,
        anchors: &mut Vec<(u32, PositionAnchor)>,
    ) {
        for layer in layers.iter() {
            if let Some(block) = layer.extra_block(b"shmd") {
                let entries = read_shmd_entries(block.data.as_slice());
                if let Some((_, base, variants)) = entries
                    .iter()
                    .find(|(key, _)| key == b"cmls")
                    .and_then(|(_, data)| read_cmls(data))
                {
                    let id = layer_id(layer).unwrap_or(0);
                    read_layer_state(doc, layer, &variants, base, id);
                    anchors.push((id, base));
                }
            }
            walk(doc, layer.children(), anchors);
        }
    }
    fn apply_anchors(layers: &mut pictura_core::LayerStack, anchors: &[(u32, PositionAnchor)]) {
        for layer in layers.iter_mut() {
            if layer.extra_block(b"fxrp").is_some() {
                if let Some(id) = layer_id(layer) {
                    if let Some((_, anchor)) =
                        anchors.iter().find(|(anchor_id, _)| *anchor_id == id)
                    {
                        set_layer_anchor(layer, *anchor);
                    }
                }
            }
            if let Some(children) = layer.children_mut() {
                apply_anchors(children, anchors);
            }
        }
    }
    let layers = doc.layers.clone();
    let mut anchors = Vec::new();
    walk(doc, &layers, &mut anchors);
    apply_anchors(&mut doc.layers, &anchors);
}

fn output_resource(section: Vec<u8>, id: u16, data: Option<&[u8]>) -> Vec<u8> {
    let (mut resources, consumed) = decode_section(&section);
    let existing = resources.iter().position(|resource| resource.id == id);
    match (existing, data) {
        (Some(index), Some(data)) => {
            resources[index] = frame_image_resource(id, "", data);
        }
        (Some(index), None) => {
            resources.remove(index);
        }
        (None, Some(data)) => resources.push(frame_image_resource(id, "", data)),
        (None, None) => return section,
    }
    let mut out = encode_image_resources(&resources);
    out.extend_from_slice(&section[consumed..]);
    out
}

fn comp_descriptor(store: &LayerCompStore) -> DescValue {
    let comps = store
        .comps()
        .iter()
        .map(|comp| {
            let mut items = vec![
                (b"Nm  ".to_vec(), DescValue::Text(comp.name.clone())),
                (b"compID".to_vec(), DescValue::Long(comp.id as i32)),
                (
                    b"capturedInfo".to_vec(),
                    DescValue::Long(comp.record.bits() as i32),
                ),
            ];
            if !comp.comment.is_empty() {
                items.push((b"KkCm".to_vec(), DescValue::Text(comp.comment.clone())));
            }
            items.push((
                WARNING_IDS_KEY.to_vec(),
                DescValue::List(
                    comp.entries
                        .iter()
                        .map(|entry| DescValue::Long(entry.layer_id as i32))
                        .collect(),
                ),
            ));
            if !comp.warning_cleared.is_empty() {
                items.push((
                    WARNING_CLEARED_KEY.to_vec(),
                    DescValue::List(
                        comp.warning_cleared
                            .iter()
                            .map(|id| DescValue::Long(*id as i32))
                            .collect(),
                    ),
                ));
            }
            DescValue::Object {
                name: String::new(),
                class_id: b"Comp".to_vec(),
                items,
            }
        })
        .collect();
    DescValue::Object {
        name: String::new(),
        class_id: b"CompList".to_vec(),
        items: vec![(b"list".to_vec(), DescValue::List(comps))],
    }
}

/// Replace/remove resource 1065 and refresh resource 1044 after identity
/// assignment. `section` is the image-resource section the writer will emit.
pub(crate) fn output_resources(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    if !doc.layer_comps.dirty && !doc.layer_comps.ids_dirty {
        return section;
    }
    let mut section = section;
    if doc.layer_comps.dirty {
        let data = if doc.layer_comps.is_empty() {
            None
        } else {
            Some(write_descriptor(&comp_descriptor(&doc.layer_comps)))
        };
        section = output_resource(section, COMPS_RESOURCE, data.as_deref());
    }
    if doc.layer_comps.ids_dirty {
        let seed = doc
            .layer_comps
            .next_layer_id()
            .max(max_layer_id(doc).saturating_add(1));
        section = output_resource(section, ID_SEED_RESOURCE, Some(&seed.to_be_bytes()));
    }
    section
}

fn max_layer_id(doc: &Document) -> u32 {
    fn walk(layers: &pictura_core::LayerStack, highest: &mut u32) {
        for layer in layers.iter() {
            if let Some(id) = layer_id(layer) {
                *highest = (*highest).max(id);
            }
            walk(layer.children(), highest);
        }
    }
    let mut highest = 0;
    walk(&doc.layers, &mut highest);
    highest
}

fn shmd_entry(out: &mut Vec<u8>, key: &[u8], data: &[u8]) {
    out.extend_from_slice(b"8BIM");
    out.extend_from_slice(key);
    out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    out.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        out.push(0);
    }
}

fn merge_unknown(
    mut items: Vec<(Vec<u8>, DescValue)>,
    original: Option<&DescValue>,
) -> Vec<(Vec<u8>, DescValue)> {
    let Some(DescValue::Object {
        items: original, ..
    }) = original
    else {
        return items;
    };
    let known: BTreeSet<Vec<u8>> = items.iter().map(|(key, _)| key.clone()).collect();
    for (key, value) in original {
        if key.as_slice() != b"compList" && !known.contains(key) {
            items.push((key.clone(), value.clone()));
        }
    }
    items
}

fn variant_for(comp: &LayerComp, layer_id: u32, original: Option<&DescValue>) -> Option<DescValue> {
    let state = comp.entry(layer_id)?;
    let mut items = Vec::new();
    if comp.record.visibility {
        if let Some(visible) = state.visible {
            items.push((b"enab".to_vec(), DescValue::Bool(visible)));
        }
    }
    if comp.record.appearance {
        if let Some(blend) = state.blend {
            let key = match blend {
                BlendKey::Mode(mode) => mode.to_psd_key(),
                BlendKey::Raw(key) => key,
            };
            items.push((
                b"blendOptions".to_vec(),
                DescValue::Object {
                    name: String::new(),
                    class_id: b"null".to_vec(),
                    items: vec![(
                        b"Md  ".to_vec(),
                        DescValue::Enum {
                            kind: b"BlnM".to_vec(),
                            value: key.to_vec(),
                        },
                    )],
                },
            ));
        }
        if let Some(style) = state.style_visible {
            items.push((b"KkFx".to_vec(), DescValue::Bool(style)));
        }
    }
    if comp.record.position {
        if let Some(position) = state.position {
            let point = DescValue::Object {
                name: String::new(),
                class_id: b"null".to_vec(),
                items: vec![
                    (b"Hrzn".to_vec(), DescValue::Double(position.x)),
                    (b"Vrtc".to_vec(), DescValue::Double(position.y)),
                ],
            };
            items.push((b"FXRefPoint".to_vec(), point.clone()));
            items.push((
                b"vectorMask".to_vec(),
                DescValue::Object {
                    name: String::new(),
                    class_id: b"null".to_vec(),
                    items: vec![(b"Ofst".to_vec(), point)],
                },
            ));
        }
    }
    if items.is_empty() && !comp.record.position {
        return None;
    }
    items = merge_unknown(items, original);
    items.push((
        b"compList".to_vec(),
        DescValue::List(vec![DescValue::Long(comp.id as i32)]),
    ));
    Some(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items,
    })
}

fn cmls_descriptor(
    store: &LayerCompStore,
    layer: &Layer,
    original: &[(Vec<i32>, DescValue)],
    only_base: bool,
) -> Option<Vec<u8>> {
    let id = layer_id(layer)?;
    let participating = store.comps().iter().any(|comp| comp.entry(id).is_some());
    if !only_base && !participating {
        return None;
    }
    let mut variants = if only_base {
        original
            .iter()
            .filter(|(ids, _)| !ids.contains(&0))
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>()
    } else {
        store
            .comps()
            .iter()
            .filter_map(|comp| {
                let original = original
                    .iter()
                    .find(|(ids, _)| ids.contains(&(comp.id as i32)));
                variant_for(comp, id, original.map(|(_, value)| value))
            })
            .collect::<Vec<_>>()
    };
    let base = layer_anchor(layer);
    let base_blend = match BlendKey::from_layer(layer) {
        BlendKey::Mode(mode) => mode.to_psd_key(),
        BlendKey::Raw(key) => key,
    };
    let mut base_items = vec![
        (b"enab".to_vec(), DescValue::Bool(layer.visible)),
        (
            b"blendOptions".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"null".to_vec(),
                items: vec![(
                    b"Md  ".to_vec(),
                    DescValue::Enum {
                        kind: b"BlnM".to_vec(),
                        value: base_blend.to_vec(),
                    },
                )],
            },
        ),
        (
            b"FXRefPoint".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"null".to_vec(),
                items: vec![
                    (b"Hrzn".to_vec(), DescValue::Double(base.x)),
                    (b"Vrtc".to_vec(), DescValue::Double(base.y)),
                ],
            },
        ),
    ];
    let base_original = original
        .iter()
        .find(|(ids, _)| ids.contains(&0))
        .map(|(_, value)| value);
    base_items = merge_unknown(base_items, base_original);
    base_items.push((
        b"compList".to_vec(),
        DescValue::List(vec![DescValue::Long(0)]),
    ));
    variants.push(DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: base_items,
    });
    let root = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: vec![
            (
                b"origFXRefPoint".to_vec(),
                DescValue::Object {
                    name: String::new(),
                    class_id: b"null".to_vec(),
                    items: vec![
                        (b"Hrzn".to_vec(), DescValue::Double(base.x)),
                        (b"Vrtc".to_vec(), DescValue::Double(base.y)),
                    ],
                },
            ),
            (b"LyrI".to_vec(), DescValue::Long(id as i32)),
            (b"layerSettings".to_vec(), DescValue::List(variants)),
        ],
    };
    Some(write_descriptor(&root))
}

/// The `shmd` block contents for `layer`, or `None` to keep its preserved
/// block. `Some(empty)` means the block should be dropped.
pub(crate) fn layer_shmd(store: &LayerCompStore, layer: &Layer) -> Option<Vec<u8>> {
    if !store.parsed || (!store.dirty && !store.base_dirty) {
        return None;
    }
    let original = layer
        .extra_block(b"shmd")
        .map(|block| read_shmd_entries(block.data.as_slice()))
        .unwrap_or_default();
    if store.dirty && store.is_empty() {
        let kept: Vec<(Vec<u8>, Vec<u8>)> = original
            .into_iter()
            .filter(|(key, _)| key != b"cmls")
            .collect();
        if kept.is_empty() {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        out.extend_from_slice(&(kept.len() as u32).to_be_bytes());
        for (key, data) in kept {
            shmd_entry(&mut out, &key, &data);
        }
        return Some(out);
    }
    let original_variants = original
        .iter()
        .find(|(key, _)| key == b"cmls")
        .and_then(|(_, data)| read_cmls(data))
        .map(|(_, _, variants)| variants)
        .unwrap_or_default();
    let only_base = !store.dirty && store.base_dirty;
    let id = layer_id(layer)?;
    let participating = store.comps().iter().any(|comp| comp.entry(id).is_some());
    if !participating && !only_base {
        return None;
    }
    let cmls = cmls_descriptor(store, layer, &original_variants, only_base)?;
    let cmls_count = original.iter().filter(|(key, _)| key == b"cmls").count();
    let mut out = Vec::new();
    out.extend_from_slice(&((original.len() - cmls_count + 1) as u32).to_be_bytes());
    let mut wrote_cmls = false;
    for (key, data) in original {
        if key == b"cmls" {
            if !wrote_cmls {
                shmd_entry(&mut out, b"cmls", &cmls);
                wrote_cmls = true;
            }
        } else {
            shmd_entry(&mut out, &key, &data);
        }
    }
    if !wrote_cmls {
        shmd_entry(&mut out, b"cmls", &cmls);
    }
    Some(out)
}

/// True when the layer writer should consult [`layer_shmd`] for this document.
pub(crate) fn rewrites_shmd(store: &LayerCompStore) -> bool {
    store.parsed && (store.dirty || store.base_dirty)
}

/// The image-resource section after the comp chain, for the save path.
pub(crate) fn resources_for_output(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    output_resources(doc, section)
}

#[cfg(test)]
#[path = "layer_comps_tests.rs"]
mod tests;
