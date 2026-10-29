//! Tests for the Lens Correction smart filter codec.

use super::*;
use crate::common::Reader;
use crate::descriptor::{self, get_object_item, DescValue};
use pictura_core::{ColorMode, Document, Layer, LayerBlock, PsdRect, SmartObject, SmartObjectKind};

fn embedded_layer() -> Layer {
    Layer {
        name: "L".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        smart_object: Some(
            {
                let mut so = SmartObject::default();
                so.kind = SmartObjectKind::Embedded;
                so.filename = "src.psd".into();
                so
            }
            .with_payload(b"8BPS\x00\x01not-a-real-psd".to_vec()),
        ),
        ..Default::default()
    }
}

fn doc_with(layer: Layer) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, pictura_core::BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn read_back(layer: Layer) -> Document {
    crate::read_psd(&crate::write_psd(&doc_with(layer)).expect("writes")).expect("re-reads")
}

/// A realistic slot list: booleans, whole-number ints, and fractional doubles.
fn slots() -> Vec<f64> {
    vec![
        1.0,
        0.0,
        1.0,
        1.0,
        2.0,
        1_234_567_890.0,
        -12.5,
        3.25,
        -4.0,
        0.0,
        -20.0,
        30.0,
        10.0,
        -5.0,
        45.0,
        120.0,
        3.0,
        1.0,
        0.0125,
        -0.08,
        0.0,
        0.004,
        0.0,
        -0.0001,
        1.0004,
        0.0002,
        -0.0001,
        1.0,
        0.0,
        0.0,
        0.9996,
        -0.0002,
        0.0001,
        -0.3,
        0.05,
        -0.01,
    ]
}

/// The `filterFXList[0]` entry of a preserved `SoLd` descriptor.
fn config_filter_entry(config_descriptor: &[u8]) -> Vec<(Vec<u8>, DescValue)> {
    let mut r = Reader::new(&config_descriptor[8..]);
    let DescValue::Object { items, .. } =
        descriptor::read_descriptor(&mut r).expect("descriptor parses")
    else {
        panic!("descriptor is an object");
    };
    let DescValue::Object {
        items: filter_fx, ..
    } = get_object_item(&items, b"filterFX").expect("filterFX present")
    else {
        panic!("filterFX is an object");
    };
    let DescValue::List(list) = get_object_item(filter_fx, b"filterFXList").expect("list") else {
        panic!("filterFXList is a list");
    };
    let DescValue::Object { items: entry, .. } = &list[0] else {
        panic!("entry is an object");
    };
    entry.clone()
}

#[test]
fn slots_round_trip_through_the_descriptor() {
    let slots = slots();
    let bytes = crate::write_descriptor(&encode_lens_correction_fltr(&slots));
    assert_eq!(decode_lens_correction_slots(&bytes), slots);
}

#[test]
fn missing_keys_decode_to_the_opening_slots() {
    assert_eq!(decode_lens_correction_slots(&[]), DEFAULT_SLOTS.to_vec());
    assert_eq!(
        decode_lens_correction_slots(b"not a descriptor"),
        DEFAULT_SLOTS.to_vec()
    );

    // A descriptor whose Angle key is dropped falls back to the opening 0.
    let mut slots = DEFAULT_SLOTS.to_vec();
    slots[14] = 45.0;
    let DescValue::Object { items, .. } = encode_lens_correction_fltr(&slots) else {
        unreachable!("an object");
    };
    let items: Vec<(Vec<u8>, DescValue)> = items
        .into_iter()
        .filter(|(key, _)| key != b"Angl")
        .collect();
    let bytes = crate::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"x".to_vec(),
        items,
    });
    assert_eq!(decode_lens_correction_slots(&bytes)[14], 0.0);

    // A record from before the profile slots: only the first 16 keys. The
    // tail takes its opening values, so the coefficients flag is off.
    let full = self::slots();
    let DescValue::Object { items, .. } = encode_lens_correction_fltr(&full) else {
        unreachable!("an object");
    };
    let bytes = crate::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"x".to_vec(),
        items: items.into_iter().take(16).collect(),
    });
    let decoded = decode_lens_correction_slots(&bytes);
    assert_eq!(decoded[..16], full[..16]);
    assert_eq!(decoded[16..], DEFAULT_SLOTS[16..]);
    assert_eq!(decoded[17], 0.0);
}

#[test]
fn attach_records_the_filter() {
    let slots = slots();
    let mut layer = embedded_layer();
    attach_lens_correction_filter(&mut layer, &slots).expect("attaches");

    let so = layer.smart_object.as_ref().expect("smart object");
    assert_eq!(so.smart_filters.len(), 1);
    assert_eq!(so.smart_filters[0].filter_id, LENS_CORRECTION_FILTER_ID);
    assert_eq!(so.smart_filters[0].name, LENS_CORRECTION_FILTER_NAME);
    assert_eq!(
        decode_lens_correction_slots(&so.smart_filters[0].options),
        slots
    );

    let back = read_back(layer);
    let so = back.layers[0].smart_object.as_ref().expect("survives");
    assert_eq!(so.smart_filters.len(), 1);
    assert_eq!(so.smart_filters[0].filter_id, LENS_CORRECTION_FILTER_ID);
    assert_eq!(so.smart_filters[0].name, LENS_CORRECTION_FILTER_NAME);
    assert_eq!(
        decode_lens_correction_slots(&so.smart_filters[0].options),
        slots
    );
}

#[test]
fn reattaching_keeps_unmodeled_keys() {
    // A preserved entry whose Fltr carries a key this codec does not model.
    let DescValue::Object { items, .. } = encode_lens_correction_fltr(&DEFAULT_SLOTS) else {
        unreachable!("an object");
    };
    let mut items = items;
    items.push((b"Xtra".to_vec(), DescValue::Long(7)));
    let fltr = DescValue::Object {
        name: String::new(),
        class_id: b"Pictura Lens Correction Filter".to_vec(),
        items,
    };
    let so = {
        let mut so = SmartObject::default();
        so.kind = SmartObjectKind::Embedded;
        so.smart_filters = vec![SmartFilter {
            filter_id: LENS_CORRECTION_FILTER_ID,
            name: LENS_CORRECTION_FILTER_NAME.to_string(),
            enabled: true,
            options: crate::write_descriptor(&fltr).into(),
        }];
        so
    }
    .with_payload(b"8BPS\x00\x01x".to_vec());
    let mut layer = embedded_layer();
    let data = crate::smart_writer::author_sold_block(&so, &layer, 2, 2);
    layer.extra_blocks.push(LayerBlock {
        key: *b"SoLd",
        data: data.into(),
    });

    let new_slots = slots();
    attach_lens_correction_filter(&mut layer, &new_slots).expect("re-attaches");

    let back = read_back(layer);
    let so = back.layers[0].smart_object.as_ref().expect("smart object");
    assert_eq!(so.smart_filters.len(), 1, "replaced, not duplicated");
    assert_eq!(
        decode_lens_correction_slots(&so.smart_filters[0].options),
        new_slots
    );

    let entry = config_filter_entry(&so.config_descriptor);
    assert!(
        get_object_item(&entry, b"blendOptions").is_some(),
        "the per-filter blendOptions survives the merge"
    );
    let DescValue::Object { items: fltr, .. } =
        get_object_item(&entry, b"Fltr").expect("Fltr present")
    else {
        panic!("Fltr is an object");
    };
    assert_eq!(
        get_object_item(fltr, b"Xtra"),
        Some(&DescValue::Long(7)),
        "an unmodeled Fltr key survives the merge"
    );
}

#[test]
fn a_foreign_filter_id_is_left_alone() {
    let foreign = SmartFilter {
        filter_id: 4242,
        name: "Foreign".to_string(),
        enabled: true,
        options: crate::write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"null".to_vec(),
            items: vec![(b"Keep".to_vec(), DescValue::Long(7))],
        })
        .into(),
    };
    let so = {
        let mut so = SmartObject::default();
        so.kind = SmartObjectKind::Embedded;
        so.smart_filters = vec![foreign.clone()];
        so
    }
    .with_payload(b"8BPS\x00\x01x".to_vec());
    let mut layer = embedded_layer();
    let data = crate::smart_writer::author_sold_block(&so, &layer, 2, 2);
    layer.extra_blocks.push(LayerBlock {
        key: *b"SoLd",
        data: data.into(),
    });

    attach_lens_correction_filter(&mut layer, &DEFAULT_SLOTS).expect("attaches");

    let back = read_back(layer);
    let so = back.layers[0].smart_object.as_ref().expect("smart object");
    assert_eq!(so.smart_filters.len(), 2);
    let kept = so
        .smart_filters
        .iter()
        .find(|f| f.filter_id == 4242)
        .expect("foreign filter survives");
    assert_eq!(kept.options, foreign.options);
}
