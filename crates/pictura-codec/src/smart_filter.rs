//! Edit a smart filter's `Fltr` options in place.
//!
//! The preserved `SoLd`/`SoLE` block stays the source of truth: this module
//! parses it to the descriptor DOM, replaces one key, and re-serializes the
//! whole block so every unmodeled key survives the round trip.

use pictura_core::Layer;

use crate::common::Reader;
use crate::descriptor::{self, get_object_item_mut, set_object_item, DescValue};
use crate::error::PsdError;

/// Replace (or insert) one key in the `Fltr` options of the Pictura Raw filter at
/// `filter_index` on `layer`, rewriting the preserved `SoLd`/`SoLE` block in place.
pub fn set_camera_raw_option(
    layer: &mut Layer,
    filter_index: usize,
    key: &[u8],
    value: DescValue,
) -> Result<(), PsdError> {
    let block_idx = layer
        .extra_blocks
        .iter()
        .position(|b| &b.key == b"SoLd")
        .or_else(|| layer.extra_blocks.iter().position(|b| &b.key == b"SoLE"))
        .ok_or_else(|| malformed("layer has no SoLd/SoLE smart object block"))?;

    let data = &layer.extra_blocks[block_idx].data;
    let mut r = Reader::new(data);
    let signature = r.take(4)?.to_vec();
    if signature != b"soLD" && signature != b"soLE" {
        return Err(PsdError::Unsupported(format!(
            "smart object layer data signature {:?}",
            String::from_utf8_lossy(&signature)
        )));
    }
    let _outer_version = r.u32()?;
    let mut desc = descriptor::read_descriptor(&mut r)?;

    let options = {
        let DescValue::Object {
            items: desc_items, ..
        } = &mut desc
        else {
            return Err(malformed("descriptor is not an object"));
        };
        let DescValue::Object {
            items: filter_fx, ..
        } = get_object_item_mut(desc_items, b"filterFX")
            .ok_or_else(|| malformed("missing filterFX"))?
        else {
            return Err(malformed("filterFX is not an object"));
        };
        let DescValue::List(filters) = get_object_item_mut(filter_fx, b"filterFXList")
            .ok_or_else(|| malformed("missing filterFXList"))?
        else {
            return Err(malformed("filterFXList is not a list"));
        };
        let DescValue::Object { items: item, .. } = filters
            .get_mut(filter_index)
            .ok_or_else(|| malformed("filter index out of range"))?
        else {
            return Err(malformed("filter entry is not an object"));
        };
        let fltr = get_object_item_mut(item, b"Fltr").ok_or_else(|| malformed("missing Fltr"))?;
        let DescValue::Object {
            items: fltr_items, ..
        } = fltr
        else {
            return Err(malformed("Fltr is not an object"));
        };
        set_object_item(fltr_items, key, value);
        descriptor::write_descriptor(fltr)
    };

    let mut new_data = data[..8].to_vec();
    new_data.extend_from_slice(&descriptor::write_descriptor(&desc));
    // psd-tools reads layer tagged blocks without consuming a trailing pad, so the
    // declared length must be even. The reference 4-aligns the SoLd block data after
    // the descriptor; match that so a same-size edit keeps the original length.
    // ponytail: zero pad to 4; revisit if a reader rejects the trailing bytes.
    while !new_data.len().is_multiple_of(4) {
        new_data.push(0);
    }
    layer.extra_blocks[block_idx].data = new_data.clone();

    if let Some(so) = layer.smart_object.as_mut() {
        so.config_descriptor = new_data;
        if let Some(filter) = so.smart_filters.get_mut(filter_index) {
            filter.options = options;
        }
    }
    Ok(())
}

fn malformed(what: &str) -> PsdError {
    PsdError::Invalid(format!("Pictura Raw filter descriptor: {what}"))
}

/// The index of the preserved `SoLd`/`SoLE` config block, preferring `SoLd`.
fn config_block_index(layer: &Layer) -> Option<usize> {
    layer
        .extra_blocks
        .iter()
        .position(|b| &b.key == b"SoLd")
        .or_else(|| layer.extra_blocks.iter().position(|b| &b.key == b"SoLE"))
}

/// Parse a preserved config block into its descriptor, validating the signature.
fn parse_config(layer: &Layer, block_idx: usize) -> Result<DescValue, PsdError> {
    let data = &layer.extra_blocks[block_idx].data;
    let mut r = Reader::new(data);
    let signature = r.take(4)?.to_vec();
    if signature != b"soLD" && signature != b"soLE" {
        return Err(PsdError::Unsupported(format!(
            "smart object layer data signature {:?}",
            String::from_utf8_lossy(&signature)
        )));
    }
    let _outer_version = r.u32()?;
    descriptor::read_descriptor(&mut r)
}

/// Re-serialize a descriptor onto the block's `soLD`/`soLE` + version prefix,
/// padded to the same 4-byte boundary as `insert_into_config`.
fn serialize_config(data: &[u8], desc: &DescValue) -> Vec<u8> {
    let mut new_data = data[..8].to_vec();
    new_data.extend_from_slice(&descriptor::write_descriptor(desc));
    while !new_data.len().is_multiple_of(4) {
        new_data.push(0);
    }
    new_data
}

/// Store rewritten block bytes on the layer and keep the typed view's
/// `config_descriptor` consistent.
fn store_config(layer: &mut Layer, block_idx: usize, data: Vec<u8>) {
    layer.extra_blocks[block_idx].data = data.clone();
    if let Some(so) = layer.smart_object.as_mut() {
        so.config_descriptor = data;
    }
}

/// Mutable access to the `filterFX` object's items inside a descriptor.
fn filter_fx_mut(desc: &mut DescValue) -> Option<&mut Vec<(Vec<u8>, DescValue)>> {
    let DescValue::Object { items, .. } = desc else {
        return None;
    };
    let DescValue::Object {
        items: filter_fx, ..
    } = get_object_item_mut(items, b"filterFX")?
    else {
        return None;
    };
    Some(filter_fx)
}

/// Mutable access to one `filterFXList` entry's items.
fn filter_entry_mut(desc: &mut DescValue, index: usize) -> Option<&mut Vec<(Vec<u8>, DescValue)>> {
    let filter_fx = filter_fx_mut(desc)?;
    let DescValue::List(list) = get_object_item_mut(filter_fx, b"filterFXList")? else {
        return None;
    };
    let DescValue::Object { items: entry, .. } = list.get_mut(index)? else {
        return None;
    };
    Some(entry)
}

/// Mutable access to the `filterFX.filterFXList` array inside a descriptor.
fn filter_list_mut(desc: &mut DescValue) -> Option<&mut Vec<DescValue>> {
    let filter_fx = filter_fx_mut(desc)?;
    match get_object_item_mut(filter_fx, b"filterFXList") {
        Some(DescValue::List(list)) => Some(list),
        _ => None,
    }
}

/// Drop the whole `filterFX` object from a descriptor; the empty stack has no
/// `filterFXStyle`/`filterFXList` left to author.
fn remove_filter_fx(desc: &mut DescValue) {
    if let DescValue::Object { items, .. } = desc {
        items.retain(|(key, _)| key.as_slice() != b"filterFX");
    }
}

/// Remove the smart filter at `filter_index` from `layer`.
///
/// A preserved `SoLd`/`SoLE` descriptor's `filterFXList` entry is removed and the
/// block rewritten; when the list becomes empty the whole `filterFX` object is
/// dropped. The typed `smart_filters` view is kept in step. Without a preserved
/// block only the typed view changes. An out-of-range index is an error.
pub fn delete_smart_filter(layer: &mut Layer, filter_index: usize) -> Result<(), PsdError> {
    let mut removed = false;
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        match filter_list_mut(&mut desc) {
            Some(list) if filter_index < list.len() => {
                list.remove(filter_index);
                if list.is_empty() {
                    remove_filter_fx(&mut desc);
                }
                removed = true;
            }
            Some(_) => return Err(malformed("filter index out of range")),
            None => return Err(malformed("missing filterFXList")),
        }
        let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
        store_config(layer, block_idx, data);
    }
    if let Some(so) = layer.smart_object.as_mut() {
        if filter_index < so.smart_filters.len() {
            so.smart_filters.remove(filter_index);
            removed = true;
        }
    }
    if !removed {
        return Err(malformed("filter index out of range"));
    }
    Ok(())
}

/// Move the smart filter at `from` to `to` on `layer`.
///
/// Both the preserved `filterFXList` order and the typed `smart_filters` order
/// change, so the applied order (bottom-up on render) follows the panel order.
/// `from == to` and equal indices are a no-op; an out-of-range index errors.
pub fn reorder_smart_filters(layer: &mut Layer, from: usize, to: usize) -> Result<(), PsdError> {
    let len = layer
        .smart_object
        .as_ref()
        .map_or(0, |so| so.smart_filters.len());
    if from >= len || to >= len {
        return Err(malformed("filter index out of range"));
    }
    if from == to {
        return Ok(());
    }
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        match filter_list_mut(&mut desc) {
            Some(list) if from < list.len() && to < list.len() => {
                let entry = list.remove(from);
                list.insert(to, entry);
            }
            Some(_) => return Err(malformed("filter index out of range")),
            None => return Err(malformed("missing filterFXList")),
        }
        let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
        store_config(layer, block_idx, data);
    }
    if let Some(so) = layer.smart_object.as_mut() {
        let entry = so.smart_filters.remove(from);
        so.smart_filters.insert(to, entry);
    }
    Ok(())
}

/// Remove every smart filter from `layer`.
///
/// The preserved `SoLd`/`SoLE` descriptor's whole `filterFX` object is dropped
/// (a stack with no filters authors no descriptor) and the typed view is
/// cleared. The filter mask flags are left for the writer to omit with the rest
/// of `filterFX`.
pub fn clear_smart_filters(layer: &mut Layer) -> Result<(), PsdError> {
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        remove_filter_fx(&mut desc);
        let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
        store_config(layer, block_idx, data);
    }
    if let Some(so) = layer.smart_object.as_mut() {
        so.smart_filters.clear();
    }
    Ok(())
}

/// Set the `enab` flag of the smart filter at `filter_index` on `layer`.
///
/// A preserved `SoLd`/`SoLE` descriptor is rewritten in place (every unmodeled
/// key survives); the typed view is synced in both cases. Without a preserved
/// block only the typed view changes and the writer authors the block on save.
pub fn set_smart_filter_enabled(
    layer: &mut Layer,
    filter_index: usize,
    enabled: bool,
) -> Result<(), PsdError> {
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        if let Some(entry) = filter_entry_mut(&mut desc, filter_index) {
            set_object_item(entry, b"enab", DescValue::Bool(enabled));
            let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
            store_config(layer, block_idx, data);
        }
    }
    if let Some(filter) = layer
        .smart_object
        .as_mut()
        .and_then(|so| so.smart_filters.get_mut(filter_index))
    {
        filter.enabled = enabled;
    }
    Ok(())
}

/// Enable or disable the whole smart-filter group on `layer`.
///
/// A preserved `SoLd`/`SoLE` descriptor's `filterFX.enab` is rewritten in place;
/// the typed view is synced in both cases. Without a preserved block only the
/// typed view changes and the writer authors the block on save.
pub fn set_smart_filters_enabled(layer: &mut Layer, enabled: bool) -> Result<(), PsdError> {
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        if let Some(filter_fx) = filter_fx_mut(&mut desc) {
            set_object_item(filter_fx, b"enab", DescValue::Bool(enabled));
            let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
            store_config(layer, block_idx, data);
        }
    }
    if let Some(so) = layer.smart_object.as_mut() {
        so.smart_filters_enabled = enabled;
    }
    Ok(())
}

/// Enable or disable the filter mask on `layer`.
///
/// A preserved `SoLd`/`SoLE` descriptor's `filterFX.filterMaskEnable` is
/// rewritten in place; the typed view is synced in both cases. Without a
/// preserved block (or before a `filterFX` exists) only the typed view changes
/// and the writer authors the flag with the block.
pub fn set_filter_mask_enabled(layer: &mut Layer, enabled: bool) -> Result<(), PsdError> {
    if let Some(block_idx) = config_block_index(layer) {
        let mut desc = parse_config(layer, block_idx)?;
        if let Some(filter_fx) = filter_fx_mut(&mut desc) {
            set_object_item(filter_fx, b"filterMaskEnable", DescValue::Bool(enabled));
            let data = serialize_config(&layer.extra_blocks[block_idx].data, &desc);
            store_config(layer, block_idx, data);
        }
    }
    if let Some(so) = layer.smart_object.as_mut() {
        so.filter_mask_enabled = enabled;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::get_object_item;
    use pictura_core::{LayerBlock, SmartFilter, SmartObject};
    use std::path::PathBuf;

    fn object(items: Vec<(&[u8], DescValue)>) -> DescValue {
        DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
        }
    }

    fn double(value: DescValue) -> f64 {
        match value {
            DescValue::Double(v) => v,
            other => panic!("expected Double, got {other:?}"),
        }
    }

    #[test]
    fn fixture_camera_raw_option_edit_round_trips() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object02.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter_mut()
            .find(|l| l.name == "Layer 1 copy")
            .expect("Layer 1 copy present");

        let filter = &layer.smart_object.as_ref().unwrap().smart_filters[0];
        assert_eq!(filter.filter_id, 2683);
        let DescValue::Object { items, .. } =
            crate::camera_raw_options(&filter.options).expect("Fltr parses")
        else {
            panic!("Fltr is an object");
        };
        assert!((double(get_object_item(&items, b"Ex12").unwrap().clone()) + 1.15).abs() < 1e-6);

        set_camera_raw_option(layer, 0, b"Ex12", DescValue::Double(2.5)).expect("edit succeeds");

        let written = crate::write_psd(&doc).expect("fixture writes");
        let back = crate::read_psd(&written).expect("written fixture re-reads");
        let so = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object survives a write");
        let filter = &so.smart_filters[0];
        assert_eq!(filter.filter_id, 2683);
        let DescValue::Object { items, .. } =
            crate::camera_raw_options(&filter.options).expect("edited Fltr parses")
        else {
            panic!("Fltr is an object");
        };
        assert_eq!(
            get_object_item(&items, b"Ex12"),
            Some(&DescValue::Double(2.5))
        );
        assert_eq!(get_object_item(&items, b"Dhze"), Some(&DescValue::Long(14)));
        assert_eq!(
            get_object_item(&items, b"Temp"),
            Some(&DescValue::Long(-30))
        );
        assert_eq!(get_object_item(&items, b"Cr12"), Some(&DescValue::Long(12)));
        assert_eq!(get_object_item(&items, b"Vibr"), Some(&DescValue::Long(13)));
    }

    #[test]
    fn fixture_toggle_enab_round_trips_and_syncs_the_typed_view() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object02.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter_mut()
            .find(|l| l.name == "Layer 1 copy")
            .expect("Layer 1 copy present");

        set_smart_filter_enabled(layer, 0, false).expect("toggle the filter");
        set_smart_filters_enabled(layer, false).expect("toggle the group");

        let so = layer.smart_object.as_ref().expect("smart object");
        assert!(
            !so.smart_filters[0].enabled,
            "typed filter follows the edit"
        );
        assert!(!so.smart_filters_enabled, "typed group follows the edit");

        let written = crate::write_psd(&doc).expect("fixture writes");
        let back = crate::read_psd(&written).expect("written fixture re-reads");
        let so = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object survives a write");
        assert!(!so.smart_filters[0].enabled, "per-filter enab persists");
        assert!(!so.smart_filters_enabled, "group enab persists");
    }

    #[test]
    fn toggle_without_a_preserved_block_updates_only_the_typed_view() {
        let mut layer = Layer {
            smart_object: Some({
                let mut so = SmartObject::default();
                so.smart_filters = vec![SmartFilter {
                    filter_id: 2683,
                    name: "Camera Raw Filter".to_string(),
                    enabled: true,
                    options: Vec::new(),
                }];
                so.smart_filters_enabled = true;
                so
            }),
            ..Default::default()
        };
        set_smart_filter_enabled(&mut layer, 0, false).expect("typed toggle");
        set_smart_filters_enabled(&mut layer, false).expect("typed group toggle");
        let so = layer.smart_object.as_ref().unwrap();
        assert!(!so.smart_filters[0].enabled);
        assert!(!so.smart_filters_enabled);
        assert!(
            layer.extra_blocks.is_empty(),
            "no block is authored in memory"
        );
    }

    #[test]
    fn noop_edit_preserves_config_descriptor_bytes() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object02.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter_mut()
            .find(|l| l.name == "Layer 1 copy")
            .expect("Layer 1 copy present");
        let original = layer
            .smart_object
            .as_ref()
            .unwrap()
            .config_descriptor
            .clone();

        set_camera_raw_option(layer, 0, b"Ex12", DescValue::Double(-1.15)).expect("no-op edit");

        let edited = &layer.smart_object.as_ref().unwrap().config_descriptor;
        assert_eq!(
            edited.len(),
            original.len(),
            "a same-size edit keeps the descriptor length"
        );
        for class_id in [
            &b"warp"[..],
            &b"classFloatRect"[..],
            &b"Pnt "[..],
            &b"filterFXStyle"[..],
        ] {
            assert!(
                edited.windows(class_id.len()).any(|w| w == class_id),
                "classID {:?} survives the edit",
                String::from_utf8_lossy(class_id)
            );
        }
    }

    #[test]
    fn set_camera_raw_option_errors_without_sold() {
        let mut layer = Layer::default();
        assert!(
            set_camera_raw_option(&mut layer, 0, b"Ex12", DescValue::Double(1.0)).is_err(),
            "a layer without a smart object block must error, not panic"
        );
    }

    #[test]
    fn edit_accepts_sole_signature_and_preserves_it() {
        let entry = object(vec![
            (b"filterID", DescValue::Long(2683)),
            (b"Fltr", object(vec![(b"Ex12", DescValue::Double(-1.15))])),
        ]);
        let desc = object(vec![(
            b"filterFX",
            object(vec![(b"filterFXList", DescValue::List(vec![entry]))]),
        )]);
        let mut data = b"soLE".to_vec();
        data.extend_from_slice(&4u32.to_be_bytes());
        data.extend_from_slice(&descriptor::write_descriptor(&desc));

        let mut layer = Layer::default();
        layer.extra_blocks.push(LayerBlock {
            key: *b"SoLE",
            data,
        });
        set_camera_raw_option(&mut layer, 0, b"Ex12", DescValue::Double(2.5)).expect("SoLE edit");
        assert_eq!(&layer.extra_blocks[0].data[..4], b"soLE");

        let mut r = Reader::new(&layer.extra_blocks[0].data);
        let _ = r.take(4).unwrap();
        let _ = r.u32().unwrap();
        let DescValue::Object { items, .. } = descriptor::read_descriptor(&mut r).unwrap() else {
            panic!("descriptor object");
        };
        let DescValue::Object {
            items: filter_fx, ..
        } = get_object_item(&items, b"filterFX").unwrap()
        else {
            panic!("filterFX object");
        };
        let DescValue::List(filters) = get_object_item(filter_fx, b"filterFXList").unwrap() else {
            panic!("filterFXList list");
        };
        let DescValue::Object { items: item, .. } = &filters[0] else {
            panic!("filter entry object");
        };
        let DescValue::Object {
            items: fltr_items, ..
        } = get_object_item(item, b"Fltr").unwrap()
        else {
            panic!("Fltr object");
        };
        assert_eq!(
            get_object_item(fltr_items, b"Ex12"),
            Some(&DescValue::Double(2.5))
        );
    }

    #[test]
    fn unknown_signature_is_unsupported() {
        let mut layer = Layer::default();
        layer.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data: b"junk\x00\x00\x00\x00".to_vec(),
        });
        let err =
            set_camera_raw_option(&mut layer, 0, b"Ex12", DescValue::Double(1.0)).unwrap_err();
        assert!(matches!(err, PsdError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn descriptor_dom_round_trips_after_edit() {
        let fltr = DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: vec![
                (b"Keep".to_vec(), DescValue::Long(7)),
                (b"Ex12".to_vec(), DescValue::Double(-1.15)),
                (b"Upri".to_vec(), DescValue::Text("keep".to_string())),
            ],
        };
        let mut parsed =
            crate::camera_raw_options(&descriptor::write_descriptor(&fltr)).expect("parses");
        let DescValue::Object { items, .. } = &mut parsed else {
            panic!("Fltr is an object");
        };
        set_object_item(items, b"Ex12", DescValue::Double(2.5));

        let out = descriptor::write_descriptor(&parsed);
        let DescValue::Object { items, .. } = crate::camera_raw_options(&out).expect("re-parses")
        else {
            panic!("Fltr is an object");
        };
        assert_eq!(
            get_object_item(&items, b"Ex12"),
            Some(&DescValue::Double(2.5))
        );
        assert_eq!(get_object_item(&items, b"Keep"), Some(&DescValue::Long(7)));
        assert_eq!(
            get_object_item(&items, b"Upri"),
            Some(&DescValue::Text("keep".to_string()))
        );
    }

    fn fixture_layer_mut(doc: &mut pictura_core::Document) -> &mut Layer {
        doc.layers
            .iter_mut()
            .find(|l| l.name == "Layer 1 copy")
            .expect("Layer 1 copy present")
    }

    fn filter_ids(layer: &Layer) -> Vec<i32> {
        layer
            .smart_object
            .as_ref()
            .expect("smart object")
            .smart_filters
            .iter()
            .map(|filter| filter.filter_id)
            .collect()
    }

    #[test]
    fn reorder_delete_and_clear_round_trip_on_the_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object02.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        let second = SmartFilter {
            filter_id: 4242,
            name: "Second".to_string(),
            enabled: true,
            options: Vec::new(),
        };
        crate::attach_smart_filter(fixture_layer_mut(&mut doc), second).expect("append second");
        assert_eq!(filter_ids(fixture_layer_mut(&mut doc)), vec![2683, 4242]);

        reorder_smart_filters(fixture_layer_mut(&mut doc), 0, 1).expect("reorder");
        assert_eq!(filter_ids(fixture_layer_mut(&mut doc)), vec![4242, 2683]);

        let written = crate::write_psd(&doc).expect("fixture writes");
        let back = crate::read_psd(&written).expect("written fixture re-reads");
        let so = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object survives");
        let ids: Vec<i32> = so.smart_filters.iter().map(|f| f.filter_id).collect();
        assert_eq!(ids, vec![4242, 2683], "the reordered list persists");

        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        crate::attach_smart_filter(
            fixture_layer_mut(&mut doc),
            SmartFilter {
                filter_id: 4242,
                name: "Second".to_string(),
                enabled: true,
                options: Vec::new(),
            },
        )
        .expect("append second");
        delete_smart_filter(fixture_layer_mut(&mut doc), 0).expect("delete first");
        assert_eq!(filter_ids(fixture_layer_mut(&mut doc)), vec![4242]);
        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("re-reads");
        let ids: Vec<i32> = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("survives")
            .smart_filters
            .iter()
            .map(|f| f.filter_id)
            .collect();
        assert_eq!(ids, vec![4242], "the deleted entry is gone");

        let mut doc = crate::read_psd(&bytes).expect("fixture parses");
        clear_smart_filters(fixture_layer_mut(&mut doc)).expect("clear");
        assert!(filter_ids(fixture_layer_mut(&mut doc)).is_empty());
        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("re-reads");
        let so = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("survives");
        assert!(
            so.smart_filters.is_empty(),
            "a cleared stack authors no filterFX"
        );
    }

    #[test]
    fn out_of_range_delete_and_reorder_are_errors() {
        let mut layer = Layer {
            smart_object: Some({
                let mut so = SmartObject::default();
                so.smart_filters = vec![SmartFilter {
                    filter_id: 2683,
                    name: "only".to_string(),
                    enabled: true,
                    options: Vec::new(),
                }];
                so
            }),
            ..Default::default()
        };
        assert!(delete_smart_filter(&mut layer, 3).is_err());
        assert!(reorder_smart_filters(&mut layer, 0, 2).is_err());
        assert_eq!(filter_ids(&layer), vec![2683], "refusals do not mutate");
    }

    /// A layer carrying a preserved `SoLd` block whose `filterFXList` holds
    /// `list_len` entries (`None` omits the list entirely) and a typed view of
    /// `typed_len` entries.
    fn layer_with_preserved_filter_list(list_len: Option<usize>, typed_len: usize) -> Layer {
        let mut filter_fx: Vec<(&[u8], DescValue)> = Vec::new();
        if let Some(n) = list_len {
            filter_fx.push((b"filterFXList", DescValue::List(vec![object(vec![]); n])));
        }
        let desc = object(vec![(b"filterFX", object(filter_fx))]);
        let mut data = b"soLD".to_vec();
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&descriptor::write_descriptor(&desc));
        Layer {
            extra_blocks: vec![LayerBlock {
                key: *b"SoLd",
                data,
            }],
            smart_object: Some({
                let mut so = SmartObject::default();
                so.smart_filters = (0..typed_len)
                    .map(|_| SmartFilter {
                        filter_id: 2683,
                        name: "f".to_string(),
                        enabled: true,
                        options: Vec::new(),
                    })
                    .collect();
                so
            }),
            ..Default::default()
        }
    }

    #[test]
    fn reorder_refuses_a_preserved_list_that_cannot_follow() {
        // The preserved list is shorter than the typed view.
        let mut short = layer_with_preserved_filter_list(Some(1), 2);
        assert!(reorder_smart_filters(&mut short, 0, 1).is_err());
        assert_eq!(
            filter_ids(&short),
            vec![2683, 2683],
            "the refusal leaves the typed order untouched"
        );

        // The preserved descriptor carries no filterFXList at all.
        let mut missing = layer_with_preserved_filter_list(None, 2);
        assert!(reorder_smart_filters(&mut missing, 0, 1).is_err());
        assert_eq!(filter_ids(&missing), vec![2683, 2683]);
    }

    #[test]
    fn filter_mask_enable_flag_round_trips() {
        let mut layer = pictura_core::Layer {
            name: "L".into(),
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            smart_object: Some(
                {
                    let mut so = SmartObject::default();
                    so.kind = pictura_core::SmartObjectKind::Embedded;
                    so.filename = "src.psd".into();
                    so
                }
                .with_payload(b"8BPS\x00\x01x".to_vec()),
            ),
            ..Default::default()
        };
        let so = layer.smart_object.clone().unwrap();
        let data = crate::smart_writer::author_sold_block(&so, &layer, 2, 2);
        layer.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data,
        });
        crate::append_smart_filter(
            &mut layer,
            SmartFilter {
                filter_id: 4242,
                name: "F".into(),
                enabled: true,
                options: Vec::new(),
            },
        )
        .expect("appends");
        let preserved_flag = |layer: &pictura_core::Layer| {
            let idx = config_block_index(layer).expect("block");
            let mut desc = parse_config(layer, idx).expect("parses");
            match filter_fx_mut(&mut desc)
                .and_then(|fx| get_object_item(fx, b"filterMaskEnable").cloned())
            {
                Some(DescValue::Bool(v)) => v,
                other => panic!("no flag: {other:?}"),
            }
        };
        for enabled in [false, true] {
            set_filter_mask_enabled(&mut layer, enabled).expect("sets");
            assert_eq!(preserved_flag(&layer), enabled);
            let mut doc = pictura_core::Document::new(
                2,
                2,
                pictura_core::ColorMode::Rgb,
                pictura_core::BitDepth::Eight,
            );
            doc.layers = pictura_core::LayerStack::from_layers(vec![layer.clone()]);
            let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("reads");
            let so = back.layers[0].smart_object.as_ref().expect("smart object");
            assert_eq!(so.filter_mask_enabled, enabled);
            assert_eq!(preserved_flag(&back.layers[0]), enabled);
        }
    }
}
