//! Authoring of embedded smart objects.
//!
//! A layer that carries an embedded payload but no preserved `SoLd`/`SoLE`/
//! `plLd`/`PlLd` block gets a freshly built `SoLd` config descriptor and a
//! document-level `lnk2` record sharing one deterministic uuid. The uuid is a
//! pure function of the filename and payload, so two layers with the same
//! embedded source dedupe to one record and the output stays reproducible.

use pictura_core::{Layer, SmartObject, SmartObjectKind};

use crate::descriptor::DescValue;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_OFFSET_ALT: u64 = 0x8422_2325_cbf2_9ce4;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The layer's smart object when the writer must author one: embedded, with a
/// payload, and no preserved config block to re-emit.
pub(crate) fn should_author(layer: &Layer) -> Option<&SmartObject> {
    let so = layer.smart_object.as_ref()?;
    if so.kind != SmartObjectKind::Embedded || so.payload().is_none() {
        return None;
    }
    let config_keys = [*b"SoLd", *b"SoLE", *b"plLd", *b"PlLd"];
    if layer
        .extra_blocks
        .iter()
        .any(|b| config_keys.contains(&b.key))
    {
        return None;
    }
    Some(so)
}

fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// A deterministic RFC-4122 version-4-shaped uuid over `filename` + payload.
pub(crate) fn author_uuid(so: &SmartObject) -> String {
    let mut data = so.filename.as_bytes().to_vec();
    data.push(0);
    data.extend_from_slice(so.payload().unwrap_or(&[]));
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&fnv1a(FNV_OFFSET, &data).to_be_bytes());
    bytes[8..].copy_from_slice(&fnv1a(FNV_OFFSET_ALT, &data).to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let mut out = String::with_capacity(36);
    for (i, byte) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn text(value: &str) -> DescValue {
    DescValue::Text(value.to_string())
}

fn long(value: i32) -> DescValue {
    DescValue::Long(value)
}

fn double(value: f64) -> DescValue {
    DescValue::Double(value)
}

fn enum_value(kind: &[u8], value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: kind.to_vec(),
        value: value.to_vec(),
    }
}

fn obj_class(class_id: &[u8], items: Vec<(&[u8], DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class_id.to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    }
}

fn obj(items: Vec<(&[u8], DescValue)>) -> DescValue {
    obj_class(b"null", items)
}

fn unit_quad() -> DescValue {
    DescValue::List(
        [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
            .into_iter()
            .map(DescValue::Double)
            .collect(),
    )
}

/// The `SoLd` block data: `b"soLD"` + outer version 4 + a version-16 descriptor
/// mirroring the fields the reference and psd-tools expect for a placed object.
pub(crate) fn author_sold_block(
    so: &SmartObject,
    layer: &Layer,
    doc_width: u32,
    doc_height: u32,
) -> Vec<u8> {
    let uuid = author_uuid(so);
    let (width, height) = if layer.rect.width() > 0 && layer.rect.height() > 0 {
        (layer.rect.width() as f64, layer.rect.height() as f64)
    } else {
        (doc_width as f64, doc_height as f64)
    };
    let name = if layer.name.is_empty() {
        so.filename.as_str()
    } else {
        layer.name.as_str()
    };

    let mut descriptor = obj(vec![
        (b"Idnt", text(&uuid)),
        (b"placed", text(&uuid)),
        (b"Nm  ", text(name)),
        (b"PgNm", long(1)),
        (b"totalPages", long(1)),
        (b"Crop", long(1)),
        (
            b"frameStep",
            obj(vec![(b"numerator", long(0)), (b"denominator", long(600))]),
        ),
        (
            b"duration",
            obj(vec![(b"numerator", long(0)), (b"denominator", long(600))]),
        ),
        (b"frameCount", long(1)),
        (b"Annt", long(16)),
        (b"Type", long(2)),
        (b"Trnf", unit_quad()),
        (b"nonAffineTransform", unit_quad()),
        (
            b"warp",
            obj_class(
                b"warp",
                vec![
                    (b"warpStyle", enum_value(b"warpStyle", b"warpNone")),
                    (b"warpValue", double(0.0)),
                    (b"warpPerspective", double(0.0)),
                    (b"warpPerspectiveOther", double(0.0)),
                    (b"warpRotate", enum_value(b"warpRotate", b"Hrzn")),
                    (
                        b"bounds",
                        obj_class(
                            b"classFloatRect",
                            vec![
                                (b"Top ", double(0.0)),
                                (b"Left", double(0.0)),
                                (b"Btom", double(height)),
                                (b"Rght", double(width)),
                            ],
                        ),
                    ),
                    (b"uOrder", long(4)),
                    (b"vOrder", long(4)),
                ],
            ),
        ),
        (
            b"Sz  ",
            obj_class(
                b"Pnt ",
                vec![(b"Wdth", double(width)), (b"Hght", double(height))],
            ),
        ),
        (
            b"Rslt",
            DescValue::UnitFloat {
                unit: *b"#Rsl",
                value: 72.0,
            },
        ),
        (b"comp", long(-1)),
        (
            b"compInfo",
            obj(vec![(b"compID", long(-1)), (b"originalCompID", long(-1))]),
        ),
    ]);
    // A converted embedded object may carry a smart filter (Pictura Raw) that
    // the writer authors into the same `SoLd` descriptor.
    if let Some(filter_fx) = crate::pictura_raw::author_filter_fx(
        &so.smart_filters,
        so.smart_filters_enabled,
        so.filter_mask_enabled,
        so.filter_mask_linked,
        so.filter_mask_extend_with_white,
    ) {
        if let DescValue::Object { items, .. } = &mut descriptor {
            items.push((b"filterFX".to_vec(), filter_fx));
        }
    }

    let mut data = b"soLD".to_vec();
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(&crate::descriptor::write_descriptor(&descriptor));
    // The reference 4-aligns the SoLd data; matching it keeps a same-size filter
    // edit from changing the block length (see `smart_filter`).
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
    data
}

fn write_pascal_raw(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
}

fn write_unicode_raw(out: &mut Vec<u8>, value: &str) {
    let units: Vec<u16> = value.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u32).to_be_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
}

/// One `liFD` record matching the reader's field order and version gating.
fn linked_layer_bytes(so: &SmartObject, uuid: &str) -> Vec<u8> {
    let payload = so.payload().unwrap_or(&[]);
    let filetype = if so.filetype == [0; 4] {
        *b"8BPB"
    } else {
        so.filetype
    };
    let creator = if so.creator == [0; 4] {
        *b"8BIM"
    } else {
        so.creator
    };

    let mut v = Vec::with_capacity(payload.len() + 64);
    v.extend_from_slice(b"liFD");
    v.extend_from_slice(&7u32.to_be_bytes());
    write_pascal_raw(&mut v, uuid);
    write_unicode_raw(&mut v, &so.filename);
    v.extend_from_slice(&filetype);
    v.extend_from_slice(&creator);
    v.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    v.push(0); // no open_file descriptor
    v.extend_from_slice(payload);
    write_unicode_raw(&mut v, "\0");
    v.extend_from_slice(&0f64.to_be_bytes());
    v.push(0);
    v
}

/// Authoring smart objects in layer order, recursing into groups.
pub(crate) fn collect_authoring(layers: &[Layer]) -> Vec<&SmartObject> {
    fn walk<'a>(layers: &'a [Layer], out: &mut Vec<&'a SmartObject>) {
        for layer in layers {
            if let Some(so) = should_author(layer) {
                out.push(so);
            }
            walk(&layer.children, out);
        }
    }
    let mut out = Vec::new();
    walk(layers, &mut out);
    out
}

/// One document-level `lnk2` tagged block holding one record per distinct uuid,
/// in first-seen order. The block length is `u64` under PSB (`lnk2` is a big key)
/// and is padded externally to 4 bytes, matching a global tagged block.
pub(crate) fn author_lnk2_bytes(sos: &[&SmartObject], psb: bool) -> Vec<u8> {
    let mut records: Vec<(&SmartObject, String)> = Vec::new();
    for &so in sos {
        let uuid = author_uuid(so);
        if !records.iter().any(|(_, seen)| *seen == uuid) {
            records.push((so, uuid));
        }
    }
    if records.is_empty() {
        return Vec::new();
    }

    let mut list = Vec::new();
    for (so, uuid) in &records {
        let record = linked_layer_bytes(so, uuid);
        list.extend_from_slice(&(record.len() as u64).to_be_bytes());
        list.extend_from_slice(&record);
        let pad = (4 - record.len() % 4) % 4;
        list.extend(std::iter::repeat_n(0u8, pad));
    }

    let mut out = Vec::new();
    crate::write::write_tag_document(&mut out, b"lnk2", &list, psb);
    out
}
