use super::*;
use crate::descriptor::{self, get_object_item, set_object_item};
use crate::error::PsdError;
use pictura_core::{Layer, LayerBlock, SmartFilter, SmartObject, SmartObjectKind};
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
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
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

    let filter = &layer.smart_object().unwrap().smart_filters[0];
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
        .and_then(|l| l.smart_object())
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
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
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

    let so = layer.smart_object().expect("smart object");
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
        .and_then(|l| l.smart_object())
        .expect("smart object survives a write");
    assert!(!so.smart_filters[0].enabled, "per-filter enab persists");
    assert!(!so.smart_filters_enabled, "group enab persists");
}

#[test]
fn toggle_without_a_preserved_block_updates_only_the_typed_view() {
    let mut layer = crate::layer_from_parts(pictura_core::LayerParts {
        smart_object: Some({
            let mut so = SmartObject::default();
            so.smart_filters = vec![SmartFilter {
                filter_id: 2683,
                name: "Camera Raw Filter".to_string(),
                enabled: true,
                options: Vec::new().into(),
            }];
            so.smart_filters_enabled = true;
            so
        }),
        ..Default::default()
    });
    set_smart_filter_enabled(&mut layer, 0, false).expect("typed toggle");
    set_smart_filters_enabled(&mut layer, false).expect("typed group toggle");
    let so = layer.smart_object().unwrap();
    assert!(!so.smart_filters[0].enabled);
    assert!(!so.smart_filters_enabled);
    assert!(
        layer.extra_blocks().is_empty(),
        "no block is authored in memory"
    );
}

#[test]
fn noop_edit_preserves_config_descriptor_bytes() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
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
    let original = layer.smart_object().unwrap().config_descriptor.clone();

    set_camera_raw_option(layer, 0, b"Ex12", DescValue::Double(-1.15)).expect("no-op edit");

    let edited = &layer.smart_object().unwrap().config_descriptor;
    assert_eq!(
        edited.len(),
        original.len(),
        "a same-size edit keeps the descriptor length"
    );
    assert_eq!(
        edited.as_slice(),
        original.as_slice(),
        "a same-value edit keeps the descriptor byte-identical"
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
    crate::edit_layer(layer.as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"SoLE",
            data: data.into(),
        })
    });
    set_camera_raw_option(&mut layer, 0, b"Ex12", DescValue::Double(2.5)).expect("SoLE edit");
    assert_eq!(&layer.extra_blocks()[0].data[..4], b"soLE");

    let mut r = Reader::new(&layer.extra_blocks()[0].data);
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
    crate::edit_layer(layer.as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data: b"junk\x00\x00\x00\x00".to_vec().into(),
        })
    });
    let err = set_camera_raw_option(&mut layer, 0, b"Ex12", DescValue::Double(1.0)).unwrap_err();
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
        .smart_object()
        .expect("smart object")
        .smart_filters
        .iter()
        .map(|filter| filter.filter_id)
        .collect()
}

#[test]
fn reorder_delete_and_clear_round_trip_on_the_fixture() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_with_smart_object02.psd");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return;
    };
    let mut doc = crate::read_psd(&bytes).expect("fixture parses");
    let second = SmartFilter {
        filter_id: 4242,
        name: "Second".to_string(),
        enabled: true,
        options: Vec::new().into(),
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
        .and_then(|l| l.smart_object())
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
            options: Vec::new().into(),
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
        .and_then(|l| l.smart_object())
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
        .and_then(|l| l.smart_object())
        .expect("survives");
    assert!(
        so.smart_filters.is_empty(),
        "a cleared stack authors no filterFX"
    );
}

#[test]
fn out_of_range_delete_and_reorder_are_errors() {
    let mut layer = crate::layer_from_parts(pictura_core::LayerParts {
        smart_object: Some({
            let mut so = SmartObject::default();
            so.smart_filters = vec![SmartFilter {
                filter_id: 2683,
                name: "only".to_string(),
                enabled: true,
                options: Vec::new().into(),
            }];
            so
        }),
        ..Default::default()
    });
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
    crate::layer_from_parts(pictura_core::LayerParts {
        extra_blocks: vec![LayerBlock {
            key: *b"SoLd",
            data: data.clone().into(),
        }],
        smart_object: Some({
            let mut so = SmartObject::default();
            so.smart_filters = (0..typed_len)
                .map(|_| SmartFilter {
                    filter_id: 2683,
                    name: "f".to_string(),
                    enabled: true,
                    options: Vec::new().into(),
                })
                .collect();
            so.config_descriptor = data.into();
            so
        }),
        ..Default::default()
    })
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
    let mut layer = crate::layer_from_parts(pictura_core::LayerParts {
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
    });
    let so = layer.smart_object().cloned().unwrap();
    let data = crate::smart_writer::author_sold_block(&so, &layer, 2, 2);
    crate::edit_layer(layer.as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"SoLd",
            data: data.into(),
        })
    });
    crate::append_smart_filter(
        &mut layer,
        SmartFilter {
            filter_id: 4242,
            name: "F".into(),
            enabled: true,
            options: Vec::new().into(),
        },
    )
    .expect("appends");
    let preserved_flag = |layer: &pictura_core::Layer| {
        let mut parts = pictura_core::LayerParts::from(layer.clone());
        let mut edit = Edit::open(&mut parts).expect("block opens");
        match edit
            .fx_mut()
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
        let so = back.layers[0].smart_object().expect("smart object");
        assert_eq!(so.filter_mask_enabled, enabled);
        assert_eq!(preserved_flag(&back.layers[0]), enabled);
    }
}

fn so_block(signature: &[u8; 4], desc: &DescValue) -> Vec<u8> {
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(signature);
    header[4..].copy_from_slice(&4u32.to_be_bytes());
    serialize_config(&header, desc)
}

fn exposure_filter(filter_id: i32, name: &str, exposure: f64) -> SmartFilter {
    SmartFilter {
        filter_id,
        name: name.to_string(),
        enabled: true,
        options: descriptor::write_descriptor(&object(vec![(
            b"Ex12",
            DescValue::Double(exposure),
        )]))
        .into(),
    }
}

fn filter_stack_desc(filters: &[SmartFilter]) -> DescValue {
    let mut entries: Vec<DescValue> = filters.iter().map(filter_item).collect();
    if let Some(DescValue::Object { items, .. }) = entries.first_mut() {
        set_object_item(items, b"Upri", DescValue::Bool(true));
    }
    object(vec![
        (
            b"Idnt",
            DescValue::Text("00000000-0000-0000-0000-000000000139".into()),
        ),
        (
            b"filterFX",
            object(vec![
                (b"enab", DescValue::Bool(true)),
                (b"filterFXList", DescValue::List(entries)),
            ]),
        ),
    ])
}

fn block_key_for(signature: &[u8; 4]) -> [u8; 4] {
    if signature == b"soLD" {
        *b"SoLd"
    } else {
        *b"SoLE"
    }
}

fn preserved_layer(signature: &[u8; 4], filters: &[SmartFilter]) -> Layer {
    let desc = filter_stack_desc(filters);
    let data = so_block(signature, &desc);
    crate::layer_from_parts(pictura_core::LayerParts {
        name: "L".into(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        extra_blocks: vec![LayerBlock {
            key: block_key_for(signature),
            data: data.clone().into(),
        }],
        smart_object: Some({
            let mut so = SmartObject::default();
            so.kind = SmartObjectKind::Embedded;
            so.filename = "source.psb".into();
            so.smart_filters = filters.to_vec();
            so.config_descriptor = data.into();
            so
        }),
        ..Default::default()
    })
}

fn typed_only_layer(filters: &[SmartFilter]) -> Layer {
    crate::layer_from_parts(pictura_core::LayerParts {
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
                so.kind = SmartObjectKind::Embedded;
                so.filename = "source.psb".into();
                so.smart_filters = filters.to_vec();
                so
            }
            .with_payload(b"8BPS\x00\x01x".to_vec()),
        ),
        ..Default::default()
    })
}

fn junk_layer() -> Layer {
    let filter = exposure_filter(2683, "A", 1.5);
    let desc = object(vec![(
        b"filterFX",
        object(vec![(
            b"filterFXList",
            DescValue::List(vec![filter_item(&filter)]),
        )]),
    )]);
    let mut data = b"junk".to_vec();
    data.extend_from_slice(&4u32.to_be_bytes());
    data.extend_from_slice(&descriptor::write_descriptor(&desc));
    crate::layer_from_parts(pictura_core::LayerParts {
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        extra_blocks: vec![LayerBlock {
            key: *b"SoLd",
            data: data.into(),
        }],
        smart_object: Some({
            let mut so = SmartObject::default();
            so.kind = SmartObjectKind::Embedded;
            so.smart_filters = vec![filter];
            so
        }),
        ..Default::default()
    })
}

#[derive(Debug, PartialEq)]
struct LayerSnapshot {
    block: Vec<u8>,
    config: Vec<u8>,
    typed: Vec<SmartFilter>,
}

fn snapshot(layer: &Layer) -> LayerSnapshot {
    let so = layer.smart_object().expect("smart object");
    let block = layer
        .extra_blocks()
        .iter()
        .find(|b| matches!(&b.key, b"SoLd" | b"SoLE"))
        .map(|b| b.data.as_slice().to_vec())
        .unwrap_or_default();
    LayerSnapshot {
        block,
        config: so.config_descriptor.as_slice().to_vec(),
        typed: so.smart_filters.clone(),
    }
}

fn entry_items(entry: &DescValue) -> &[(Vec<u8>, DescValue)] {
    match entry {
        DescValue::Object { items, .. } => items,
        other => panic!("entry is not an object: {other:?}"),
    }
}

fn entry_id(entry: &DescValue) -> i32 {
    match get_object_item(entry_items(entry), b"filterID") {
        Some(DescValue::Long(id)) => *id,
        _ => 0,
    }
}

fn entry_name(entry: &DescValue) -> String {
    match get_object_item(entry_items(entry), b"Nm  ") {
        Some(DescValue::Text(name)) => name.trim_end_matches('\0').to_string(),
        _ => String::new(),
    }
}

fn entry_enabled(entry: &DescValue) -> bool {
    matches!(
        get_object_item(entry_items(entry), b"enab"),
        Some(DescValue::Bool(true))
    )
}

fn assert_preserved_shape(layer: &Layer) {
    let so = layer.smart_object().expect("smart object");
    let block = layer
        .extra_blocks()
        .iter()
        .find(|b| matches!(&b.key, b"SoLd" | b"SoLE"))
        .expect("preserved block");
    assert!(
        block.data.len().is_multiple_of(4),
        "the block stays 4-aligned"
    );
    assert_eq!(&block.data[..4], b"soLE", "the SoLE signature survives");
    assert_eq!(
        so.config_descriptor.as_slice(),
        block.data.as_slice(),
        "config_descriptor mirrors the block"
    );
}

fn assert_typed_matches_preserved(layer: &Layer) {
    assert_preserved_shape(layer);
    let so = layer.smart_object().expect("smart object");
    let mut parts = pictura_core::LayerParts::from(layer.clone());
    let mut edit = Edit::open(&mut parts).expect("preserved block opens");
    let list = edit.list_mut().expect("filterFXList");
    assert_eq!(
        list.len(),
        so.smart_filters.len(),
        "typed and preserved lengths agree"
    );
    for (entry, filter) in list.iter().zip(&so.smart_filters) {
        assert_eq!(entry_id(entry), filter.filter_id, "filterID agrees");
        assert_eq!(entry_name(entry), filter.name, "Nm agrees");
        assert_eq!(entry_enabled(entry), filter.enabled, "enab agrees");
    }
}

fn assert_agrees(layer: &Layer, preserved: bool, ids: &[i32]) {
    let so = layer.smart_object().expect("smart object");
    let typed: Vec<i32> = so.smart_filters.iter().map(|f| f.filter_id).collect();
    assert_eq!(typed.as_slice(), ids, "typed order");
    if preserved {
        assert_typed_matches_preserved(layer);
    } else {
        assert!(
            layer
                .extra_blocks()
                .iter()
                .all(|b| !matches!(&b.key, b"SoLd" | b"SoLE")),
            "typed-only storage authors no block in memory"
        );
    }
}

fn run_edit_sequence(layer: &mut Layer, preserved: bool) {
    append_smart_filter(layer, exposure_filter(777, "C", 2.0)).expect("append");
    assert_agrees(layer, preserved, &[2683, 4242, 777]);

    replace_smart_filter(layer, 1, exposure_filter(999, "R", 3.0)).expect("replace");
    assert_agrees(layer, preserved, &[2683, 999, 777]);

    set_smart_filter_enabled(layer, 0, false).expect("toggle filter");
    assert_agrees(layer, preserved, &[2683, 999, 777]);
    assert!(
        !layer.smart_object().unwrap().smart_filters[0].enabled,
        "the typed filter follows the toggle"
    );

    set_smart_filters_enabled(layer, false).expect("toggle group");
    assert_agrees(layer, preserved, &[2683, 999, 777]);
    assert!(
        !layer.smart_object().unwrap().smart_filters_enabled,
        "the typed group follows the toggle"
    );

    if preserved {
        set_camera_raw_option(layer, 0, b"Ex12", DescValue::Double(2.5)).expect("option");
        assert_agrees(layer, preserved, &[2683, 999, 777]);
        let filter = &layer.smart_object().unwrap().smart_filters[0];
        let mut parts = pictura_core::LayerParts::from(layer.clone());
        let mut edit = Edit::open(&mut parts).expect("opens");
        let entry = edit.entry_mut(0).expect("entry");
        let fltr = get_object_item(entry, b"Fltr").expect("Fltr");
        assert_eq!(
            filter.options.as_slice(),
            descriptor::write_descriptor(fltr).as_slice(),
            "typed options match the serialized Fltr"
        );
    }

    delete_smart_filter(layer, 1).expect("delete");
    assert_agrees(layer, preserved, &[2683, 777]);

    reorder_smart_filters(layer, 0, 1).expect("reorder");
    assert_agrees(layer, preserved, &[777, 2683]);
}

fn written_filter_ids(layer: &Layer) -> Vec<i32> {
    let mut doc = pictura_core::Document::new(
        2,
        2,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    );
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer.clone()]);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    back.layers[0]
        .smart_object()
        .expect("smart object survives")
        .smart_filters
        .iter()
        .map(|f| f.filter_id)
        .collect()
}

fn descriptor_has_filter_fx(layer: &Layer) -> bool {
    let mut parts = pictura_core::LayerParts::from(layer.clone());
    let mut edit = Edit::open(&mut parts).expect("opens");
    edit.fx_mut().is_some()
}

#[test]
fn edit_sequence_agrees_across_preserved_and_typed_only() {
    let a = exposure_filter(2683, "A", 1.5);
    let b = exposure_filter(4242, "B", 0.5);

    let mut preserved = preserved_layer(b"soLE", &[a.clone(), b.clone()]);
    let original = snapshot(&preserved);
    set_camera_raw_option(&mut preserved, 0, b"Ex12", DescValue::Double(1.5))
        .expect("same-value edit");
    assert_eq!(
        snapshot(&preserved),
        original,
        "a same-value option edit leaves the descriptor byte-identical"
    );
    run_edit_sequence(&mut preserved, true);
    {
        let mut parts = pictura_core::LayerParts::from(preserved.clone());
        let mut edit = Edit::open(&mut parts).expect("preserved block opens");
        let list = edit.list_mut().expect("filterFXList");
        let entry = list
            .iter()
            .find(|entry| entry_id(entry) == 2683)
            .expect("the 2683 entry survives");
        assert!(
            get_object_item(entry_items(entry), b"Upri").is_some(),
            "an unmodeled filter key survives the edits"
        );
        assert!(
            get_object_item(entry_items(entry), b"blendOptions").is_some(),
            "blendOptions survives the edits"
        );
    }

    let mut typed_only = typed_only_layer(&[a, b]);
    run_edit_sequence(&mut typed_only, false);

    let preserved_ids = written_filter_ids(&preserved);
    let typed_only_ids = written_filter_ids(&typed_only);
    assert_eq!(preserved_ids, vec![777, 2683]);
    assert_eq!(typed_only_ids, vec![777, 2683]);
    assert_eq!(
        preserved_ids, typed_only_ids,
        "both storage shapes agree after a write and read"
    );

    clear_smart_filters(&mut preserved).expect("clear");
    assert!(preserved.smart_object().unwrap().smart_filters.is_empty());
    assert!(
        !descriptor_has_filter_fx(&preserved),
        "a cleared stack authors no filterFX"
    );

    clear_smart_filters(&mut typed_only).expect("clear");
    assert!(typed_only.smart_object().unwrap().smart_filters.is_empty());
    assert!(
        typed_only.extra_blocks().is_empty(),
        "the typed-only clear authors no block in memory"
    );
}

#[test]
fn refused_edit_leaves_block_descriptor_and_typed_view_unchanged() {
    let mut layer = preserved_layer(
        b"soLE",
        &[
            exposure_filter(2683, "A", 1.5),
            exposure_filter(4242, "B", 0.5),
        ],
    );
    let before = snapshot(&layer);
    assert!(delete_smart_filter(&mut layer, 9).is_err());
    assert_eq!(
        snapshot(&layer),
        before,
        "an out-of-range delete changes nothing"
    );
    assert!(reorder_smart_filters(&mut layer, 0, 9).is_err());
    assert_eq!(
        snapshot(&layer),
        before,
        "an out-of-range reorder changes nothing"
    );

    let mut short = layer_with_preserved_filter_list(Some(1), 2);
    let before = snapshot(&short);
    assert!(reorder_smart_filters(&mut short, 0, 1).is_err());
    assert_eq!(
        snapshot(&short),
        before,
        "a preserved list that cannot follow is refused"
    );
    assert!(delete_smart_filter(&mut short, 1).is_err());
    assert_eq!(
        snapshot(&short),
        before,
        "a delete past the preserved list is refused"
    );

    let mut junk = junk_layer();
    let before = snapshot(&junk);
    assert!(set_camera_raw_option(&mut junk, 0, b"Ex12", DescValue::Double(9.0)).is_err());
    assert_eq!(snapshot(&junk), before, "an unknown signature is refused");
}

#[test]
fn replace_validates_the_config_signature() {
    let mut layer = junk_layer();
    let before = snapshot(&layer);
    let err = replace_smart_filter(&mut layer, 0, exposure_filter(999, "R", 2.0)).unwrap_err();
    assert!(matches!(err, PsdError::Unsupported(_)), "got {err:?}");
    assert_eq!(
        snapshot(&layer),
        before,
        "a refused replace leaves the layer unchanged"
    );
}
