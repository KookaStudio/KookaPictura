//! Edit a layer's smart-filter stack.
//!
//! This module owns the consistency protocol for a preserved `SoLd`/`SoLE`
//! smart-filter config: locate and validate the block, parse the descriptor,
//! navigate and mutate `filterFX`, re-serialize onto the block's original
//! 8-byte prefix with 4-byte padding, and store the bytes on both the tagged
//! block and `SmartObject.config_descriptor`. Every edit keeps the typed
//! `SmartObject.smart_filters` view in step. A layer with no preserved block
//! edits the typed view only and the writer authors the block on save.
//!
//! The preserved bytes stay the source of truth for save; unmodeled descriptor
//! keys survive every edit. Filter-specific option encoding stays in its
//! adapter (`pictura_raw` for the Camera Raw `Fltr`, `lens_correction`,
//! `pictura_filter`).

use pictura_core::{Layer, LayerParts, SmartFilter, SmartObjectKind};

use crate::common::Reader;
use crate::descriptor::{self, get_object_item, get_object_item_mut, set_object_item, DescValue};
use crate::error::PsdError;

/// One `filterFXList` entry's descriptor items.
type Items = Vec<(Vec<u8>, DescValue)>;

/// The group flags a fresh `filterFXStyle` is authored with:
/// `(enabled, mask_enabled, mask_linked, extend_with_white)`.
type FilterFxFlags = (bool, bool, bool, bool);

/// A parsed preserved `SoLd`/`SoLE` config block.
struct Preserved {
    block: usize,
    header: [u8; 8],
    desc: DescValue,
}

/// An open edit of a layer's preserved smart-filter config.
///
/// `preserved` is `None` for typed-only storage; `commit` is then a no-op and
/// the writer authors the block on save.
struct Edit<'a> {
    layer: &'a mut LayerParts,
    preserved: Option<Preserved>,
}

impl<'a> Edit<'a> {
    /// Locate, validate and parse the preserved config, or open a typed-only
    /// edit when the layer has no `SoLd`/`SoLE` block.
    fn open(layer: &'a mut LayerParts) -> Result<Self, PsdError> {
        let Some(block) = config_block_index(layer) else {
            return Ok(Self {
                layer,
                preserved: None,
            });
        };
        let data = &layer.extra_blocks[block].data;
        let mut r = Reader::new(data);
        let signature = r.take(4)?;
        if signature != b"soLD" && signature != b"soLE" {
            return Err(PsdError::Unsupported(format!(
                "smart object layer data signature {:?}",
                String::from_utf8_lossy(signature)
            )));
        }
        let mut header = [0u8; 8];
        header[..4].copy_from_slice(signature);
        header[4..].copy_from_slice(&r.u32()?.to_be_bytes());
        let desc = descriptor::read_descriptor(&mut r)?;
        Ok(Self {
            layer,
            preserved: Some(Preserved {
                block,
                header,
                desc,
            }),
        })
    }

    fn has_preserved(&self) -> bool {
        self.preserved.is_some()
    }

    /// The `filterFXList` of a preserved config; errors when it is missing.
    fn list_mut(&mut self) -> Result<&mut Vec<DescValue>, PsdError> {
        let Some(preserved) = self.preserved.as_mut() else {
            return Err(no_config());
        };
        filter_list_mut(&mut preserved.desc).ok_or_else(|| malformed("missing filterFXList"))
    }

    /// The `filterFXList`, authoring an empty `filterFXStyle` when the
    /// descriptor has no `filterFX` yet (insert path only).
    fn list_or_author_mut(&mut self) -> Result<&mut Vec<DescValue>, PsdError> {
        let flags = group_flags(self.layer);
        let Some(preserved) = self.preserved.as_mut() else {
            return Err(no_config());
        };
        author_empty_filter_fx(&mut preserved.desc, flags)?;
        filter_list_mut(&mut preserved.desc).ok_or_else(|| malformed("filterFXList is not a list"))
    }

    /// One `filterFXList` entry's items; an out-of-range index is an error.
    fn entry_mut(&mut self, index: usize) -> Result<&mut Items, PsdError> {
        let Some(preserved) = self.preserved.as_mut() else {
            return Err(no_config());
        };
        filter_entry_mut(&mut preserved.desc, index)
            .ok_or_else(|| malformed("filter index out of range"))
    }

    /// One `filterFXList` entry's value, whatever its type.
    fn entry_value_mut(&mut self, index: usize) -> Result<&mut DescValue, PsdError> {
        let Some(preserved) = self.preserved.as_mut() else {
            return Err(no_config());
        };
        filter_entry_value_mut(&mut preserved.desc, index)
            .ok_or_else(|| malformed("filter index out of range"))
    }

    /// The `filterFX` object's items, when present.
    fn fx_mut(&mut self) -> Option<&mut Items> {
        filter_fx_mut(&mut self.preserved.as_mut()?.desc)
    }

    /// Drop the whole `filterFX` object; the empty stack authors nothing.
    fn drop_filter_fx(&mut self) {
        if let Some(preserved) = self.preserved.as_mut() {
            remove_filter_fx(&mut preserved.desc);
        }
    }

    /// Serialize the edited descriptor onto its original prefix and store it on
    /// both the tagged block and the typed mirror.
    fn commit(self) -> Result<(), PsdError> {
        let Some(preserved) = self.preserved else {
            return Ok(());
        };
        let data = serialize_config(&preserved.header, &preserved.desc);
        self.layer.extra_blocks[preserved.block].data = data.clone().into();
        if let Some(so) = self.layer.smart_object.as_mut() {
            so.config_descriptor = data.into();
        }
        Ok(())
    }
}

fn malformed(what: &str) -> PsdError {
    PsdError::Invalid(format!("smart filter config descriptor: {what}"))
}

fn no_config() -> PsdError {
    malformed("layer has no SoLd/SoLE smart object block")
}

/// The index of the preserved `SoLd`/`SoLE` config block, preferring `SoLd`.
fn config_block_index(layer: &LayerParts) -> Option<usize> {
    layer
        .extra_blocks
        .iter()
        .position(|b| &b.key == b"SoLd")
        .or_else(|| layer.extra_blocks.iter().position(|b| &b.key == b"SoLE"))
}

/// Serialize a descriptor onto a block header (signature + outer version),
/// zero-padding to a 4-byte boundary.
///
/// psd-tools reads layer tagged blocks without consuming a trailing pad, so the
/// declared length must be even. The reference 4-aligns the `SoLd` block data
/// after the descriptor, which also keeps a same-size edit's length.
// ponytail: zero pad to 4; revisit if a reader rejects the trailing bytes.
pub(crate) fn serialize_config(header: &[u8], desc: &DescValue) -> Vec<u8> {
    let mut data = header.to_vec();
    data.extend_from_slice(&descriptor::write_descriptor(desc));
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
    data
}

/// Mutable access to the `filterFX` object's items inside a descriptor.
fn filter_fx_mut(desc: &mut DescValue) -> Option<&mut Items> {
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

/// Mutable access to one `filterFXList` entry's value.
fn filter_entry_value_mut(desc: &mut DescValue, index: usize) -> Option<&mut DescValue> {
    let filter_fx = filter_fx_mut(desc)?;
    let DescValue::List(list) = get_object_item_mut(filter_fx, b"filterFXList")? else {
        return None;
    };
    list.get_mut(index)
}

/// Mutable access to one `filterFXList` entry's items.
fn filter_entry_mut(desc: &mut DescValue, index: usize) -> Option<&mut Items> {
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

/// The `filterFX` descriptor for a smart object's filter list, or `None` when
/// the list is empty. Used by the writer to author an `SoLd` for a converted
/// smart object and by the insert path to author a fresh stack.
pub(crate) fn author_filter_fx(
    filters: &[SmartFilter],
    enabled: bool,
    mask_enabled: bool,
    mask_linked: bool,
    extend_with_white: bool,
) -> Option<DescValue> {
    if filters.is_empty() {
        return None;
    }
    Some(filter_fx_style(
        filters,
        enabled,
        mask_enabled,
        mask_linked,
        extend_with_white,
    ))
}

fn filter_fx_style(
    filters: &[SmartFilter],
    enabled: bool,
    mask_enabled: bool,
    mask_linked: bool,
    extend_with_white: bool,
) -> DescValue {
    let items: Vec<DescValue> = filters.iter().map(filter_item).collect();
    object(
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
    )
}

/// Author an empty `filterFXStyle` when the descriptor has none yet. An
/// existing `filterFX` is left untouched, and a non-object one is an error.
fn author_empty_filter_fx(desc: &mut DescValue, flags: FilterFxFlags) -> Result<(), PsdError> {
    let (enabled, mask_enabled, mask_linked, extend_with_white) = flags;
    let DescValue::Object { items, .. } = desc else {
        return Err(malformed("descriptor is not an object"));
    };
    match get_object_item_mut(items, b"filterFX") {
        Some(DescValue::Object { .. }) => Ok(()),
        Some(_) => Err(malformed("filterFX is not an object")),
        None => {
            set_object_item(
                items,
                b"filterFX",
                filter_fx_style(&[], enabled, mask_enabled, mask_linked, extend_with_white),
            );
            Ok(())
        }
    }
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

/// The group flags the writer authors for a fresh `filterFX` on `layer`.
fn group_flags(layer: &LayerParts) -> FilterFxFlags {
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

pub(crate) fn object(class_id: &[u8], items: Vec<(&[u8], DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class_id.to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    }
}

/// Replace (or insert) one key in the `Fltr` options of the filter at
/// `filter_index` on `layer`, rewriting the preserved `SoLd`/`SoLE` block in
/// place and keeping the typed `options` in step.
fn set_camera_raw_option_parts(
    layer: &mut LayerParts,
    filter_index: usize,
    key: &[u8],
    value: DescValue,
) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    let options = {
        let entry = edit.entry_mut(filter_index)?;
        let DescValue::Object {
            items: fltr_items, ..
        } = get_object_item_mut(entry, b"Fltr").ok_or_else(|| malformed("missing Fltr"))?
        else {
            return Err(malformed("Fltr is not an object"));
        };
        set_object_item(fltr_items, key, value);
        descriptor::write_descriptor(
            get_object_item(entry, b"Fltr").expect("the Fltr object was just checked"),
        )
    };
    edit.commit()?;
    if let Some(filter) = layer
        .smart_object
        .as_mut()
        .and_then(|so| so.smart_filters.get_mut(filter_index))
    {
        filter.options = options.into();
    }
    Ok(())
}

/// Remove the smart filter at `filter_index` from `layer`.
///
/// A preserved `SoLd`/`SoLE` descriptor's `filterFXList` entry is removed and the
/// block rewritten; when the list becomes empty the whole `filterFX` object is
/// dropped. The typed `smart_filters` view is kept in step. Without a preserved
/// block only the typed view changes. An out-of-range index is an error.
fn delete_smart_filter_parts(layer: &mut LayerParts, filter_index: usize) -> Result<(), PsdError> {
    let mut removed = false;
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        let drop_filter_fx;
        {
            let list = edit.list_mut()?;
            if filter_index >= list.len() {
                return Err(malformed("filter index out of range"));
            }
            list.remove(filter_index);
            drop_filter_fx = list.is_empty();
        }
        if drop_filter_fx {
            edit.drop_filter_fx();
        }
        edit.commit()?;
        removed = true;
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
fn reorder_smart_filters_parts(
    layer: &mut LayerParts,
    from: usize,
    to: usize,
) -> Result<(), PsdError> {
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
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        {
            let list = edit.list_mut()?;
            if from >= list.len() || to >= list.len() {
                return Err(malformed("filter index out of range"));
            }
            let entry = list.remove(from);
            list.insert(to, entry);
        }
        edit.commit()?;
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
fn clear_smart_filters_parts(layer: &mut LayerParts) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        edit.drop_filter_fx();
        edit.commit()?;
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
fn set_smart_filter_enabled_parts(
    layer: &mut LayerParts,
    filter_index: usize,
    enabled: bool,
) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        let mut wrote = false;
        if let Ok(entry) = edit.entry_mut(filter_index) {
            set_object_item(entry, b"enab", DescValue::Bool(enabled));
            wrote = true;
        }
        if wrote {
            edit.commit()?;
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
fn set_smart_filters_enabled_parts(layer: &mut LayerParts, enabled: bool) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        let mut wrote = false;
        if let Some(filter_fx) = edit.fx_mut() {
            set_object_item(filter_fx, b"enab", DescValue::Bool(enabled));
            wrote = true;
        }
        if wrote {
            edit.commit()?;
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
fn set_filter_mask_enabled_parts(layer: &mut LayerParts, enabled: bool) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        let mut wrote = false;
        if let Some(filter_fx) = edit.fx_mut() {
            set_object_item(filter_fx, b"filterMaskEnable", DescValue::Bool(enabled));
            wrote = true;
        }
        if wrote {
            edit.commit()?;
        }
    }
    if let Some(so) = layer.smart_object.as_mut() {
        so.filter_mask_enabled = enabled;
    }
    Ok(())
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
    crate::stage_content::try_edit_layer(layer, |parts| attach(parts, filter, true))
}

/// As [`attach_smart_filter`], but always appends a new entry, even when one
/// with the same `filter_id` exists, to both the preserved `filterFXList` and
/// the typed `smart_filters`, so their indices stay aligned.
pub fn append_smart_filter(layer: &mut Layer, filter: SmartFilter) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| attach(parts, filter, false))
}

/// The shared attach; `replace` merges into a same-id entry instead of
/// appending.
fn attach(layer: &mut LayerParts, filter: SmartFilter, replace: bool) -> Result<(), PsdError> {
    if config_block_index(layer).is_some() {
        return insert_into_config(layer, &filter, replace);
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
    if so.kind != SmartObjectKind::Embedded || so.payload().is_none() {
        return Err(malformed("smart object is not an embedded payload"));
    }
    match so
        .smart_filters
        .iter_mut()
        .find(|f| replace && f.filter_id == filter.filter_id)
    {
        Some(existing) => *existing = filter,
        None => so.smart_filters.push(filter),
    }
    Ok(())
}

/// Insert or replace the entry in a preserved `SoLd`/`SoLE` descriptor,
/// rewriting the block and the layer's typed view consistently.
fn insert_into_config(
    layer: &mut LayerParts,
    filter: &SmartFilter,
    replace: bool,
) -> Result<(), PsdError> {
    let mut edit = Edit::open(layer)?;
    {
        let list = edit.list_or_author_mut()?;
        match list
            .iter_mut()
            .find(|entry| replace && entry_filter_id(entry) == Some(filter.filter_id))
        {
            Some(entry) => merge_filter_entry(entry, filter),
            None => list.push(filter_item(filter)),
        }
    }
    edit.commit()?;
    if let Some(so) = layer.smart_object.as_mut() {
        match so
            .smart_filters
            .iter_mut()
            .find(|f| replace && f.filter_id == filter.filter_id)
        {
            Some(existing) => *existing = filter.clone(),
            None => so.smart_filters.push(filter.clone()),
        }
    }
    Ok(())
}

/// Replace the smart filter at `index` on `layer` with `filter`, in place: the
/// entry keeps its position, its enable flag, and (in a preserved
/// `SoLd`/`SoLE`) its `blendOptions` and unmodeled keys, while `Fltr`, `Nm  `,
/// and `filterID` take the new values. An out-of-range index is an error and
/// leaves the layer unchanged.
fn replace_smart_filter_parts(
    layer: &mut LayerParts,
    index: usize,
    mut filter: SmartFilter,
) -> Result<(), PsdError> {
    let Some(existing) = layer
        .smart_object
        .as_ref()
        .and_then(|so| so.smart_filters.get(index))
    else {
        return Err(malformed("filter index out of range"));
    };
    filter.enabled = existing.enabled;
    let mut edit = Edit::open(layer)?;
    if edit.has_preserved() {
        {
            let entry = edit.entry_value_mut(index)?;
            // A different `Fltr` class is replaced whole, so no key of the old
            // record lingers in the new one.
            if let DescValue::Object { items, .. } = entry {
                items.retain(|(key, _)| key.as_slice() != b"Fltr");
                set_object_item(items, b"filterID", DescValue::Long(filter.filter_id));
            }
            merge_filter_entry(entry, &filter);
        }
        edit.commit()?;
    }
    if let Some(slot) = layer
        .smart_object
        .as_mut()
        .and_then(|so| so.smart_filters.get_mut(index))
    {
        *slot = filter;
    }
    Ok(())
}

pub fn set_camera_raw_option(
    layer: &mut Layer,
    filter_index: usize,
    key: &[u8],
    value: DescValue,
) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        set_camera_raw_option_parts(parts, filter_index, key, value)
    })
}

pub fn delete_smart_filter(layer: &mut Layer, filter_index: usize) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        delete_smart_filter_parts(parts, filter_index)
    })
}

pub fn reorder_smart_filters(layer: &mut Layer, from: usize, to: usize) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        reorder_smart_filters_parts(parts, from, to)
    })
}

pub fn clear_smart_filters(layer: &mut Layer) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, clear_smart_filters_parts)
}

pub fn set_smart_filter_enabled(
    layer: &mut Layer,
    filter_index: usize,
    enabled: bool,
) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        set_smart_filter_enabled_parts(parts, filter_index, enabled)
    })
}

pub fn set_smart_filters_enabled(layer: &mut Layer, enabled: bool) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        set_smart_filters_enabled_parts(parts, enabled)
    })
}

pub fn set_filter_mask_enabled(layer: &mut Layer, enabled: bool) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        set_filter_mask_enabled_parts(parts, enabled)
    })
}

pub fn replace_smart_filter(
    layer: &mut Layer,
    index: usize,
    filter: SmartFilter,
) -> Result<(), PsdError> {
    crate::stage_content::try_edit_layer(layer, |parts| {
        replace_smart_filter_parts(parts, index, filter)
    })
}

#[cfg(test)]
mod tests;
