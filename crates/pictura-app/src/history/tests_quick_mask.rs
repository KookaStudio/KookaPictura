use super::*;
use pictura_core::{BitDepth, ColorMode};

fn selection(side: u32, coverage: u8) -> Selection {
    Selection {
        width: side,
        height: side,
        data: vec![coverage; (side * side) as usize].into(),
    }
}

fn snapshot(side: u32, seed: u8, mask: Option<Option<Selection>>) -> Snapshot {
    let mut doc = Document::new(side, side, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![seed; (side * side * 3) as usize].into();
    Snapshot {
        doc,
        selection: Some(selection(side, seed)),
        quick_mask: mask,
    }
}

#[test]
fn quick_mask_metadata_survives_deltas_and_live_restore() {
    let mut history = History::default();
    let masks = [None, Some(None), Some(Some(selection(2, 64)))];
    for (seed, mask) in masks.iter().enumerate() {
        history.capture(snapshot(2, seed as u8, mask.clone()), "Edit");
    }
    for index in [0, 2, 1, 0, 2] {
        let restored = history.jump(index).unwrap();
        assert_eq!(restored.quick_mask, masks[index]);
        assert_eq!(restored.selection, Some(selection(2, index as u8)));
    }
    let mut live = history.current_snapshot().unwrap().doc.clone();
    let mut clip = None;
    let mut target = LayerTarget::default();
    history
        .undo_live(&mut live, &mut clip, &mut target)
        .unwrap();
    assert_eq!(history.current_snapshot().unwrap().quick_mask, Some(None));
    assert_eq!(clip, Some(selection(2, 1)));
    history
        .redo_live(&mut live, &mut clip, &mut target)
        .unwrap();
    assert_eq!(history.current_snapshot().unwrap().quick_mask, masks[2]);
    assert_eq!(clip, Some(selection(2, 2)));
}

#[test]
fn quick_mask_metadata_survives_geometry_anchors_and_materialization() {
    let mut history = History::default();
    let masks = [
        Some(Some(selection(2, 64))),
        Some(None),
        Some(Some(selection(3, 192))),
        None,
    ];
    for (index, side) in [2, 2, 3, 3].into_iter().enumerate() {
        history.capture(snapshot(side, index as u8, masks[index].clone()), "Edit");
    }
    assert_eq!(history.anchor_count(), 2);
    for index in [0, 3, 1, 2, 0, 3] {
        assert_eq!(history.materialize(index).quick_mask, masks[index]);
        assert_eq!(history.jump(index).unwrap().quick_mask, masks[index]);
    }
}

#[test]
fn ordinary_captures_clear_reused_quick_mask_metadata() {
    let mut history = History::default();
    history.capture(snapshot(2, 1, Some(Some(selection(2, 64)))), "Mask");
    let mut live = history.current_snapshot().unwrap().doc.clone();
    let clip = Some(selection(2, 100));
    history.capture_live(&mut live, &clip, &LayerTarget::default(), "Ordinary");
    assert!(history.current_snapshot().unwrap().quick_mask.is_none());
    assert!(history.materialize(1).quick_mask.is_none());
    assert_eq!(
        history.undo().unwrap().quick_mask,
        Some(Some(selection(2, 64)))
    );
    assert!(history.redo().unwrap().quick_mask.is_none());
}

#[test]
fn attached_metadata_updates_stored_entry_and_preserves_shared_plane() {
    let mut history = History::default();
    let mut live = snapshot(2, 0, None).doc;
    let coverage = selection(2, 127);
    history.capture_live(&mut live, &None, &LayerTarget::default(), "Mask");
    history.set_current_quick_mask(Some(Some(coverage.clone())));
    let current = history.current_snapshot().unwrap();
    assert!(current
        .quick_mask
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .data
        .shares(&coverage.data));
    history.capture_live(&mut live, &None, &LayerTarget::default(), "Implicit full");
    history.set_current_quick_mask(Some(None));
    history.capture_live(&mut live, &None, &LayerTarget::default(), "Ordinary");
    assert_eq!(history.jump(0).unwrap().quick_mask, Some(Some(coverage)));
    assert_eq!(history.jump(1).unwrap().quick_mask, Some(None));
    history.set_current_quick_mask(None);
    history.jump(2).unwrap();
    assert!(history.jump(1).unwrap().quick_mask.is_none());
}

#[test]
fn quick_mask_metadata_survives_depth_drop_and_branch_truncation() {
    let mut history = History::default();
    history.set_byte_cap(usize::MAX);
    for seed in 0..=25 {
        history.capture(snapshot(2, seed, Some(Some(selection(2, seed)))), "Mask");
    }
    assert_eq!(history.count(), MAX_DEPTH + 1);
    for index in [0, 10, 20] {
        assert_eq!(
            history.jump(index).unwrap().quick_mask,
            Some(Some(selection(2, index as u8 + 5)))
        );
    }
    assert!(history.set_brush_source(BrushSource::State(20)));
    history.jump(10).unwrap();
    history.capture(snapshot(2, 99, Some(None)), "Branch");
    assert_eq!(history.count(), 12);
    assert_eq!(
        history.brush_source_doc().unwrap().quick_mask,
        Some(Some(selection(2, 25)))
    );
    assert_eq!(
        history.undo().unwrap().quick_mask,
        Some(Some(selection(2, 15)))
    );
    assert_eq!(history.redo().unwrap().quick_mask, Some(None));
}

#[test]
fn quick_mask_metadata_survives_purge_and_named_snapshots() {
    let mut history = History::default();
    history.capture(snapshot(2, 1, Some(Some(selection(2, 64)))), "Mask");
    history.add_snapshot(
        "Named",
        history.current_snapshot().unwrap().clone(),
        LayerTarget::default(),
    );
    history.capture(snapshot(3, 2, Some(None)), "Resize");
    history.purge_states();
    assert_eq!(history.count(), 1);
    assert_eq!(history.current_snapshot().unwrap().quick_mask, Some(None));
    assert_eq!(
        history.snapshot(0).unwrap().quick_mask,
        Some(Some(selection(2, 64)))
    );
    history.capture(snapshot(3, 3, None), "Ordinary");
    assert_eq!(history.undo().unwrap().quick_mask, Some(None));
    assert!(history.set_brush_source(BrushSource::Snapshot(0)));
    history.purge_snapshots();
    assert_eq!(history.brush_source_doc().unwrap().quick_mask, Some(None));
    history.purge();
    history.capture(snapshot(3, 4, Some(Some(selection(3, 192)))), "Mask");
    assert_eq!(history.undo().unwrap().quick_mask, Some(None));
    assert_eq!(
        history.redo().unwrap().quick_mask,
        Some(Some(selection(3, 192)))
    );
}
