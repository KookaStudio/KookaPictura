//! Smart-object resolution: link a layer's config descriptor to its
//! document-level linked-layer record by `uuid`.
//!
//! The preserved bytes (`Layer.extra_blocks`, `Document.layer_section_extra`)
//! remain the source of truth for re-emission; everything here derives a typed
//! view and degrades to [`SmartObjectKind::Unresolved`] rather than failing a
//! file whose smart-object data is malformed but preserved.

use pictura_core::{Layer, LayerBlock, LayerStack, SmartFilter, SmartObject, SmartObjectKind};

use crate::common::{is_psb_big_key, Reader};
use crate::descriptor::DescValue;
use crate::descriptor::{self, get_object_item, read_unicode_string, skip_descriptor_block};
use crate::error::PsdError;

/// One record from a document-level `lnkD`/`lnk2`/`lnk3`/`lnkE` list.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkedRecord {
    kind: [u8; 4],
    version: u32,
    uuid: String,
    filename: String,
    filetype: [u8; 4],
    creator: [u8; 4],
    payload: Option<Vec<u8>>,
}

/// Resolve smart objects for `layers` (and their children) from the preserved
/// document-level linked-layer bytes. Never fails: an unparseable list or
/// descriptor leaves the layer unresolved.
pub(crate) fn resolve_smart_objects(
    layers: &mut LayerStack,
    layer_section_extra: &[u8],
    is_psb: bool,
) {
    let records = collect_linked_records(layer_section_extra, is_psb);
    let masks = crate::filter_effects::collect_filter_masks(layer_section_extra, is_psb);
    for layer in layers.iter_mut() {
        resolve_layer(layer, &records, &masks);
    }
}

fn resolve_layer(
    layer: &mut Layer,
    records: &[LinkedRecord],
    masks: &[crate::filter_effects::FilterMaskRecord],
) {
    layer.smart_object = build_smart_object(layer, records, masks);
    for child in layer.children.iter_mut() {
        resolve_layer(child, records, masks);
    }
}

fn build_smart_object(
    layer: &Layer,
    records: &[LinkedRecord],
    masks: &[crate::filter_effects::FilterMaskRecord],
) -> Option<SmartObject> {
    let block = find_config_block(&layer.extra_blocks)?;
    let raw = block.data.clone();
    // `uuid` links the layer to its linked-source record (`Idnt`); `placed` is
    // the second SoLd uuid, which the document-level filter effect (`FEid`) is
    // keyed by. Photoshop writes them differently; we author them equal.
    let (uuid, placed, smart_filters, flags) = match &block.key {
        b"SoLd" | b"SoLE" => {
            let DescValue::Object { items, .. } = read_layer_data_descriptor(&raw).ok()? else {
                return None;
            };
            let text = |key: &[u8]| match get_object_item(&items, key) {
                Some(DescValue::Text(id)) => Some(id.trim_end_matches('\0').to_string()),
                _ => None,
            };
            let uuid = text(b"Idnt")?;
            let placed = text(b"placed").unwrap_or_else(|| uuid.clone());
            let (filters, flags) = parse_filter_fx(&items);
            (uuid, placed, filters, flags)
        }
        b"plLd" | b"PlLd" => {
            let uuid = read_placed_layer_uuid(&raw).ok()?;
            (uuid.clone(), uuid, Vec::new(), FilterFxFlags::default())
        }
        _ => return None,
    };

    let mut so = {
        let mut so = SmartObject::default();
        so.uuid = uuid.clone();
        so.placed_uuid = placed.clone();
        so.config_descriptor = raw;
        so.smart_filters = smart_filters;
        so.filter_mask_enabled = flags.mask_enabled;
        so.filter_mask_linked = flags.mask_linked;
        so.filter_mask_extend_with_white = flags.extend_with_white;
        so.smart_filters_enabled = flags.enabled;
        so
    };
    if let Some(mask) = masks.iter().find(|m| m.uuid == placed || m.uuid == uuid) {
        so.filter_mask = Some(pictura_core::FilterMask {
            rect: mask.rect,
            data: mask.coverage.clone().into(),
        });
    }
    let Some(record) = records.iter().find(|r| r.uuid == uuid) else {
        return Some(so);
    };

    so.filename = record.filename.trim_end_matches('\0').to_string();
    so.filetype = record.filetype;
    so.creator = record.creator;
    match &record.kind {
        b"liFD" => {
            so.kind = SmartObjectKind::Embedded;
            so.crs_xmp = record.payload.as_deref().and_then(extract_crs);
            so.crs = so.crs_xmp.as_deref().map(crate::crs_xmp::parse_crs);
            if let Some(payload) = record.payload.clone() {
                so.set_payload(payload);
            }
        }
        b"liFE" => so.kind = SmartObjectKind::External,
        b"liFA" => so.kind = SmartObjectKind::Alias,
        _ => so.kind = SmartObjectKind::Unresolved,
    }
    Some(so)
}

/// The group-level flags carried by a smart object's `filterFXStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilterFxFlags {
    enabled: bool,
    mask_enabled: bool,
    mask_linked: bool,
    extend_with_white: bool,
}

impl Default for FilterFxFlags {
    fn default() -> Self {
        // Match the writer's defaults for a missing `filterFXStyle`.
        Self {
            enabled: true,
            mask_enabled: true,
            mask_linked: false,
            extend_with_white: true,
        }
    }
}

/// Build the typed `SmartFilter` list from a parsed `SoLd`/`SoLE` descriptor:
/// `filterFX` (Object) -> `filterFXList` (List of Objects) -> one entry each,
/// plus the group-level `filterMask*` flags on the `filterFX` object.
fn parse_filter_fx(items: &[(Vec<u8>, DescValue)]) -> (Vec<SmartFilter>, FilterFxFlags) {
    let Some(DescValue::Object {
        items: filter_fx, ..
    }) = get_object_item(items, b"filterFX")
    else {
        return (Vec::new(), FilterFxFlags::default());
    };
    let flags = FilterFxFlags {
        enabled: bool_item(filter_fx, b"enab", true),
        mask_enabled: bool_item(filter_fx, b"filterMaskEnable", true),
        mask_linked: bool_item(filter_fx, b"filterMaskLinked", false),
        extend_with_white: bool_item(filter_fx, b"filterMaskExtendWithWhite", true),
    };
    let Some(DescValue::List(list)) = get_object_item(filter_fx, b"filterFXList") else {
        return (Vec::new(), flags);
    };
    let filters = list
        .iter()
        .filter_map(|value| {
            let DescValue::Object { items: item, .. } = value else {
                return None;
            };
            let filter_id = match get_object_item(item, b"filterID") {
                Some(DescValue::Long(id)) => *id,
                _ => 0,
            };
            let name =
                match get_object_item(item, b"Nm  ").or_else(|| get_object_item(item, b"name")) {
                    Some(DescValue::Text(name)) => name.trim_end_matches('\0').to_string(),
                    _ => String::new(),
                };
            let enabled = match get_object_item(item, b"enab") {
                Some(DescValue::Bool(enabled)) => *enabled,
                _ => true,
            };
            let options = match get_object_item(item, b"Fltr") {
                Some(value) => descriptor::write_descriptor(value),
                None => Vec::new(),
            };
            Some(SmartFilter {
                filter_id,
                name,
                enabled,
                options,
            })
        })
        .collect();
    (filters, flags)
}

/// Read a bool item, falling back to `default` when missing or the wrong type.
fn bool_item(items: &[(Vec<u8>, DescValue)], key: &[u8], default: bool) -> bool {
    match get_object_item(items, key) {
        Some(DescValue::Bool(value)) => *value,
        _ => default,
    }
}

/// The config tagged block, preferring the modern descriptors over the legacy
/// placed-layer one (matching psd-tools' lookup order).
fn find_config_block(blocks: &[LayerBlock]) -> Option<&LayerBlock> {
    [b"SoLd", b"SoLE", b"plLd", b"PlLd"]
        .into_iter()
        .find_map(|key| blocks.iter().find(|b| &b.key == key))
}

// ---------------------------------------------------------------------------
// Document-level linked-layer list
// ---------------------------------------------------------------------------

/// Parse every linked record in the preserved top-level tagged blocks. On any
/// malformed block, keep what parsed so far instead of failing the file.
fn collect_linked_records(layer_section_extra: &[u8], is_psb: bool) -> Vec<LinkedRecord> {
    let mut records = Vec::new();
    let mut r = Reader::new(layer_section_extra);
    while r.remaining() >= 12 {
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" {
            break;
        }
        let Ok(key) = r.take(4) else { break };
        let len = if is_psb && is_psb_big_key(&arr4(key)) {
            let Ok(len) = r.u64() else { break };
            usize::try_from(len).unwrap_or(usize::MAX)
        } else {
            let Ok(len) = r.u32() else { break };
            len as usize
        };
        let Ok(data) = r.take(len) else {
            break;
        };
        // A global tagged block is padded externally to a 4-byte boundary.
        if r.skip((4 - len % 4) % 4).is_err() {
            break;
        }
        if matches!(key, b"lnkD" | b"lnk2" | b"lnk3" | b"lnkE") {
            if let Ok(mut parsed) = parse_linked_layers(data) {
                records.append(&mut parsed);
            }
        }
    }
    records
}

/// Parse a linked-layer list: a sequence of `u64`-length-prefixed records,
/// each padded to a 4-byte boundary.
fn parse_linked_layers(data: &[u8]) -> Result<Vec<LinkedRecord>, PsdError> {
    let mut r = Reader::new(data);
    let mut out = Vec::new();
    while r.remaining() >= 8 {
        let len = r.u64()? as usize;
        let block = r.take(len)?;
        let pad = (4 - len % 4) % 4;
        r.skip(pad)?;
        out.push(parse_linked_layer(block)?);
    }
    Ok(out)
}

fn parse_linked_layer(data: &[u8]) -> Result<LinkedRecord, PsdError> {
    let mut r = Reader::new(data);
    let kind = arr4(r.take(4)?);
    let version = r.u32()?;
    if !(1..=8).contains(&version) {
        return Err(PsdError::Unsupported(format!(
            "linked layer version {version}"
        )));
    }
    let uuid = read_pascal_string(&mut r)?;
    let filename = read_unicode_string(&mut r)?;
    let filetype = arr4(r.take(4)?);
    let creator = arr4(r.take(4)?);
    let datasize = r.u64()? as usize;
    let open_file = r.u8()? != 0;
    if open_file {
        skip_descriptor_block(&mut r)?;
    }

    let mut payload = None;
    match &kind {
        b"liFD" => payload = Some(r.take(datasize)?.to_vec()),
        b"liFE" => {
            skip_descriptor_block(&mut r)?; // linked_file descriptor
            if version > 3 {
                r.skip(16)?; // timestamp: u32 + 4 bytes + f64
            }
            let _filesize = r.u64()?;
            if version > 2 {
                let _ = r.take(datasize)?;
            }
        }
        b"liFA" => r.skip(8)?,
        _ => {}
    }
    if version >= 5 {
        let _child_id = read_unicode_string(&mut r)?;
    }
    if version >= 6 {
        r.skip(8)?; // mod_time
    }
    if version >= 7 {
        r.skip(1)?; // lock_state
    }
    if &kind == b"liFE" && version == 2 {
        let _ = r.take(datasize)?;
    }
    Ok(LinkedRecord {
        kind,
        version,
        uuid,
        filename,
        filetype,
        creator,
        payload,
    })
}

/// Rebuild the preserved top-level tagged blocks without the linked-source
/// record(s) whose Pascal uuid equals `uuid`.
///
/// Every non-matched block is copied byte-for-byte (padding included); a
/// matched `lnkD`/`lnk2`/`lnk3`/`lnkE` block is re-emitted with the same key, a
/// recomputed exact length, and external padding to a 4-byte boundary. A
/// malformed block is copied verbatim.
/// Returns `Some` only when at least one record was removed, `None` otherwise
/// (so the caller's bytes are left untouched).
pub fn remove_linked_source(
    layer_section_extra: &[u8],
    uuid: &str,
    is_psb: bool,
) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(layer_section_extra.len());
    let mut removed = false;
    let mut r = Reader::new(layer_section_extra);
    while r.remaining() >= 12 {
        let start = r.pos;
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" {
            r.pos = start;
            break;
        }
        let Ok(key) = r.take(4) else {
            r.pos = start;
            break;
        };
        let big = is_psb && is_psb_big_key(&arr4(key));
        let len = if big {
            let Ok(len) = r.u64() else {
                r.pos = start;
                break;
            };
            usize::try_from(len).unwrap_or(usize::MAX)
        } else {
            let Ok(len) = r.u32() else {
                r.pos = start;
                break;
            };
            len as usize
        };
        let Ok(data) = r.take(len) else {
            r.pos = start;
            break;
        };
        // A global tagged block is padded externally to a 4-byte boundary.
        if r.skip((4 - len % 4) % 4).is_err() {
            r.pos = start;
            break;
        }
        let end = r.pos;
        if matches!(key, b"lnkD" | b"lnk2" | b"lnk3" | b"lnkE") {
            if let Some(kept) = remove_linked_records(data, uuid) {
                crate::write::write_tag_document(&mut out, &arr4(key), &kept, is_psb);
                removed = true;
                continue;
            }
        }
        out.extend_from_slice(&layer_section_extra[start..end]);
    }
    if r.remaining() > 0 {
        out.extend_from_slice(&layer_section_extra[r.pos..]);
    }
    removed.then_some(out)
}

/// Drop every record whose uuid equals `uuid` from a linked-layer list, keeping
/// the surviving records' exact bytes. `None` when the list is malformed or
/// nothing matched, so the caller can copy the whole block verbatim.
fn remove_linked_records(data: &[u8], uuid: &str) -> Option<Vec<u8>> {
    let records = parse_linked_layers(data).ok()?;
    if !records.iter().any(|record| record.uuid == uuid) {
        return None;
    }
    let mut r = Reader::new(data);
    let mut out = Vec::with_capacity(data.len());
    for record in &records {
        let start = r.pos;
        let len = r.u64().ok()? as usize;
        r.take(len).ok()?;
        let pad = (4 - len % 4) % 4;
        r.skip(pad).ok()?;
        if record.uuid != uuid {
            out.extend_from_slice(&data[start..r.pos]);
        }
    }
    if r.remaining() > 0 {
        out.extend_from_slice(&data[r.pos..]);
    }
    Some(out)
}

/// Replace the payload of the `liFD` record whose Pascal uuid equals `uuid` in
/// the preserved top-level tagged blocks, keeping every other byte verbatim
/// (padding and framing included). Returns `Some` only when a record's payload
/// was rewritten, `None` otherwise.
pub(crate) fn replace_embedded_payload(
    layer_section_extra: &[u8],
    uuid: &str,
    new_payload: &[u8],
    is_psb: bool,
) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(layer_section_extra.len());
    let mut changed = false;
    let mut r = Reader::new(layer_section_extra);
    while r.remaining() >= 12 {
        let start = r.pos;
        let Ok(sig) = r.take(4) else { break };
        if sig != b"8BIM" {
            r.pos = start;
            break;
        }
        let Ok(key) = r.take(4) else {
            r.pos = start;
            break;
        };
        let big = is_psb && is_psb_big_key(&arr4(key));
        let len = if big {
            let Ok(len) = r.u64() else {
                r.pos = start;
                break;
            };
            usize::try_from(len).unwrap_or(usize::MAX)
        } else {
            let Ok(len) = r.u32() else {
                r.pos = start;
                break;
            };
            len as usize
        };
        let Ok(data) = r.take(len) else {
            r.pos = start;
            break;
        };
        if r.skip((4 - len % 4) % 4).is_err() {
            r.pos = start;
            break;
        }
        let end = r.pos;
        if matches!(key, b"lnkD" | b"lnk2" | b"lnk3" | b"lnkE") {
            if let Some(kept) = replace_linked_payload(data, uuid, new_payload) {
                crate::write::write_tag_document(&mut out, &arr4(key), &kept, is_psb);
                changed = true;
                continue;
            }
        }
        out.extend_from_slice(&layer_section_extra[start..end]);
    }
    if r.remaining() > 0 {
        out.extend_from_slice(&layer_section_extra[r.pos..]);
    }
    changed.then_some(out)
}

/// Replace the payload of the matched `liFD` record inside a linked-layer list,
/// re-framing that record's `u64` length and 4-byte pad. Other records are
/// copied byte-for-byte. `None` when the list is malformed or no `liFD` record
/// has the uuid.
fn replace_linked_payload(data: &[u8], uuid: &str, new_payload: &[u8]) -> Option<Vec<u8>> {
    let records = parse_linked_layers(data).ok()?;
    if !records
        .iter()
        .any(|record| record.uuid == uuid && record.kind == *b"liFD")
    {
        return None;
    }
    let mut r = Reader::new(data);
    let mut out = Vec::with_capacity(data.len() + new_payload.len());
    for record in &records {
        let start = r.pos;
        let len = r.u64().ok()? as usize;
        let block = r.take(len).ok()?;
        let pad = (4 - len % 4) % 4;
        r.skip(pad).ok()?;
        let end = r.pos;
        if record.uuid == uuid && record.kind == *b"liFD" {
            let new_block = splice_payload(block, new_payload)?;
            out.extend_from_slice(&(new_block.len() as u64).to_be_bytes());
            out.extend_from_slice(&new_block);
            let pad = (4 - new_block.len() % 4) % 4;
            out.extend(std::iter::repeat_n(0u8, pad));
        } else {
            out.extend_from_slice(&data[start..end]);
        }
    }
    if r.remaining() > 0 {
        out.extend_from_slice(&data[r.pos..]);
    }
    Some(out)
}

/// Replace the payload span of one `liFD` record block, updating its `u64`
/// size field; the uuid/filename/type headers and version trailer stay verbatim.
fn splice_payload(block: &[u8], new_payload: &[u8]) -> Option<Vec<u8>> {
    let mut r = Reader::new(block);
    r.take(4).ok()?;
    r.u32().ok()?;
    read_pascal_string(&mut r).ok()?;
    read_unicode_string(&mut r).ok()?;
    r.take(4).ok()?;
    r.take(4).ok()?;
    let size_off = r.pos;
    let datasize = r.u64().ok()? as usize;
    if r.u8().ok()? != 0 {
        skip_descriptor_block(&mut r).ok()?;
    }
    let payload_start = r.pos;
    if payload_start + datasize > block.len() {
        return None;
    }
    let mut out = Vec::with_capacity(block.len() - datasize + new_payload.len());
    out.extend_from_slice(&block[..size_off]);
    out.extend_from_slice(&(new_payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&block[size_off + 8..payload_start]);
    out.extend_from_slice(new_payload);
    out.extend_from_slice(&block[payload_start + datasize..]);
    Some(out)
}

// ---------------------------------------------------------------------------
// Layer config descriptors
// ---------------------------------------------------------------------------

/// `SoLd`/`SoLE`: `soLD` + outer version + a version-16 `DescriptorBlock`.
fn read_layer_data_descriptor(data: &[u8]) -> Result<DescValue, PsdError> {
    let mut r = Reader::new(data);
    let _kind = r.take(4)?;
    let _version = r.u32()?;
    descriptor::read_descriptor(&mut r)
}

/// Legacy `plLd`/`PlLd`: `plcL` + version + a Pascal (MacRoman) uuid.
fn read_placed_layer_uuid(data: &[u8]) -> Result<String, PsdError> {
    let mut r = Reader::new(data);
    let _kind = r.take(4)?;
    let _version = r.u32()?;
    read_pascal_string(&mut r)
}

/// Pascal string (length byte + bytes). MacRoman high bytes are approximated as
/// lossy UTF-8, which is exact for the ASCII uuids and filenames here.
fn read_pascal_string(r: &mut Reader) -> Result<String, PsdError> {
    let len = r.u8()? as usize;
    let data = r.take(len)?;
    Ok(String::from_utf8_lossy(data).into_owned())
}

fn arr4(s: &[u8]) -> [u8; 4] {
    let mut a = [0u8; 4];
    a.copy_from_slice(&s[..4]);
    a
}

/// Return the XMP packet from an embedded payload when it carries `crs:`
/// settings, else `None`.
fn extract_crs(payload: &[u8]) -> Option<Vec<u8>> {
    let start = find_subslice(payload, b"<x:xmpmeta")?;
    let rest = &payload[start..];
    let len = find_subslice(rest, b"</x:xmpmeta>")
        .map(|i| i + b"</x:xmpmeta>".len())
        .or_else(|| find_subslice(rest, b"/>").map(|i| i + 2))?;
    let packet = &rest[..len];
    find_subslice(packet, b"crs:").map(|_| packet.to_vec())
}

/// Naive substring search; payloads are ~1 MiB and this runs once per embedded
/// object, so a two-pointer scan is plenty.
pub(crate) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::CrsSettings;
    use std::path::PathBuf;

    fn pascal(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut v = vec![b.len() as u8];
        v.extend_from_slice(b);
        v
    }

    fn unicode(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut v = (units.len() as u32).to_be_bytes().to_vec();
        for unit in units {
            v.extend_from_slice(&unit.to_be_bytes());
        }
        v
    }

    fn empty_descriptor() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&16u32.to_be_bytes());
        d.extend_from_slice(&0u32.to_be_bytes()); // name
        d.extend_from_slice(&0u32.to_be_bytes()); // classID key length
        d.extend_from_slice(b"null");
        d.extend_from_slice(&0u32.to_be_bytes()); // item count
        d
    }

    fn plld(uuid: &str) -> Vec<u8> {
        let mut v = b"plcL".to_vec();
        v.extend_from_slice(&3u32.to_be_bytes());
        v.extend_from_slice(&pascal(uuid));
        v
    }

    fn linked_layer(
        kind: &[u8; 4],
        version: u32,
        uuid: &str,
        filename: &str,
        data: &[u8],
    ) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(kind);
        v.extend_from_slice(&version.to_be_bytes());
        v.extend_from_slice(&pascal(uuid));
        v.extend_from_slice(&unicode(filename));
        v.extend_from_slice(b"8BPB");
        v.extend_from_slice(b"8BIM");
        v.extend_from_slice(&(data.len() as u64).to_be_bytes());
        v.push(0); // no open_file descriptor
        match kind {
            b"liFD" => v.extend_from_slice(data),
            b"liFE" => {
                v.extend_from_slice(&empty_descriptor());
                if version > 3 {
                    v.extend_from_slice(&[0u8; 16]);
                }
                v.extend_from_slice(&0u64.to_be_bytes());
            }
            b"liFA" => v.extend_from_slice(&[0u8; 8]),
            _ => {}
        }
        if version >= 5 {
            v.extend_from_slice(&unicode("\0"));
        }
        if version >= 6 {
            v.extend_from_slice(&0f64.to_be_bytes());
        }
        if version >= 7 {
            v.push(0);
        }
        v
    }

    /// A document-level tagged block: exact length, padded externally to 4.
    fn tagged(key: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = b"8BIM".to_vec();
        v.extend_from_slice(key);
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        v.extend_from_slice(&vec![0u8; (4 - data.len() % 4) % 4]);
        v
    }

    fn linked_list(blocks: &[Vec<u8>]) -> Vec<u8> {
        let mut v = Vec::new();
        for b in blocks {
            v.extend_from_slice(&(b.len() as u64).to_be_bytes());
            v.extend_from_slice(b);
            v.extend_from_slice(&vec![0u8; (4 - b.len() % 4) % 4]);
        }
        v
    }

    fn config_layer(uuid: &str) -> Layer {
        Layer {
            name: "L".into(),
            extra_blocks: vec![LayerBlock {
                key: *b"PlLd",
                data: plld(uuid),
            }],
            ..Default::default()
        }
    }

    fn fnv1a(bytes: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        h
    }

    #[test]
    fn external_record_is_reported_without_embedded_payload() {
        let uuid = "11111111-2222-3333-4444-555555555555";
        let record = linked_layer(b"liFE", 3, uuid, "linked.psd", &[]);
        let section = tagged(b"lnk2", &linked_list(&[record]));

        let mut layers = LayerStack::from_layers(vec![config_layer(uuid)]);
        resolve_smart_objects(&mut layers, &section, false);
        let so = layers[0].smart_object.as_ref().expect("resolved");
        assert_eq!(so.kind, SmartObjectKind::External);
        assert!(so.payload().is_none(), "external payload is not exposed");
        assert_eq!(so.filename, "linked.psd");
    }

    #[test]
    fn config_without_matching_record_is_unresolved() {
        let mut layers =
            LayerStack::from_layers(vec![config_layer("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")]);
        resolve_smart_objects(&mut layers, &tagged(b"lnk2", &linked_list(&[])), false);
        let so = layers[0].smart_object.as_ref().expect("resolved");
        assert_eq!(so.kind, SmartObjectKind::Unresolved);
        assert_eq!(so.uuid, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        assert!(so.payload().is_none());
    }

    #[test]
    fn embedded_record_exposes_payload() {
        let uuid = "99999999-8888-7777-6666-555555555555";
        let record = linked_layer(b"liFD", 7, uuid, "Layer 1.psb", b"8BPS\x00\x02raw");
        let section = tagged(b"lnk2", &linked_list(&[record]));

        let mut layers = LayerStack::from_layers(vec![config_layer(uuid)]);
        resolve_smart_objects(&mut layers, &section, false);
        let so = layers[0].smart_object.as_ref().expect("resolved");
        assert_eq!(so.kind, SmartObjectKind::Embedded);
        assert_eq!(so.filename, "Layer 1.psb");
        assert_eq!(so.filetype, *b"8BPB");
        assert_eq!(so.creator, *b"8BIM");
        assert_eq!(so.payload(), Some(&b"8BPS\x00\x02raw"[..]));
    }

    #[test]
    fn embedded_payload_with_crs_xmp_is_exposed() {
        let uuid = "12345678-1234-1234-1234-123456789abc";
        let xmp = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description \
crs:Exposure2012=\"+0.50\"/></rdf:RDF></x:xmpmeta>";
        let record = linked_layer(b"liFD", 7, uuid, "raw.psb", xmp);
        let section = tagged(b"lnk2", &linked_list(&[record]));

        let mut layers = LayerStack::from_layers(vec![config_layer(uuid)]);
        resolve_smart_objects(&mut layers, &section, false);
        let so = layers[0].smart_object.as_ref().expect("resolved");
        assert_eq!(so.kind, SmartObjectKind::Embedded);
        let crs = so.crs_xmp.as_ref().expect("crs xmp is exposed");
        assert!(
            crs.windows(4).any(|window| window == b"crs:"),
            "the packet carries a crs: property"
        );
        assert_eq!(
            so.crs,
            Some(CrsSettings {
                exposure: Some(0.5),
                ..Default::default()
            })
        );
    }

    #[test]
    fn malformed_record_degrades_without_panic() {
        assert!(parse_linked_layers(&[0, 0, 0, 0, 0, 0, 0, 5, 1, 2, 3]).is_err());
        let section = tagged(b"lnk2", &[0, 0, 0, 0, 0, 0, 0, 5, 1, 2, 3]);
        assert!(collect_linked_records(&section, false).is_empty());
    }

    #[test]
    fn remove_linked_source_drops_one_record_and_preserves_the_rest() {
        let keep = "11111111-2222-3333-4444-555555555555";
        let drop = "99999999-8888-7777-6666-555555555555";
        let record_keep = linked_layer(b"liFD", 7, keep, "keep.psb", b"keep");
        let record_drop = linked_layer(b"liFD", 7, drop, "drop.psb", b"drop!");
        let unrelated = tagged(b"abcd", b"unrelated-bytes");
        let section = [
            tagged(b"lnk2", &linked_list(&[record_keep.clone(), record_drop])),
            unrelated.clone(),
        ]
        .concat();

        let cleaned = remove_linked_source(&section, drop, false).expect("a record was removed");
        let expected = [tagged(b"lnk2", &linked_list(&[record_keep])), unrelated].concat();
        assert_eq!(
            cleaned, expected,
            "survivor and unrelated block are byte-preserved"
        );

        assert!(
            !cleaned.windows(drop.len()).any(|w| w == drop.as_bytes()),
            "the removed uuid no longer appears"
        );
        let records = collect_linked_records(&cleaned, false);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].uuid, keep);

        assert_eq!(remove_linked_source(&section, "no-such-uuid", false), None);
    }

    #[test]
    fn remove_linked_source_copies_a_malformed_block_verbatim() {
        let drop = "99999999-8888-7777-6666-555555555555";
        let malformed = tagged(b"lnk2", &[0, 0, 0, 0, 0, 0, 0, 5, 1, 2, 3]);
        let good = tagged(
            b"lnk2",
            &linked_list(&[linked_layer(b"liFD", 7, drop, "drop.psb", b"drop")]),
        );
        let section = [malformed.clone(), good].concat();

        let cleaned = remove_linked_source(&section, drop, false).expect("the good block changed");
        assert!(
            cleaned.starts_with(&malformed),
            "a malformed block is copied byte-for-byte"
        );
    }

    #[test]
    fn fixture_smart_object_resolves_embedded_payload() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object01.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let doc = crate::read_psd(&bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == "Layer 1")
            .expect("Layer 1 present");
        let so = layer.smart_object.as_ref().expect("smart object resolved");
        assert_eq!(so.kind, SmartObjectKind::Embedded);
        assert!(!so.uuid.is_empty(), "authored uuid is present");
        assert_eq!(
            so.uuid,
            crate::smart_writer::author_uuid(so),
            "the uuid is the deterministic function of filename + payload"
        );
        assert_eq!(so.filename, "Layer 1.psb");
        assert_eq!(&so.filetype, b"8BPB");
        assert_eq!(&so.creator, b"8BIM");
        assert!(!so.config_descriptor.is_empty());
        assert!(so.smart_filters.is_empty(), "fixture 01 has no filterFX");

        let payload = so.payload().expect("embedded payload");
        assert_eq!(&payload[..4], b"8BPS");
        assert_eq!(payload[4], 0);
        assert_eq!(payload[5], 2, "version 2 embedded source");
        assert!(payload.len() > 8, "payload is a real (minimal) PSB");
        let payload_hash = fnv1a(payload);
        assert!(so.crs_xmp.is_none(), "fixture XMP carries no crs: property");

        // The preserved blocks must still re-emit and re-resolve: writing the
        // read document keeps the object intact byte-for-byte.
        let written = crate::write_psd(&doc).expect("fixture writes");
        let back = crate::read_psd(&written).expect("written fixture re-reads");
        let so2 = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object survives a write");
        assert_eq!(so2.uuid, so.uuid);
        assert_eq!(so2.kind, SmartObjectKind::Embedded);
        assert_eq!(fnv1a(so2.payload().unwrap()), payload_hash);
    }

    #[test]
    fn fixture_config_descriptor_survives_write() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object01.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let doc = crate::read_psd(&bytes).expect("fixture parses");
        let original = doc
            .layers
            .iter()
            .find(|l| l.name == "Layer 1")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object resolved")
            .config_descriptor
            .clone();

        let back = crate::read_psd(&crate::write_psd(&doc).expect("fixture writes"))
            .expect("written fixture re-reads");
        let reread = back
            .layers
            .iter()
            .find(|l| l.name == "Layer 1")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object survives a write")
            .config_descriptor
            .clone();
        assert_eq!(reread, original, "config descriptor bytes are unchanged");
    }

    #[test]
    fn fixture_camera_raw_filter_options_parse() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/test_with_smart_object02.psd");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} not found", path.display());
            return;
        };
        let doc = crate::read_psd(&bytes).expect("fixture parses");
        let so = doc
            .layers
            .iter()
            .find(|l| l.name == "Layer 1 copy")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object resolved");

        assert_eq!(so.smart_filters.len(), 1, "exactly one smart filter");
        assert!(so.smart_filters_enabled, "the group is enabled by default");
        let filter = &so.smart_filters[0];
        assert_eq!(filter.filter_id, 2683);
        assert_eq!(filter.name, "Camera Raw Filter");
        assert!(filter.enabled);
        assert!(!filter.options.is_empty());

        let DescValue::Object { items, .. } =
            crate::camera_raw_options(&filter.options).expect("Fltr parses")
        else {
            panic!("Fltr is an object");
        };
        assert!(matches!(
            get_object_item(&items, b"Ex12"),
            Some(DescValue::Double(_))
        ));
        assert_eq!(get_object_item(&items, b"Cr12"), Some(&DescValue::Long(12)));
        assert_eq!(
            get_object_item(&items, b"Temp"),
            Some(&DescValue::Long(-30))
        );
        assert_eq!(get_object_item(&items, b"Vibr"), Some(&DescValue::Long(13)));
        assert_eq!(get_object_item(&items, b"Dhze"), Some(&DescValue::Long(14)));
    }
}
