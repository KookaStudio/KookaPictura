//! Authoring of embedded smart objects.
//!
//! A layer that carries an embedded payload but no preserved `SoLd`/`SoLE`/
//! `plLd`/`PlLd` block gets a freshly built `SoLd` config descriptor and a
//! document-level `lnk2` record sharing one uuid. Vector placements carry an
//! assigned source identity. Legacy embedded objects use a deterministic
//! identity derived from filename and payload.

use pictura_core::warp::Warp;
use pictura_core::{Homography, Layer, PsdRect, SmartObject, SmartObjectKind};

use crate::descriptor::DescValue;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_OFFSET_ALT: u64 = 0x8422_2325_cbf2_9ce4;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The layer's smart object when the writer must author one: embedded, with a
/// payload, and no preserved config block to re-emit.
pub(crate) fn should_author(layer: &Layer) -> Option<&SmartObject> {
    let so = layer.smart_object()?;
    if so.kind != SmartObjectKind::Embedded || so.payload().is_none() {
        return None;
    }
    let config_keys = [*b"SoLd", *b"SoLE", *b"plLd", *b"PlLd"];
    if layer
        .extra_blocks()
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

/// Use an authored vector's source identity or derive a legacy identity from
/// its filename and payload.
pub(crate) fn author_uuid(so: &SmartObject) -> String {
    if so.vector_placement.is_some() && !so.uuid.is_empty() {
        return so.uuid.clone();
    }
    let mut data = so.filename.as_bytes().to_vec();
    data.push(0);
    data.extend_from_slice(so.payload().unwrap_or(&[]));
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&fnv1a(FNV_OFFSET, &data).to_be_bytes());
    bytes[8..].copy_from_slice(&fnv1a(FNV_OFFSET_ALT, &data).to_be_bytes());
    format_uuid(bytes)
}

pub fn new_vector_source_id() -> String {
    format_uuid(crate::os_random())
}

fn format_uuid(mut bytes: [u8; 16]) -> String {
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

fn quad_value(quad: [(f64, f64); 4]) -> DescValue {
    DescValue::List(
        quad.into_iter()
            .flat_map(|(x, y)| [x, y])
            .map(DescValue::Double)
            .collect(),
    )
}

fn corners([left, top, right, bottom]: [f64; 4]) -> [(f64, f64); 4] {
    [(left, top), (right, top), (right, bottom), (left, bottom)]
}

fn rect_bounds(rect: PsdRect) -> [f64; 4] {
    [rect.left, rect.top, rect.right, rect.bottom].map(f64::from)
}

fn quad_bounds(quad: [(f64, f64); 4]) -> [f64; 4] {
    quad.into_iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |[left, top, right, bottom], (x, y)| {
            [
                left.min(x).floor(),
                top.min(y).floor(),
                right.max(x).ceil(),
                bottom.max(y).ceil(),
            ]
        },
    )
}

fn move_frame(points: &mut [(f64, f64); 4], from: [f64; 4], to: [f64; 4]) -> Option<()> {
    if from[0] >= from[2] || from[1] >= from[3] {
        return None;
    }
    let sx = (to[2] - to[0]) / (from[2] - from[0]);
    let sy = (to[3] - to[1]) / (from[3] - from[1]);
    for (x, y) in points {
        *x = to[0] + (*x - from[0]) * sx;
        *y = to[1] + (*y - from[1]) * sy;
    }
    Some(())
}

fn placed_vector_frame(
    so: &SmartObject,
    rect: PsdRect,
    native_bounds: [f64; 4],
) -> Option<([(f64, f64); 4], Warp)> {
    let Some((first, rest)) = so.vector_transforms.split_first() else {
        return Some((corners(rect_bounds(rect)), Warp::none(native_bounds)));
    };
    // A first preset warp followed only by homographies fits the standard
    // descriptor. Other chains keep their full geometry in the private replay.
    let standard_warp = first.warp.as_ref().filter(|warp| {
        warp.style.is_preset()
            && warp.bounds == rect_bounds(first.source)
            && rest.iter().all(|step| step.warp.is_none())
    });
    let mut points = corners(rect_bounds(first.source));
    let mut frame = rect_bounds(first.source);
    for (index, step) in so.vector_transforms.iter().enumerate() {
        let source = rect_bounds(step.source);
        move_frame(&mut points, frame, source)?;
        let projected = step.warp.as_ref().map_or(source, |warp| warp.bounds);
        let transform = Homography::from_quads(corners(projected), step.quad)?;
        for point in &mut points {
            if let Some(warp) = &step.warp {
                if index != 0 || standard_warp.is_none() {
                    *point = warp.map(point.0, point.1);
                }
            }
            *point = transform.apply(point.0, point.1)?;
        }
        frame = step
            .destination
            .map_or_else(|| quad_bounds(step.quad), rect_bounds);
    }
    move_frame(&mut points, frame, rect_bounds(rect))?;
    let mut warp = standard_warp
        .cloned()
        .unwrap_or_else(|| Warp::none(native_bounds));
    warp.bounds = native_bounds;
    Some((points, warp))
}

fn warp_value(warp: &Warp) -> DescValue {
    let [left, top, right, bottom] = warp.bounds;
    obj_class(
        b"warp",
        vec![
            (
                b"warpStyle",
                enum_value(b"warpStyle", warp.style.psd_name().as_bytes()),
            ),
            (b"warpValue", double(warp.bend)),
            (b"warpPerspective", double(warp.h_distort)),
            (b"warpPerspectiveOther", double(warp.v_distort)),
            (
                b"warpRotate",
                enum_value(b"warpRotate", if warp.vertical { b"Vrtc" } else { b"Hrzn" }),
            ),
            (
                b"bounds",
                obj_class(
                    b"classFloatRect",
                    vec![
                        (b"Top ", double(top)),
                        (b"Left", double(left)),
                        (b"Btom", double(bottom)),
                        (b"Rght", double(right)),
                    ],
                ),
            ),
            (b"uOrder", long(4)),
            (b"vOrder", long(4)),
        ],
    )
}

fn size_value(width: f64, height: f64) -> DescValue {
    obj_class(
        b"Pnt ",
        vec![(b"Wdth", double(width)), (b"Hght", double(height))],
    )
}

fn placed_geometry(
    so: &SmartObject,
    rect: PsdRect,
    width: f64,
    height: f64,
) -> (DescValue, DescValue) {
    let bounds = [0.0, 0.0, width, height];
    let (quad, warp) = if so.vector_placement.is_some() {
        placed_vector_frame(so, rect, bounds)
            .unwrap_or_else(|| (corners(rect_bounds(rect)), Warp::none(bounds)))
    } else {
        (corners([0.0, 0.0, 1.0, 1.0]), Warp::none(bounds))
    };
    (quad_value(quad), warp_value(&warp))
}

pub(crate) fn refresh_authored_vector_geometry(layer: &Layer, data: &[u8]) -> Option<Vec<u8>> {
    let so = layer
        .smart_object()
        .filter(|so| so.authored_vector_placement)?;
    let placement = so.vector_placement?;
    let mut descriptor = crate::smart_object::read_layer_data_descriptor(data).ok()?;
    let DescValue::Object { items, .. } = &mut descriptor else {
        return None;
    };
    let (width, height) = (placement.width_points, placement.height_points);
    let (quad, warp) = placed_geometry(so, layer.rect, width, height);
    for (key, value) in [
        (b"PgNm".as_slice(), long(placement.page as i32)),
        (b"totalPages", long(placement.page_count as i32)),
        (b"Crop", long(i32::from(placement.crop))),
        (b"Trnf", quad.clone()),
        (b"nonAffineTransform", quad),
        (b"warp", warp),
        (b"Sz  ", size_value(width, height)),
    ] {
        crate::descriptor::set_object_item(items, key, value);
    }
    Some(serialize_sold(&data[..8], &descriptor))
}

fn serialize_sold(header: &[u8], descriptor: &DescValue) -> Vec<u8> {
    let mut data = header.to_vec();
    data.extend_from_slice(&crate::descriptor::write_descriptor(descriptor));
    data.resize(data.len().next_multiple_of(4), 0);
    data
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

    let (width, height, page, page_count, crop, kind) =
        so.vector_placement
            .map_or((width, height, 1, 1, 1, 2), |placement| {
                (
                    placement.width_points,
                    placement.height_points,
                    placement.page as i32,
                    placement.page_count as i32,
                    i32::from(placement.crop),
                    1,
                )
            });
    let (quad, warp) = placed_geometry(so, layer.rect, width, height);
    let mut descriptor = obj(vec![
        (b"Idnt", text(&uuid)),
        (b"placed", text(&uuid)),
        (b"Nm  ", text(name)),
        (b"PgNm", long(page)),
        (b"totalPages", long(page_count)),
        (b"Crop", long(crop)),
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
        (b"Type", long(kind)),
        (b"Trnf", quad.clone()),
        (b"nonAffineTransform", quad),
        (b"warp", warp),
        (b"Sz  ", size_value(width, height)),
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

    serialize_sold(b"soLD\0\0\0\x04", &descriptor)
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
            walk(layer.children(), out);
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
