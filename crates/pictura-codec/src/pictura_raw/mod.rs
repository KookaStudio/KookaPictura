//! Pictura Raw: typed read/write of the on-disk camera-raw smart filter.
//!
//! The on-disk form is the reference's standard camera-raw smart filter
//! (`filterID 2683`, name `"Camera Raw Filter"`) inside `SoLd.filterFX`, so
//! the reference can render it; "Pictura Raw" is only our user-facing name.
//!
//! ponytail: behavioural parity only. The `Fltr` short keys and their value
//! types are grounded on a reference Photoshop descriptor; see
//! `docs/dev/camera-raw-cc-notes.md` §Fltr key map. Document-level
//! `FXid`/`FEid`/`FMsk` render caches are not authored (the baked proxy carries
//! the pixels).

use pictura_core::{Layer, PicturaRawSettings, SmartFilter, SmartObjectKind};

use crate::common::Reader;
use crate::descriptor::{self, get_object_item, get_object_item_mut, set_object_item, DescValue};
use crate::error::PsdError;

mod codec;

pub use codec::{decode_pictura_raw_settings, encode_pictura_raw_fltr};

#[cfg(test)]
mod tests;

// The two constants below, the `Fltr` class ID `Adobe Camera Raw Filter`, the
// `Nm  ` value, and `filterID 2683` are the reference's on-disk identifiers for the
// camera-raw smart filter. They are kept byte-for-byte for the reference
// compatibility and used nominatively; see NOTICE.md.
/// The PSD smart-filter id of the camera-raw filter.
pub const CAMERA_RAW_FILTER_ID: i32 = 2683;
/// The filter's display name as stored in `filterFXList[].Nm  `.
pub const CAMERA_RAW_FILTER_NAME: &str = "Camera Raw Filter";

/// The `filterFX` descriptor for a smart object's filter list, or `None` when
/// the list is empty. Used by the writer to author an `SoLd` for a converted
/// smart object and by [`attach_smart_filter`] to insert one.
pub(crate) fn author_filter_fx(
    filters: &[SmartFilter],
    enabled: bool,
    mask_enabled: bool,
    mask_linked: bool,
    extend_with_white: bool,
) -> Option<DescValue> {
    let items: Vec<DescValue> = filters.iter().map(filter_item).collect();
    if items.is_empty() {
        return None;
    }
    Some(object(
        b"filterFXStyle",
        vec![
            (b"enab", DescValue::Bool(enabled)),
            (b"validAtPosition", DescValue::Bool(true)),
            (b"filterMaskEnable", DescValue::Bool(mask_enabled)),
            (b"filterMaskLinked", DescValue::Bool(mask_linked)),
            (
                b"filterMaskExtendWithWhite",
                DescValue::Bool(extend_with_white),
            ),
            (b"filterFXList", DescValue::List(items)),
        ],
    ))
}

fn filter_item(filter: &SmartFilter) -> DescValue {
    let fltr = {
        let mut r = Reader::new(&filter.options);
        descriptor::read_descriptor(&mut r).unwrap_or_else(|_| object(b"null", Vec::new()))
    };
    object(
        b"filterFX",
        vec![
            (b"Nm  ", DescValue::Text(format!("{}\0", filter.name))),
            (
                b"blendOptions",
                object(
                    b"blendOptions",
                    vec![
                        (
                            b"Opct",
                            DescValue::UnitFloat {
                                unit: *b"#Prc",
                                value: 100.0,
                            },
                        ),
                        (
                            b"Md  ",
                            DescValue::Enum {
                                kind: b"BlnM".to_vec(),
                                value: b"Nrml".to_vec(),
                            },
                        ),
                    ],
                ),
            ),
            (b"enab", DescValue::Bool(filter.enabled)),
            (b"hasoptions", DescValue::Bool(true)),
            (
                b"FrgC",
                object(
                    b"HSBC",
                    vec![
                        (b"H   ", DescValue::Double(0.0)),
                        (b"Strt", DescValue::Double(0.0)),
                        (b"Brgh", DescValue::Double(100.0)),
                    ],
                ),
            ),
            (
                b"BckC",
                object(
                    b"RGBC",
                    vec![
                        (b"Rd  ", DescValue::Double(255.0)),
                        (b"Grn ", DescValue::Double(255.0)),
                        (b"Bl  ", DescValue::Double(255.0)),
                    ],
                ),
            ),
            (b"Fltr", fltr),
            (b"filterID", DescValue::Long(filter.filter_id)),
        ],
    )
}

/// Attach `filter` to `layer`'s smart object, replacing an existing filter with
/// the same `filter_id` or appending a new one.
///
/// Three cases, in order:
/// 1. A preserved `SoLd`/`SoLE` block is parsed, its `filterFXList` rewritten,
///    and the layer's typed view kept consistent. Every other descriptor key
///    survives.
/// 2. A smart object with no preserved block records the filter on
///    `SmartObject.smart_filters`; the writer authors the block on save.
/// 3. Anything else is refused without mutating: a layer with no smart object,
///    a non-embedded object, or a legacy `plLd`/`PlLd` placed layer (no `SoLd`
///    to carry a filter and the writer does not author one beside it).
pub fn attach_smart_filter(layer: &mut Layer, filter: SmartFilter) -> Result<(), PsdError> {
    if let Some(block_idx) = config_block_index(layer) {
        return insert_into_config(layer, block_idx, &filter);
    }
    if layer
        .extra_blocks
        .iter()
        .any(|b| matches!(&b.key, b"plLd" | b"PlLd"))
    {
        return Err(PsdError::Unsupported(
            "legacy placed-layer smart object".into(),
        ));
    }
    let Some(so) = layer.smart_object.as_mut() else {
        return Err(malformed("layer has no smart object"));
    };
    if so.kind != SmartObjectKind::Embedded || so.payload.is_none() {
        return Err(malformed("smart object is not an embedded payload"));
    }
    match so
        .smart_filters
        .iter_mut()
        .find(|f| f.filter_id == filter.filter_id)
    {
        Some(existing) => *existing = filter,
        None => so.smart_filters.push(filter),
    }
    Ok(())
}

/// Attach the camera-raw smart filter carrying `settings` to `layer`.
///
/// A thin wrapper over [`attach_smart_filter`] with the camera-raw filter id.
pub fn attach_pictura_raw_filter(
    layer: &mut Layer,
    settings: &PicturaRawSettings,
) -> Result<(), PsdError> {
    attach_smart_filter(layer, new_filter(settings))
}

fn config_block_index(layer: &Layer) -> Option<usize> {
    layer
        .extra_blocks
        .iter()
        .position(|b| &b.key == b"SoLd")
        .or_else(|| layer.extra_blocks.iter().position(|b| &b.key == b"SoLE"))
}

fn new_filter(settings: &PicturaRawSettings) -> SmartFilter {
    SmartFilter {
        filter_id: CAMERA_RAW_FILTER_ID,
        name: CAMERA_RAW_FILTER_NAME.to_string(),
        enabled: true,
        options: descriptor::write_descriptor(&encode_pictura_raw_fltr(settings)),
    }
}

/// The group flags the writer authors for a fresh `filterFX` on `layer`:
/// `(enabled, mask_enabled, mask_linked, extend_with_white)`.
fn group_flags(layer: &Layer) -> (bool, bool, bool, bool) {
    layer
        .smart_object
        .as_ref()
        .map(|so| {
            (
                so.smart_filters_enabled,
                so.filter_mask_enabled,
                so.filter_mask_linked,
                so.filter_mask_extend_with_white,
            )
        })
        .unwrap_or((true, true, false, true))
}

/// The `filterID` of a `filterFXList` entry, or `None` when absent.
fn entry_filter_id(entry: &DescValue) -> Option<i32> {
    let DescValue::Object { items, .. } = entry else {
        return None;
    };
    match get_object_item(items, b"filterID") {
        Some(DescValue::Long(id)) => Some(*id),
        _ => None,
    }
}

/// Merge a newly attached filter into an existing `filterFXList` entry: overlay
/// the new `Fltr` keys onto the entry's preserved `Fltr` (new values win), refresh
/// `Nm  `/`enab`, and keep `blendOptions` and every other unmodeled key.
fn merge_filter_entry(entry: &mut DescValue, filter: &SmartFilter) {
    let DescValue::Object { items, .. } = entry else {
        *entry = filter_item(filter);
        return;
    };
    let new_fltr = {
        let mut r = Reader::new(&filter.options);
        descriptor::read_descriptor(&mut r).unwrap_or_else(|_| object(b"null", Vec::new()))
    };
    match get_object_item_mut(items, b"Fltr") {
        Some(DescValue::Object {
            items: existing_fltr,
            ..
        }) => {
            if let DescValue::Object {
                items: new_items, ..
            } = new_fltr
            {
                for (key, value) in new_items {
                    set_object_item(existing_fltr, &key, value);
                }
            }
        }
        Some(slot) => *slot = new_fltr,
        None => set_object_item(items, b"Fltr", new_fltr),
    }
    set_object_item(
        items,
        b"Nm  ",
        DescValue::Text(format!("{}\0", filter.name)),
    );
    set_object_item(items, b"enab", DescValue::Bool(filter.enabled));
}

/// Insert or replace the entry in a preserved `SoLd`/`SoLE` descriptor,
/// rewriting the block and the layer's typed view consistently.
fn insert_into_config(
    layer: &mut Layer,
    block_idx: usize,
    filter: &SmartFilter,
) -> Result<(), PsdError> {
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
    let DescValue::Object { items, .. } = &mut desc else {
        return Err(malformed("descriptor is not an object"));
    };
    match get_object_item_mut(items, b"filterFX") {
        Some(DescValue::Object {
            items: filter_fx, ..
        }) => match get_object_item_mut(filter_fx, b"filterFXList") {
            Some(DescValue::List(list)) => {
                match list
                    .iter_mut()
                    .find(|entry| entry_filter_id(entry) == Some(filter.filter_id))
                {
                    Some(entry) => merge_filter_entry(entry, filter),
                    None => list.push(filter_item(filter)),
                }
            }
            _ => return Err(malformed("filterFXList is not a list")),
        },
        Some(_) => return Err(malformed("filterFX is not an object")),
        None => {
            let (enabled, mask_enabled, linked, extend) = group_flags(layer);
            set_object_item(
                items,
                b"filterFX",
                author_filter_fx(
                    std::slice::from_ref(filter),
                    enabled,
                    mask_enabled,
                    linked,
                    extend,
                )
                .ok_or_else(|| malformed("no filter to author"))?,
            );
        }
    }

    let mut new_data = data[..8].to_vec();
    new_data.extend_from_slice(&descriptor::write_descriptor(&desc));
    // psd-tools reads layer tagged blocks without consuming a trailing pad, so
    // the declared length must be even; the reference 4-aligns the SoLd descriptor.
    while !new_data.len().is_multiple_of(4) {
        new_data.push(0);
    }
    layer.extra_blocks[block_idx].data = new_data.clone();
    if let Some(so) = layer.smart_object.as_mut() {
        so.config_descriptor = new_data;
        match so
            .smart_filters
            .iter_mut()
            .find(|f| f.filter_id == filter.filter_id)
        {
            Some(existing) => *existing = filter.clone(),
            None => so.smart_filters.push(filter.clone()),
        }
    }
    Ok(())
}

pub(crate) fn object(class_id: &[u8], items: Vec<(&[u8], DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class_id.to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    }
}

fn malformed(what: &str) -> PsdError {
    PsdError::Invalid(format!("Pictura Raw filter descriptor: {what}"))
}
