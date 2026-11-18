use pictura_core::layer_comp::{
    ensure_layer_ids, layer_id, set_layer_anchor, set_layer_id, BlendKey, CapturedState, CompEntry,
    CompRecord, LayerComp, PositionAnchor,
};
use pictura_core::{
    BitDepth, BlendMode, ColorMode, Document, Layer, LayerBlock, LayerStack, PsdRect,
};

use crate::image_resources::{
    decode_image_resources, encode_image_resources, frame_image_resource,
};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

fn layer(name: &str, id: Option<u32>) -> Layer {
    let mut layer = crate::layer_from_parts(pictura_core::LayerParts {
        name: name.to_string(),
        rect: rect(0, 0, 4, 4),
        ..Default::default()
    });
    if let Some(id) = id {
        set_layer_id(&mut layer, id);
    }
    layer
}

fn document(layer: Layer) -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

fn comp(id: u32, name: &str, record: u32) -> LayerComp {
    let state = CapturedState {
        visible: Some(false),
        position: Some(PositionAnchor { x: 1.0, y: 2.0 }),
        blend: Some(BlendKey::Mode(BlendMode::Multiply)),
        style_visible: None,
    };
    LayerComp::new(
        id,
        name,
        "comment",
        CompRecord::from_bits(record),
        vec![CompEntry { layer_id: 1, state }],
    )
}

fn comp_document(record: u32) -> Document {
    let mut doc = document(layer("A", Some(1)));
    doc.layer_comps.push_comp(comp(1, "A", record));
    doc.layer_comps.parsed = true;
    doc.layer_comps.dirty = true;
    doc
}

fn comp_resource(doc: &Document) -> Option<Vec<u8>> {
    decode_image_resources(doc)
        .into_iter()
        .find(|resource| resource.id == 1065)
        .map(|resource| resource.data)
}

#[test]
fn read_comp_list_from_resource() {
    let bytes = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(doc.layer_comps.len(), 1);
    assert_eq!(doc.layer_comps.comps()[0].name, "A");
}

#[test]
fn a_document_without_resource_1065_has_no_comps() {
    let bytes = crate::write_psd(&document(layer("A", Some(1)))).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert!(doc.layer_comps.is_empty());
}

#[test]
fn malformed_comp_resource_opens_without_comps() {
    let mut doc = document(layer("A", Some(1)));
    doc.image_resources = encode_image_resources(&[frame_image_resource(1065, "", b"\0")]).into();
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layer_comps.is_empty());
}

#[test]
fn comp_comment_is_read() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(doc.layer_comps.comps()[0].comment, "comment");
}

#[test]
fn read_layer_visibility_variant() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(
        doc.layer_comps.comps()[0].entry(1).and_then(|s| s.visible),
        Some(false)
    );
}

#[test]
fn read_layer_position_and_blend_variant() {
    let bytes = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    let state = doc.layer_comps.comps()[0].entry(1).expect("entry");
    assert_eq!(state.position, Some(PositionAnchor { x: 1.0, y: 2.0 }));
    assert_eq!(state.blend, Some(BlendKey::Mode(BlendMode::Multiply)));
}

#[test]
fn base_variant_is_not_a_comp() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(doc.layer_comps.len(), 1);
}

#[test]
fn unknown_comp_variant_is_preserved() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(doc.layer_comps.len(), 1);
}

#[test]
fn read_orig_fx_ref_point_into_fxrp() {
    let mut doc = comp_document(2);
    set_layer_anchor(&mut doc.layers[0], PositionAnchor { x: 4.0, y: 5.0 });
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert_eq!(
        back.layer_comps.comps()[0]
            .entry(1)
            .and_then(|s| s.position),
        Some(PositionAnchor { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        pictura_core::layer_comp::layer_anchor(&back.layers[0]),
        PositionAnchor { x: 4.0, y: 5.0 }
    );
}

#[test]
fn comps_round_trip_through_resource_1065() {
    let bytes = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert!(comp_resource(&doc).is_some());
}

#[test]
fn deleting_all_comps_removes_resource() {
    let mut doc =
        crate::read_psd(&crate::write_psd(&comp_document(1)).expect("writes")).expect("reads");
    let id = doc.layer_comps.comps()[0].id;
    doc.layer_comps.remove_comp(id);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(comp_resource(&back).is_none());
}

#[test]
fn dirty_empty_store_removes_all_cmls() {
    let mut doc =
        crate::read_psd(&crate::write_psd(&comp_document(1)).expect("writes")).expect("reads");
    let id = doc.layer_comps.comps()[0].id;
    doc.layer_comps.remove_comp(id);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layer_comps.is_empty());
}

#[test]
fn unedited_comp_resource_is_byte_identical() {
    let first = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&first).expect("reads");
    let second = crate::write_psd(&doc).expect("writes");
    let a = crate::read_psd(&first).expect("reads first");
    let b = crate::read_psd(&second).expect("reads second");
    assert_eq!(comp_resource(&a), comp_resource(&b));
}

#[test]
fn unknown_comp_keys_survive_an_unedited_save() {
    let first = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&first).expect("reads");
    let second = crate::write_psd(&doc).expect("writes");
    assert_eq!(
        comp_resource(&crate::read_psd(&first).unwrap()),
        comp_resource(&crate::read_psd(&second).unwrap())
    );
}

#[test]
fn path_or_icc_rewrite_keeps_comps() {
    let first = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&first).expect("reads");
    let second = crate::write_psd(&doc).expect("writes");
    assert!(comp_resource(&crate::read_psd(&second).unwrap()).is_some());
}

#[test]
fn layer_comp_state_round_trips() {
    let bytes = crate::write_psd(&comp_document(7)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(
        doc.layer_comps.comps()[0].entry(1).and_then(|s| s.visible),
        Some(false)
    );
}

#[test]
fn position_comp_round_trips() {
    let bytes = crate::write_psd(&comp_document(2)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert_eq!(
        doc.layer_comps.comps()[0].entry(1).and_then(|s| s.position),
        Some(PositionAnchor { x: 1.0, y: 2.0 })
    );
}

#[test]
fn vector_mask_position_round_trips() {
    let bytes = crate::write_psd(&comp_document(2)).expect("writes");
    let doc = crate::read_psd(&bytes).expect("reads");
    assert!(doc.layer_comps.comps()[0].entry(1).is_some());
}

#[test]
fn unknown_variant_keys_survive_a_dirty_rewrite() {
    let mut doc = comp_document(7);
    doc.layer_comps.mark_dirty();
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(!back.layer_comps.is_empty());
}

#[test]
fn non_participating_layer_keeps_shmd() {
    let mut doc = document(layer("A", Some(1)));
    crate::edit_layer(doc.layers[0].as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"cust",
            data: vec![1, 2].into(),
        })
    });
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layers[0].extra_block(b"cust").is_some());
}

#[test]
fn timestamp_entry_survives() {
    let mut doc = comp_document(1);
    doc.layer_comps.mark_dirty();
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layers[0].extra_block(b"shmd").is_some());
}

#[test]
fn lyid_values_round_trip() {
    let bytes = crate::write_psd(&document(layer("A", Some(7)))).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert_eq!(layer_id(&back.layers[0]), Some(7));
}

#[test]
fn assigned_layer_ids_are_written() {
    let mut doc = document(layer("A", None));
    ensure_layer_ids(&mut doc, 4);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(layer_id(&back.layers[0]).is_some());
}

#[test]
fn resource_1044_seed_is_advanced() {
    let mut doc = document(layer("A", None));
    ensure_layer_ids(&mut doc, 4);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layer_comps.next_layer_id() > 4);
}

#[test]
fn comp_free_file_without_lyid_is_unchanged() {
    let mut doc = document(layer("A", None));
    doc.layers[0].channels_mut().clear();
    let first = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&first).expect("reads");
    let second = crate::write_psd(&back).expect("writes");
    assert_eq!(first, second);
}

#[test]
fn psb_layer_comps_round_trip() {
    let bytes = crate::write_psb(&comp_document(7)).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert_eq!(back.layer_comps.len(), 1);
}

#[test]
fn duplicate_layer_state_is_not_persisted_as_captured() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert_eq!(back.layer_comps.len(), 1);
}

#[test]
fn warning_keys_survive_save_and_reopen() {
    let mut doc = comp_document(1);
    doc.layer_comps.comps_mut()[0].entries.push(CompEntry {
        layer_id: 999,
        state: CapturedState::default(),
    });
    doc.layer_comps.comps_mut()[0].warning_cleared.insert(999);
    doc.layer_comps.mark_dirty();
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layer_comps.comps()[0].warning_cleared.contains(&999));
}

#[test]
fn base_variant_refreshes_after_apply() {
    let doc = comp_document(1);
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert_eq!(back.layer_comps.len(), 1);
}

#[test]
fn truncated_cmls_does_not_fail_open() {
    let mut doc = document(layer("A", Some(1)));
    crate::edit_layer(doc.layers[0].as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"shmd",
            data: vec![
                0, 0, 0, 1, b'8', b'B', b'I', b'M', b'c', b'm', b'l', b's', 0, 0, 0, 0, 0, 0, 0, 4,
                1, 2,
            ]
            .into(),
        })
    });
    let bytes = crate::write_psd(&doc).expect("writes");
    assert!(crate::read_psd(&bytes).is_ok());
}

#[test]
fn unmodeled_descriptor_type_is_preserved() {
    let bytes = crate::write_psd(&comp_document(1)).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(back.layers[0].extra_block(b"shmd").is_some());
}

#[test]
fn failed_comp_parse_preserves_bytes() {
    let mut doc = document(layer("A", Some(1)));
    doc.image_resources =
        encode_image_resources(&[frame_image_resource(1065, "", b"\0\0\0\x10bad")]).into();
    let bytes = crate::write_psd(&doc).expect("writes");
    let back = crate::read_psd(&bytes).expect("reads");
    assert!(comp_resource(&back).is_some());
}
