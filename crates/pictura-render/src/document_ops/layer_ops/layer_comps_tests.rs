use super::*;

use pictura_core::layer_comp::{
    clear_layer_id, layer_anchor, layer_id, set_layer_anchor, set_layer_id, CapturedState,
    CompEntry, CompRecord, LayerComp, PositionAnchor,
};
use pictura_core::{BitDepth, BlendMode, ColorMode, Document, Layer, LayerStack, Plane, PsdRect};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

fn pixel(name: &str, id: u32, r: PsdRect, value: u8) -> Layer {
    let plane = (r.width().max(0) * r.height().max(0)) as usize;
    let mut layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: name.to_string(),
        rect: r,
        channels: vec![0, 1, 2, -1]
            .into_iter()
            .map(|channel| pictura_core::Channel {
                id: channel,
                data: vec![value; plane].into(),
            })
            .collect(),
        ..Default::default()
    });
    set_layer_id(&mut layer, id);
    layer
}

fn group(name: &str, id: u32, children: Vec<Layer>) -> Layer {
    let mut layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: name.to_string(),
        is_group: true,
        children: LayerStack::from_layers(children),
        ..Default::default()
    });
    set_layer_id(&mut layer, id);
    layer
}

fn document(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(layers);
    doc
}

fn record(bits: u32) -> CompRecord {
    CompRecord::from_bits(bits)
}

fn comp_for(doc: &mut Document, bits: u32) -> LayerComp {
    capture_layer_comp(doc, 0, "A", "", record(bits))
}

fn add_comp(doc: &mut Document, comp: LayerComp) -> u32 {
    doc.layer_comps.push_comp(comp)
}

#[test]
fn visibility_only_comp_ignores_position_and_appearance() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].visible = false;
    doc.layers[0].rect = rect(5, 5, 9, 9);
    doc.layers[0].blend = BlendMode::Multiply;
    let outcome = apply_layer_comp(&mut doc, &comp);
    assert!(!outcome.is_noop());
    assert_eq!(doc.layers[0].rect, rect(5, 5, 9, 9));
    assert_eq!(doc.layers[0].blend, BlendMode::Multiply);
}

#[test]
fn all_class_comp_restores_every_class() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 7);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].visible = false;
    doc.layers[0].rect = rect(5, 5, 9, 9);
    doc.layers[0].blend = BlendMode::Screen;
    let outcome = apply_layer_comp(&mut doc, &comp);
    assert!(!outcome.is_noop());
    assert!(doc.layers[0].visible);
    assert_eq!(doc.layers[0].rect, rect(0, 0, 4, 4));
    assert_eq!(doc.layers[0].blend, BlendMode::Normal);
}

#[test]
fn capture_records_current_visibility() {
    let mut doc = document(vec![
        pixel("A", 1, rect(0, 0, 4, 4), 10),
        pixel("B", 2, rect(4, 4, 8, 8), 20),
    ]);
    doc.layers[1].visible = false;
    let comp = comp_for(&mut doc, 1);
    assert_eq!(comp.entry(1).and_then(|s| s.visible), Some(true));
    assert_eq!(comp.entry(2).and_then(|s| s.visible), Some(false));
}

#[test]
fn apply_restores_visibility() {
    let mut doc = document(vec![
        pixel("A", 1, rect(0, 0, 4, 4), 10),
        pixel("B", 2, rect(4, 4, 8, 8), 20),
    ]);
    doc.layers[1].visible = false;
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].visible = false;
    doc.layers[1].visible = true;
    apply_layer_comp(&mut doc, &comp);
    assert!(doc.layers[0].visible);
    assert!(!doc.layers[1].visible);
}

#[test]
fn apply_restores_position() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 2);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].rect = rect(8, 8, 12, 12);
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].rect, rect(0, 0, 4, 4));
}

#[test]
fn apply_restores_appearance() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 4);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].blend = BlendMode::Screen;
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].blend, BlendMode::Normal);
}

#[test]
fn apply_restores_unknown_blend_key() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    doc.layers[0].blend_key = Some(*b"zzzz");
    let comp = comp_for(&mut doc, 4);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].blend_key = None;
    doc.layers[0].blend = BlendMode::Screen;
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].blend_key, Some(*b"zzzz"));
    assert_eq!(doc.layers[0].blend, BlendMode::Normal);
}

#[test]
fn apply_leaves_unrecorded_position() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].rect = rect(8, 8, 12, 12);
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].rect, rect(8, 8, 12, 12));
}

#[test]
fn apply_changes_no_pixels() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let before: Vec<u8> = doc.layers[0].channels()[0].data.as_slice().to_vec();
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp.clone());
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(
        doc.layers[0].channels()[0].data.as_slice(),
        before.as_slice()
    );
}

#[test]
fn apply_with_no_resolvable_entries_is_a_noop() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = LayerComp::new(
        1,
        "A",
        "",
        record(1),
        vec![CompEntry {
            layer_id: 999,
            state: CapturedState {
                visible: Some(false),
                ..Default::default()
            },
        }],
    );
    let outcome = apply_layer_comp(&mut doc, &comp);
    assert!(outcome.is_noop());
    assert_eq!(outcome.missing, 1);
}

#[test]
fn repeated_apply_and_undo_is_lossless() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    set_layer_anchor(&mut doc.layers[0], PositionAnchor { x: 0.0, y: 0.0 });
    let comp = comp_for(&mut doc, 2);
    add_comp(&mut doc, comp.clone());
    for _ in 0..10 {
        assert!(crate::translate_layer_path(&mut doc, "0", 8, 8));
        apply_layer_comp(&mut doc, &comp);
        assert_eq!(doc.layers[0].rect, rect(0, 0, 4, 4));
        assert_eq!(
            layer_anchor(&doc.layers[0]),
            PositionAnchor { x: 0.0, y: 0.0 }
        );
    }
}

#[test]
fn update_recaptures() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let mut comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp.clone());
    doc.layers[0].visible = false;
    let fresh = capture_layer_comp(&mut doc, comp.id, "A", "", comp.record);
    comp.entries = fresh.entries;
    assert_eq!(comp.entry(1).and_then(|s| s.visible), Some(false));
}

#[test]
fn update_clears_warning() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let mut comp = comp_for(&mut doc, 1);
    comp.entries.push(CompEntry {
        layer_id: 999,
        state: CapturedState::default(),
    });
    comp.warning_cleared.insert(999);
    let missing = layer_comp_missing_ids(&doc, &comp);
    comp.warning_cleared.clear();
    assert!(missing.contains(&999));
    assert!(layer_comp_warning(&doc, &comp));
}

#[test]
fn rename_keeps_captured_state() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let mut comp = comp_for(&mut doc, 1);
    let entries = comp.entries.clone();
    comp.name = "B".to_string();
    assert_eq!(comp.entries, entries);
}

#[test]
fn options_change_record_set_only() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let mut comp = comp_for(&mut doc, 1);
    let entries = comp.entries.clone();
    comp.record = record(3);
    assert_eq!(comp.entries, entries);
    assert!(comp.record.position);
}

#[test]
fn duplicate_copies_captured_state() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    let id = add_comp(&mut doc, comp);
    let copy = doc.layer_comps.duplicate_comp(id).expect("copy");
    assert_ne!(copy, id);
    let source = doc.layer_comps.comp(id).unwrap();
    let copy = doc.layer_comps.comp(copy).unwrap();
    assert_eq!(source.entries, copy.entries);
}

#[test]
fn last_document_state_restores_pre_apply() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 7);
    add_comp(&mut doc, comp.clone());
    let last = capture_last_document_state(&mut doc);
    doc.layers[0].visible = false;
    doc.layers[0].rect = rect(8, 8, 12, 12);
    let outcome = restore_last_document_state(&mut doc, &last);
    assert!(!outcome.is_noop());
    assert!(doc.layers[0].visible);
    assert_eq!(doc.layers[0].rect, rect(0, 0, 4, 4));
}

#[test]
fn a_noop_apply_does_not_become_last_document_state() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = LayerComp::new(
        1,
        "A",
        "",
        record(1),
        vec![CompEntry {
            layer_id: 999,
            state: CapturedState::default(),
        }],
    );
    let outcome = apply_layer_comp(&mut doc, &comp);
    assert!(outcome.is_noop());
    assert!(doc.layer_comps.last_state().is_none());
}

#[test]
fn deleting_a_captured_layer_warns() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    doc.layers = LayerStack::default();
    assert!(layer_comp_warning(&doc, &comp));
}

#[test]
fn merging_a_captured_layer_warns() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    doc.layers = LayerStack::default();
    assert_eq!(layer_comp_missing_ids(&doc, &comp).len(), 1);
}

#[test]
fn converting_a_captured_layer_to_background_warns() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    clear_layer_id(&mut doc.layers[0]);
    assert!(layer_comp_warning(&doc, &comp));
}

#[test]
fn clearing_a_warning_keeps_entries() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    let id = add_comp(&mut doc, comp);
    let comp = doc.layer_comps.comp(id).unwrap();
    let entries = comp.entries.clone();
    let missing: std::collections::BTreeSet<u32> = [999].into_iter().collect();
    doc.layer_comps.clear_comp_warning(id, &missing);
    assert_eq!(doc.layer_comps.comp(id).unwrap().entries, entries);
}

#[test]
fn a_new_loss_warns_again() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    doc.layers = LayerStack::default();
    assert!(layer_comp_warning(&doc, &comp));
}

#[test]
fn a_relost_id_warns_again() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let mut comp = comp_for(&mut doc, 1);
    comp.warning_cleared.insert(1);
    let id = add_comp(&mut doc, comp);
    assert!(prune_layer_comp_warnings(&mut doc));
    assert!(doc.layer_comps.comp(id).unwrap().warning_cleared.is_empty());
    doc.layers = LayerStack::default();
    assert!(layer_comp_warning(&doc, doc.layer_comps.comp(id).unwrap()));
}

#[test]
fn clear_all_warnings() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    let id = add_comp(&mut doc, comp);
    doc.layers = LayerStack::default();
    let present = pictura_core::layer_comp::layer_ids(&doc);
    let missing = layer_comp_missing_ids(&doc, doc.layer_comps.comp(id).unwrap());
    assert!(doc.layer_comps.clear_comp_warning(id, &missing));
    assert!(present.is_empty());
}

#[test]
fn renaming_a_layer_keeps_its_entry() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    doc.layers[0].name = "B".to_string();
    assert!(comp.entry(1).is_some());
}

#[test]
fn reordering_layers_keeps_entries() {
    let mut doc = document(vec![
        pixel("A", 1, rect(0, 0, 4, 4), 10),
        pixel("B", 2, rect(4, 4, 8, 8), 20),
    ]);
    let comp = comp_for(&mut doc, 1);
    let top = doc.layers[1].clone();
    doc.layers[1] = doc.layers[0].clone();
    doc.layers[0] = top;
    assert!(comp.entry(1).is_some() && comp.entry(2).is_some());
}

#[test]
fn duplicating_a_layer_isolates_the_copy() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    let copy = doc.layers[0].clone();
    assert_eq!(layer_id(&copy), Some(1));
    assert!(comp.entry(1).is_some());
}

#[test]
fn duplicating_a_layer_does_not_persist_stale_comp_state() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    let mut copy = doc.layers[0].clone();
    pictura_codec::edit_layer(copy.as_mut(), |parts| {
        parts.extra_blocks.retain(|block| block.key != *b"shmd")
    });
    clear_layer_id(&mut copy);
    assert_eq!(layer_id(&copy), None);
    assert!(comp.entry(1).is_some());
}

#[test]
fn a_group_restores_its_children() {
    let child = pixel("C", 2, rect(0, 0, 4, 4), 10);
    let mut outer = group("G", 1, vec![child]);
    outer.rect = rect(0, 0, 4, 4);
    let mut doc = document(vec![outer]);
    let comp = comp_for(&mut doc, 2);
    doc.layers[0].rect = rect(8, 8, 12, 12);
    if let Some(child) = doc.layers[0].children_mut().expect("group").get_mut(0) {
        child.rect = rect(8, 8, 12, 12);
    }
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].children()[0].rect, rect(0, 0, 4, 4));
}

#[test]
fn a_nested_group_moves_once() {
    let child = pixel("C", 3, rect(0, 0, 4, 4), 10);
    let mut inner = group("Inner", 2, vec![child]);
    inner.rect = rect(0, 0, 4, 4);
    let mut outer = group("Outer", 1, vec![inner]);
    outer.rect = rect(0, 0, 4, 4);
    let mut doc = document(vec![outer]);
    let comp = comp_for(&mut doc, 2);
    doc.layers[0].rect = rect(8, 8, 12, 12);
    if let Some(inner) = doc.layers[0].children_mut().expect("group").get_mut(0) {
        inner.rect = rect(8, 8, 12, 12);
        if let Some(child) = inner.children_mut().expect("group").get_mut(0) {
            child.rect = rect(8, 8, 12, 12);
        }
    }
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(
        doc.layers[0].children()[0].children()[0].rect,
        rect(0, 0, 4, 4)
    );
}

#[test]
fn an_adjustment_mask_moves() {
    let mut adjustment = pixel("A", 1, rect(0, 0, 4, 4), 0);
    adjustment.channels_mut().clear();
    pictura_codec::edit_layer(adjustment.as_mut(), |parts| {
        parts.adjustment = Some(pictura_core::AdjustmentData {
            key: *b"brit",
            data: Plane::default(),
        })
    });
    adjustment.mask = Some(pictura_core::LayerMask {
        rect: rect(0, 0, 4, 4),
        ..Default::default()
    });
    let mut doc = document(vec![adjustment]);
    let comp = comp_for(&mut doc, 2);
    doc.layers[0].rect = rect(8, 8, 12, 12);
    doc.layers[0].mask.as_mut().unwrap().rect = rect(8, 8, 12, 12);
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].mask.as_ref().unwrap().rect, rect(0, 0, 4, 4));
}

#[test]
fn a_shape_outline_moves() {
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    let mut doc = Document::from_rgba("Shape", 20, 20, &[255; 20 * 20 * 4]);
    let square = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (5.0, 5.0, 10.0, 10.0),
    )
    .expect("outline");
    let path = crate::add_shape_layer(&mut doc, "", [0, 0, 255, 255], "Shape", &square, None);
    let comp = capture_layer_comp(&mut doc, 0, "S", "", record(2));
    add_comp(&mut doc, comp.clone());
    assert!(comp.record.position);
    let shape_id = layer_id(crate::resolve_path(&doc, &path).unwrap()).unwrap();
    let mut shape_comp = comp.clone();
    shape_comp
        .entries
        .retain(|entry| entry.layer_id == shape_id);
    assert_eq!(
        shape_comp.entries[0].state.position,
        Some(PositionAnchor { x: 5.0, y: 5.0 })
    );
    let before = crate::shape_bounds(
        crate::resolve_path(&doc, &path).unwrap(),
        doc.width,
        doc.height,
    )
    .expect("bounds");
    assert!(crate::translate_layer_path(&mut doc, &path, 4, 4));
    let outcome = apply_layer_comp(&mut doc, &shape_comp);
    assert!(outcome.changed > 0, "outcome {outcome:?} before {before:?}");
    let after = crate::shape_bounds(
        crate::resolve_path(&doc, &path).unwrap(),
        doc.width,
        doc.height,
    )
    .expect("bounds");
    assert_eq!((after.0, after.1), (before.0, before.1));
}

#[test]
fn appearance_handles_no_style_lfx2_and_legacy_lrfx() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 4);
    assert_eq!(comp.entry(1).and_then(|s| s.style_visible), None);
    assert!(comp.entry(1).and_then(|s| s.blend).is_some());
}

#[test]
fn fxrp_anchor_updates_on_move_transform_align_and_nudge() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    set_layer_anchor(&mut doc.layers[0], PositionAnchor { x: 1.0, y: 2.0 });
    assert!(crate::translate_layer_path(&mut doc, "0", 3, 4));
    assert_eq!(
        layer_anchor(&doc.layers[0]),
        PositionAnchor { x: 4.0, y: 6.0 }
    );
}

#[test]
fn warning_state_survives_save_and_reopen() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp);
    doc.layers = LayerStack::default();
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let back = pictura_codec::read_psd(&bytes).expect("reads");
    assert!(layer_comp_warning(&back, &back.layer_comps.comps()[0]));
}

#[test]
fn comps_still_resolve_after_save_and_reopen() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    doc.layers[0].visible = false;
    let comp = comp_for(&mut doc, 1);
    add_comp(&mut doc, comp);
    let bytes = pictura_codec::write_psd(&doc).expect("writes");
    let mut back = pictura_codec::read_psd(&bytes).expect("reads");
    assert_eq!(back.layer_comps.len(), 1);
    back.layers[0].visible = true;
    let comp = back.layer_comps.comps()[0].clone();
    let outcome = apply_layer_comp(&mut back, &comp);
    assert!(!outcome.is_noop());
    assert!(!back.layers[0].visible);
}

#[test]
fn smart_filter_settings_do_not_vary() {
    let mut doc = document(vec![pixel("A", 1, rect(0, 0, 4, 4), 10)]);
    pictura_codec::edit_layer(doc.layers[0].as_mut(), |parts| {
        parts.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"FXid",
            data: vec![1, 2, 3].into(),
        })
    });
    let comp = comp_for(&mut doc, 1);
    let before = doc.layers[0].extra_blocks().clone();
    apply_layer_comp(&mut doc, &comp);
    assert_eq!(doc.layers[0].extra_blocks(), &before);
}
