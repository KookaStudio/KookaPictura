//! The document-level filter mask: the `FEid` (`FilterEffects`) coverage and
//! the `FMsk` display block. Both live in `Document::layer_section_extra` and
//! are preserved verbatim; this module reads the coverage, can drop a mask's
//! blocks, and authors a mask on save. The reference's rendered RGB channels in
//! `FEid` are not modelled.
//!
//! The format is grounded on a Photoshop-authored fixture (#956), used only as
//! a black-box reference; see the change design.

use pictura_core::{FilterMask, Layer, PsdRect, SmartObject};

use crate::common::{
    is_psb_big_key, Reader, COMPRESSION_RAW, COMPRESSION_ZIP, COMPRESSION_ZIP_PREDICTION,
};
use crate::depth::undo_prediction;
use crate::error::PsdError;

/// One smart object's filter-mask coverage.
pub(crate) struct FilterMaskRecord {
    pub(crate) uuid: String,
    pub(crate) rect: PsdRect,
    pub(crate) coverage: Vec<u8>,
}

/// The `FEid` coverage records in `layer_section_extra`.
pub(crate) fn collect_filter_masks(
    layer_section_extra: &[u8],
    is_psb: bool,
) -> Vec<FilterMaskRecord> {
    let mut out = Vec::new();
    for_each_block(layer_section_extra, is_psb, |key, data| {
        if &key == b"FEid" {
            if let Ok(mut records) = parse_filter_effects(data) {
                out.append(&mut records);
            }
        }
    });
    out
}

/// Remove the filter mask of the smart object whose uuid is `uuid` from the
/// preserved global tagged blocks, keeping every other block byte-for-byte.
///
/// Drops the matching `FEid` entry, and the `FMsk` block when that entry was
/// the only one. Returns `None` (leaving the bytes untouched) when nothing
/// matches or the `FEid` block is malformed.
pub fn remove_filter_mask(layer_section_extra: &[u8], uuid: &str, is_psb: bool) -> Option<Vec<u8>> {
    let mut matches = false;
    let mut entries = 0usize;
    for_each_block(layer_section_extra, is_psb, |key, data| {
        if &key == b"FEid" {
            if let Some((_, records)) = split_filter_effects(data) {
                entries += records.len();
                matches |= records.iter().any(|(id, _)| id == uuid);
            }
        }
    });
    if !matches {
        return None;
    }
    let sole_entry = entries == 1;

    let mut out = Vec::with_capacity(layer_section_extra.len());
    let mut r = Reader::new(layer_section_extra);
    while let Some((key, start, end, data)) = next_block(&mut r, is_psb) {
        if &key == b"FEid" {
            if sole_entry {
                continue;
            }
            if let Some((version, records)) = split_filter_effects(data) {
                let survivors: Vec<&Vec<u8>> = records
                    .iter()
                    .filter(|(id, _)| id != uuid)
                    .map(|(_, raw)| raw)
                    .collect();
                if survivors.len() != records.len() {
                    let mut data = version.to_be_bytes().to_vec();
                    for raw in survivors {
                        data.extend_from_slice(raw);
                    }
                    crate::write::write_tag_document(&mut out, b"FEid", &data, is_psb);
                    continue;
                }
            }
        } else if &key == b"FMsk" && sole_entry {
            continue;
        }
        out.extend_from_slice(&layer_section_extra[start..end]);
    }
    if r.remaining() > 0 {
        out.extend_from_slice(&layer_section_extra[r.pos..]);
    }
    Some(out)
}

/// `extra` (the output's document-level tagged blocks, framed for `psb`) with
/// every smart object's `filter_mask` authored into `FEid`.
///
/// An entry whose rect and coverage already equal the mask is kept verbatim; a
/// changed one is replaced and a missing one appended, every other entry kept.
/// An `FMsk` block is added only when none exists. `extra` is returned
/// unchanged when nothing needs authoring or its `FEid` is malformed.
///
/// ponytail: the entry carries no rendered colour cache and the `FMsk` colour
/// and opacity are unverified defaults; Photoshop has not opened the result.
pub(crate) fn author_filter_masks(
    extra: &[u8],
    layers: &[Layer],
    color_channels: u32,
    psb: bool,
) -> Vec<u8> {
    let mut wanted: Vec<(String, &FilterMask)> = Vec::new();
    collect_wanted(layers, &mut wanted);
    if wanted.is_empty() {
        return extra.to_vec();
    }

    let mut existing: Option<FilterEffectEntries> = None;
    let mut has_fmsk = false;
    let mut malformed = false;
    for_each_block(extra, psb, |key, data| match &key {
        b"FEid" if existing.is_none() => match split_filter_effects(data) {
            Some(split) => existing = Some(split),
            None => malformed = true,
        },
        b"FMsk" => has_fmsk = true,
        _ => {}
    });
    if malformed {
        return extra.to_vec();
    }
    let had_feid = existing.is_some();
    let (version, mut entries) = existing.unwrap_or((3, Vec::new()));
    let mut changed = false;
    for (uuid, mask) in wanted {
        let Some(raw) = encode_entry(&uuid, mask, color_channels) else {
            continue;
        };
        match entries.iter_mut().find(|(id, _)| *id == uuid) {
            Some((_, old)) if same_mask(old, mask) => {}
            Some((_, old)) => {
                *old = raw;
                changed = true;
            }
            None => {
                entries.push((uuid, raw));
                changed = true;
            }
        }
    }
    if !changed {
        return extra.to_vec();
    }
    let mut feid = version.to_be_bytes().to_vec();
    for (_, raw) in &entries {
        feid.extend_from_slice(raw);
    }

    let mut out = Vec::with_capacity(extra.len() + feid.len() + 32);
    let mut r = Reader::new(extra);
    let mut replaced = false;
    while let Some((key, start, end, _)) = next_block(&mut r, psb) {
        if &key == b"FEid" && !replaced {
            crate::write::write_tag_document(&mut out, b"FEid", &feid, psb);
            replaced = true;
            continue;
        }
        out.extend_from_slice(&extra[start..end]);
    }
    if !had_feid {
        crate::write::write_tag_document(&mut out, b"FEid", &feid, psb);
    }
    if !has_fmsk {
        crate::write::write_tag_document(&mut out, b"FMsk", &FMSK_DEFAULT, psb);
    }
    out.extend_from_slice(&extra[r.pos..]);
    out
}

/// The `FMsk` block authored beside a new mask: colour space RGB, red, then
/// 50% opacity (psd-tools' `FilterMask` layout). Unverified defaults.
const FMSK_DEFAULT: [u8; 12] = [0, 0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 50];

/// The `(uuid, mask)` of every masked smart object under `layers`, keyed as
/// the re-read object will be: the authored uuid when its `SoLd` is authored
/// this save, else the `placed` uuid, falling back to `Idnt`.
fn collect_wanted<'a>(layers: &'a [Layer], out: &mut Vec<(String, &'a FilterMask)>) {
    for layer in layers {
        if let Some(so) = layer.smart_object.as_ref() {
            if let Some(mask) = so.filter_mask.as_ref() {
                let uuid = mask_key(layer, so);
                if !uuid.is_empty() && !out.iter().any(|(id, _)| *id == uuid) {
                    out.push((uuid, mask));
                }
            }
        }
        collect_wanted(&layer.children, out);
    }
}

fn mask_key(layer: &Layer, so: &SmartObject) -> String {
    if crate::smart_writer::should_author(layer).is_some() {
        crate::smart_writer::author_uuid(so)
    } else if so.placed_uuid.is_empty() {
        so.uuid.clone()
    } else {
        so.placed_uuid.clone()
    }
}

/// Whether a raw `FEid` entry (length prefix, body, padding) already holds
/// `mask`.
fn same_mask(raw: &[u8], mask: &FilterMask) -> bool {
    let Some(len) = raw
        .get(..8)
        .and_then(|b| usize::try_from(u64::from_be_bytes(b.try_into().ok()?)).ok())
    else {
        return false;
    };
    raw.get(8..8 + len)
        .and_then(|body| parse_filter_effect(body).ok())
        .is_some_and(|record| record.rect == mask.rect && record.coverage[..] == mask.data[..])
}

/// One raw `FEid` entry for `mask`: the colour channels unwritten and the
/// coverage, ZIP-with-prediction, in the last channel, as the reader expects.
/// `None` when the mask's plane does not match its rect.
fn encode_entry(uuid: &str, mask: &FilterMask, color_channels: u32) -> Option<Vec<u8>> {
    let (w, h) = (
        mask.rect.width().max(0) as usize,
        mask.rect.height().max(0) as usize,
    );
    if w == 0 || h == 0 || mask.data.len() != w * h || uuid.len() > 255 {
        return None;
    }
    let mut channel = COMPRESSION_ZIP_PREDICTION.to_be_bytes().to_vec();
    channel.extend(crate::write::zip_scanlines(&[&mask.data[..]], w, h, true, 8).ok()?);

    let mut body = Vec::new();
    for v in [
        mask.rect.top,
        mask.rect.left,
        mask.rect.bottom,
        mask.rect.right,
    ] {
        body.extend_from_slice(&v.to_be_bytes());
    }
    body.extend_from_slice(&8u32.to_be_bytes());
    body.extend_from_slice(&color_channels.to_be_bytes());
    for _ in 0..=color_channels {
        body.extend_from_slice(&0u32.to_be_bytes());
    }
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&(channel.len() as u64).to_be_bytes());
    body.extend_from_slice(&channel);

    let mut entry = vec![uuid.len() as u8];
    entry.extend_from_slice(uuid.as_bytes());
    entry.extend_from_slice(&1u32.to_be_bytes());
    entry.extend_from_slice(&(body.len() as u64).to_be_bytes());
    entry.extend_from_slice(&body);

    let mut raw = (entry.len() as u64).to_be_bytes().to_vec();
    raw.extend_from_slice(&entry);
    raw.extend(std::iter::repeat_n(0u8, (4 - entry.len() % 4) % 4));
    Some(raw)
}

/// Walk the global tagged blocks, calling `f` with each `(key, data)`.
fn for_each_block(bytes: &[u8], is_psb: bool, mut f: impl FnMut([u8; 4], &[u8])) {
    let mut r = Reader::new(bytes);
    while let Some((key, _, _, data)) = next_block(&mut r, is_psb) {
        f(key, data);
    }
}

/// Read the next global tagged block, returning `(key, start, end, data)`.
/// Stops (returns `None`) on a malformed block, leaving the rest to the caller.
fn next_block<'a>(r: &mut Reader<'a>, is_psb: bool) -> Option<([u8; 4], usize, usize, &'a [u8])> {
    if r.remaining() < 12 {
        return None;
    }
    let start = r.pos;
    let sig = r.take(4).ok()?;
    if sig != b"8BIM" {
        r.pos = start;
        return None;
    }
    let key: [u8; 4] = r.take(4).ok()?.try_into().ok()?;
    let len = if is_psb && is_psb_big_key(&key) {
        usize::try_from(r.u64().ok()?).unwrap_or(usize::MAX)
    } else {
        r.u32().ok()? as usize
    };
    let data = match r.take(len) {
        Ok(data) => data,
        Err(_) => {
            r.pos = start;
            return None;
        }
    };
    if r.skip((4 - len % 4) % 4).is_err() {
        r.pos = start;
        return None;
    }
    Some((key, start, r.pos, data))
}

/// An `FEid` payload split into its version and each entry as `(uuid, raw
/// bytes)`, the raw bytes being the entry's `u64` length prefix, data, and
/// padding.
type FilterEffectEntries = (u32, Vec<(String, Vec<u8>)>);

/// Split an `FEid` payload into its version and each entry as `(uuid, raw
/// bytes)`, the raw bytes being the entry's `u64` length prefix, data, and
/// padding.
fn split_filter_effects(data: &[u8]) -> Option<FilterEffectEntries> {
    let mut r = Reader::new(data);
    let version = r.u32().ok()?;
    let mut entries = Vec::new();
    while r.remaining() >= 8 {
        let start = r.pos;
        let len = usize::try_from(r.u64().ok()?).ok()?;
        let body = r.take(len).ok()?;
        r.skip((4 - len % 4) % 4).ok()?;
        entries.push((entry_uuid(body)?, data[start..r.pos].to_vec()));
    }
    Some((version, entries))
}

/// The Pascal uuid at the start of an `FEid` entry body.
fn entry_uuid(entry: &[u8]) -> Option<String> {
    let mut r = Reader::new(entry);
    let n = r.u8().ok()? as usize;
    Some(String::from_utf8_lossy(r.take(n).ok()?).into_owned())
}

/// Parse every `FEid` entry, keeping the coverage of the last channel.
fn parse_filter_effects(data: &[u8]) -> Result<Vec<FilterMaskRecord>, PsdError> {
    let mut r = Reader::new(data);
    let version = r.u32()?;
    if !(1..=3).contains(&version) {
        return Err(PsdError::Unsupported("FilterEffects version".into()));
    }
    let mut out = Vec::new();
    while r.remaining() >= 8 {
        let len = usize::try_from(r.u64()?).unwrap_or(usize::MAX);
        let entry = r.take(len)?;
        r.skip((4 - len % 4) % 4)?;
        if let Ok(record) = parse_filter_effect(entry) {
            out.push(record);
        }
    }
    Ok(out)
}

fn parse_filter_effect(entry: &[u8]) -> Result<FilterMaskRecord, PsdError> {
    let mut r = Reader::new(entry);
    let n = r.u8()? as usize;
    let uuid = String::from_utf8_lossy(r.take(n)?).into_owned();
    let _version = r.u32()?;
    let body_len = usize::try_from(r.u64()?).unwrap_or(usize::MAX);
    let body = r.take(body_len)?;

    let mut b = Reader::new(body);
    let top = b.i32()?;
    let left = b.i32()?;
    let bottom = b.i32()?;
    let right = b.i32()?;
    let depth = b.u32()?;
    let max_channels = b.u32()?;
    let count = (max_channels as usize)
        .checked_add(2)
        .ok_or_else(|| PsdError::Invalid("filter effect channels".into()))?;
    let mut mask_channel: Option<Vec<u8>> = None;
    for i in 0..count {
        let written = b.u32()?;
        if written == 0 {
            continue;
        }
        let len = usize::try_from(b.u64()?).unwrap_or(usize::MAX);
        let data = b.take(len)?;
        if i + 1 == count {
            mask_channel = Some(data.to_vec());
        }
    }
    let rect = PsdRect {
        top,
        left,
        bottom,
        right,
    };
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let channel = mask_channel.ok_or_else(|| PsdError::Invalid("filter mask channel".into()))?;
    let coverage = decode_channel(&channel, w, h, depth)?;
    Ok(FilterMaskRecord {
        uuid,
        rect,
        coverage,
    })
}

/// Decode a filter-effect channel (`u16 compression` + payload) to `w * h`
/// samples. Only the compressions a mask uses are supported.
fn decode_channel(data: &[u8], w: usize, h: usize, depth: u32) -> Result<Vec<u8>, PsdError> {
    let mut r = Reader::new(data);
    let compression = r.u16()?;
    let rest = r.remaining();
    let payload = r.take(rest)?;
    let expected = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul((depth as usize).div_ceil(8)))
        .ok_or_else(|| PsdError::Invalid("filter mask size".into()))?;
    match compression {
        COMPRESSION_RAW => Ok(payload.to_vec()),
        COMPRESSION_ZIP => crate::read::inflate(payload, expected),
        COMPRESSION_ZIP_PREDICTION => {
            let mut out = crate::read::inflate(payload, expected)?;
            undo_prediction(&mut out, w, h, depth as u16);
            Ok(out)
        }
        other => Err(PsdError::Unsupported(format!(
            "filter mask compression {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depth::apply_prediction;
    use crate::descriptor::DescValue;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use pictura_core::{BitDepth, ColorMode, Document, LayerBlock, LayerStack};
    use std::io::Write;

    fn block(key: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = b"8BIM".to_vec();
        out.extend_from_slice(key);
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(data);
        out.extend(std::iter::repeat_n(0u8, (4 - data.len() % 4) % 4));
        out
    }

    fn zip_predicted(plane: &[u8], w: usize, h: usize) -> Vec<u8> {
        let mut data = plane.to_vec();
        apply_prediction(&mut data, w, h, 8);
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&data).expect("zlib");
        enc.finish().expect("zlib")
    }

    /// An `FEid` payload: version 3, one effect, mask on the last channel.
    fn feid_data(
        uuid: &str,
        rect: (i32, i32, i32, i32),
        plane: &[u8],
        w: usize,
        h: usize,
    ) -> Vec<u8> {
        let (top, left, bottom, right) = rect;
        let mut body = Vec::new();
        for v in [top, left, bottom, right] {
            body.extend_from_slice(&v.to_be_bytes());
        }
        body.extend_from_slice(&8u32.to_be_bytes()); // depth
        body.extend_from_slice(&0u32.to_be_bytes()); // max_channels -> 2 channels
        body.extend_from_slice(&0u32.to_be_bytes()); // channel 0 unwritten
        let mut chan = 3u16.to_be_bytes().to_vec(); // ZIP with prediction
        chan.extend(zip_predicted(plane, w, h));
        body.extend_from_slice(&1u32.to_be_bytes()); // channel 1 written
        body.extend_from_slice(&(chan.len() as u64).to_be_bytes());
        body.extend_from_slice(&chan);

        let mut entry = vec![uuid.len() as u8];
        entry.extend_from_slice(uuid.as_bytes());
        entry.extend_from_slice(&1u32.to_be_bytes());
        entry.extend_from_slice(&(body.len() as u64).to_be_bytes());
        entry.extend_from_slice(&body);

        let mut data = 3u32.to_be_bytes().to_vec();
        data.extend_from_slice(&(entry.len() as u64).to_be_bytes());
        data.extend_from_slice(&entry);
        data.extend(std::iter::repeat_n(0u8, (4 - entry.len() % 4) % 4));
        data
    }

    fn mask_extra(
        uuid: &str,
        rect: (i32, i32, i32, i32),
        plane: &[u8],
        w: usize,
        h: usize,
    ) -> Vec<u8> {
        let mut out = block(b"FEid", &feid_data(uuid, rect, plane, w, h));
        out.extend(block(b"FMsk", &[0u8; 10]));
        out
    }

    fn plane() -> Vec<u8> {
        // 4x3: a white left column, black elsewhere.
        let mut p = vec![0u8; 12];
        for y in 0..3 {
            p[y * 4] = 255;
        }
        p
    }

    #[test]
    fn filter_mask_reads_from_the_feid_block() {
        let extra = mask_extra("u-1", (0, 0, 3, 4), &plane(), 4, 3);
        let masks = collect_filter_masks(&extra, false);
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].uuid, "u-1");
        assert_eq!(
            masks[0].rect,
            PsdRect {
                top: 0,
                left: 0,
                bottom: 3,
                right: 4
            }
        );
        assert_eq!(masks[0].coverage, plane());
    }

    #[test]
    fn a_document_without_a_mask_has_none() {
        assert!(collect_filter_masks(&[], false).is_empty());
        assert!(collect_filter_masks(b"not blocks", false).is_empty());
    }

    #[test]
    fn a_mask_survives_a_document_round_trip() {
        let extra = mask_extra("u-1", (0, 0, 3, 4), &plane(), 4, 3);
        let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
        doc.layer_section_extra = extra.into();
        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("re-reads");
        let masks = collect_filter_masks(&back.layer_section_extra, false);
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].coverage, plane());
    }

    #[test]
    fn deleting_a_filter_mask_drops_the_blocks() {
        let extra = mask_extra("u-1", (0, 0, 3, 4), &plane(), 4, 3);
        let out = remove_filter_mask(&extra, "u-1", false).expect("removes");
        assert!(collect_filter_masks(&out, false).is_empty());
        assert!(!out.windows(4).any(|w| w == b"FMsk"));
        assert!(!out.windows(4).any(|w| w == b"FEid"));
    }

    #[test]
    fn removing_an_unrelated_uuid_changes_nothing() {
        let extra = mask_extra("u-1", (0, 0, 3, 4), &plane(), 4, 3);
        assert!(remove_filter_mask(&extra, "other", false).is_none());
    }

    #[test]
    fn a_mask_attaches_to_its_smart_object_by_uuid() {
        let so = {
            let mut so = SmartObject::default();
            so.kind = pictura_core::SmartObjectKind::Embedded;
            so.filename = "src.psd".into();
            so
        }
        .with_payload(b"8BPS\x00\x01x".to_vec());
        let uuid = crate::smart_writer::author_uuid(&so);
        let mut layer = Layer {
            name: "L".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 3,
                right: 4,
            },
            ..Default::default()
        };
        let sold = crate::smart_writer::author_sold_block(&so, &layer, 4, 3);
        layer.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data: sold.into(),
        });
        let mut layers = LayerStack::from_layers(vec![layer]);
        let extra = mask_extra(&uuid, (0, 0, 3, 4), &plane(), 4, 3);
        crate::smart_object::resolve_smart_objects(&mut layers, &extra, false);
        let mask = layers[0]
            .smart_object
            .as_ref()
            .and_then(|so| so.filter_mask.as_ref())
            .expect("mask attaches");
        assert_eq!(mask.data.to_vec(), plane());
    }

    #[test]
    fn a_mask_attaches_by_the_placed_uuid() {
        // Photoshop keys `FEid` by the SoLd `placed` uuid, not `Idnt`.
        let descriptor = DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: vec![
                (b"Idnt".to_vec(), DescValue::Text("idnt-1\0".into())),
                (b"placed".to_vec(), DescValue::Text("placed-1\0".into())),
            ],
        };
        let mut sold = b"soLD".to_vec();
        sold.extend_from_slice(&16u32.to_be_bytes());
        sold.extend_from_slice(&crate::write_descriptor(&descriptor));
        let mut layer = Layer {
            name: "L".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 3,
                right: 4,
            },
            ..Default::default()
        };
        layer.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data: sold.into(),
        });
        let mut layers = LayerStack::from_layers(vec![layer]);
        let extra = mask_extra("placed-1", (0, 0, 3, 4), &plane(), 4, 3);
        crate::smart_object::resolve_smart_objects(&mut layers, &extra, false);
        let so = layers[0].smart_object.as_ref().expect("smart object");
        assert_eq!(so.uuid, "idnt-1");
        assert_eq!(so.placed_uuid, "placed-1");
        assert!(so.filter_mask.is_some(), "the mask attaches by placed");
    }

    fn masked_layer(filename: &str, coverage: Vec<u8>) -> Layer {
        let mut layer = Layer {
            name: filename.into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 3,
                right: 4,
            },
            smart_object: Some(
                {
                    let mut so = SmartObject::default();
                    so.kind = pictura_core::SmartObjectKind::Embedded;
                    so.filename = filename.into();
                    so.filter_mask_enabled = true;
                    so.filter_mask = Some(FilterMask {
                        rect: PsdRect {
                            top: 0,
                            left: 0,
                            bottom: 3,
                            right: 4,
                        },
                        data: coverage.into(),
                    });
                    so
                }
                .with_payload(format!("8BPS\x00\x01{filename}").into_bytes()),
            ),
            ..Default::default()
        };
        for id in 0..3i16 {
            layer.channels.push(pictura_core::Channel {
                id,
                data: vec![90u8; 12].into(),
            });
        }
        layer
    }

    fn masked_doc(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(4, 3, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(layers);
        doc
    }

    fn feid_entries(extra: &[u8], psb: bool) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for_each_block(extra, psb, |key, data| {
            if &key == b"FEid" {
                out = split_filter_effects(data).expect("FEid splits").1;
            }
        });
        out
    }

    fn block_bytes(extra: &[u8], wanted: &[u8; 4]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for_each_block(extra, false, |key, data| {
            if &key == wanted {
                out.push(data.to_vec());
            }
        });
        out
    }

    #[test]
    fn authored_mask_reads_back() {
        let doc = masked_doc(vec![masked_layer("a.psd", plane())]);
        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("reads");
        let so = back.layers[0].smart_object.as_ref().expect("smart object");
        let mask = so.filter_mask.as_ref().expect("the mask attaches");
        assert_eq!(
            mask.rect,
            doc.layers[0]
                .smart_object
                .as_ref()
                .unwrap()
                .filter_mask
                .as_ref()
                .unwrap()
                .rect
        );
        assert_eq!(mask.data.to_vec(), plane());
        assert!(so.filter_mask_enabled);
        assert_eq!(block_bytes(&back.layer_section_extra, b"FMsk").len(), 1);
    }

    #[test]
    fn converted_object_keyed_by_authored_uuid() {
        let doc = masked_doc(vec![masked_layer("a.psd", plane())]);
        let uuid = crate::smart_writer::author_uuid(doc.layers[0].smart_object.as_ref().unwrap());
        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("reads");
        let entries = feid_entries(&back.layer_section_extra, false);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, uuid);
        assert_eq!(
            back.layers[0].smart_object.as_ref().unwrap().placed_uuid,
            uuid
        );
        assert!(back.layers[0]
            .smart_object
            .as_ref()
            .unwrap()
            .filter_mask
            .is_some());
    }

    #[test]
    fn unchanged_mask_bytes_preserved() {
        let doc = masked_doc(vec![masked_layer("a.psd", plane())]);
        let once = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("reads");
        assert_eq!(
            author_filter_masks(&once.layer_section_extra, &once.layers, 3, false),
            once.layer_section_extra.to_vec(),
            "an unedited mask authors nothing"
        );
        let twice = crate::read_psd(&crate::write_psd(&once).expect("writes")).expect("reads");
        for key in [b"FEid", b"FMsk"] {
            assert_eq!(
                block_bytes(&twice.layer_section_extra, key),
                block_bytes(&once.layer_section_extra, key)
            );
        }
    }

    #[test]
    fn changed_mask_replaces_entry_only() {
        let doc = masked_doc(vec![
            masked_layer("a.psd", plane()),
            masked_layer("b.psd", plane()),
        ]);
        let mut once = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("reads");
        let before = feid_entries(&once.layer_section_extra, false);
        assert_eq!(before.len(), 2);
        let edited = vec![128u8; 12];
        once.layers[1]
            .smart_object
            .as_mut()
            .unwrap()
            .filter_mask
            .as_mut()
            .unwrap()
            .data = edited.clone().into();
        let extra = author_filter_masks(&once.layer_section_extra, &once.layers, 3, false);
        let after = feid_entries(&extra, false);
        assert_eq!(after.len(), 2);
        assert_eq!(after[0], before[0], "the other entry keeps its bytes");
        assert_ne!(after[1].1, before[1].1);
        let masks = collect_filter_masks(&extra, false);
        assert_eq!(masks[1].coverage, edited);
        assert_eq!(block_bytes(&extra, b"FMsk").len(), 1, "no second FMsk");
    }

    #[test]
    fn authored_mask_round_trips_psb() {
        let doc = masked_doc(vec![masked_layer("a.psd", plane())]);
        let back = crate::read_psd(&crate::write_psb(&doc).expect("writes")).expect("reads");
        assert!(back.is_psb);
        let mask = back.layers[0]
            .smart_object
            .as_ref()
            .and_then(|so| so.filter_mask.as_ref())
            .expect("the mask attaches");
        assert_eq!(mask.data.to_vec(), plane());
    }
}
