use super::*;
use pictura_core::{BitDepth, Channel, ColorMode, Document, PsdRect};

fn one_layer() -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let _ = doc
        .layers
        .push(pictura_codec::layer_from_parts(pictura_core::LayerParts {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            channels: [(0, 0), (1, 1), (2, 2), (-1, 3)]
                .map(|(id, c)| Channel {
                    id,
                    data: vec![c as u8; 64].into(),
                })
                .into(),
            ..Default::default()
        }));
    doc
}

/// "Open" then `edits` edits, each a new red fill named by its index.
fn edited(limit: usize, edits: usize) -> (History, Document, LayerTarget) {
    let mut history = History::default();
    history.set_state_limit_for_test(limit);
    let mut doc = one_layer();
    let target = LayerTarget::default();
    history.capture_live(&mut doc, &None, &target, "Open");
    for i in 1..=edits {
        doc.layers[0].channels_mut()[0].data = vec![i as u8; 64].into();
        history.capture_live(&mut doc, &None, &target, &format!("Edit {i}"));
    }
    (history, doc, target)
}

#[test]
fn capture_keeps_the_state_limit() {
    let (history, _, _) = edited(20, 25);
    assert_eq!(history.count(), 21);
    assert_eq!(history.depth(), 20);
    assert_eq!(history.label(20), "Edit 25");
}

#[test]
fn lowering_the_limit_drops_the_oldest() {
    let (mut history, doc, target) = edited(20, 10);
    let snapshot = Snapshot {
        doc: doc.clone(),
        selection: None,
        quick_mask: None,
    };
    history.add_snapshot("Kept", snapshot, target);
    let revision = history.revision();
    history.set_state_limit_for_test(4);
    assert!(history.apply_state_limit());
    assert_eq!(history.count(), 5);
    assert_eq!(history.depth(), 4);
    assert_eq!(history.label(0), "Edit 6");
    assert_eq!(history.label(4), "Edit 10");
    assert_eq!(history.snapshot_count(), 1);
    assert_ne!(history.revision(), revision);
    assert!(!history.apply_state_limit(), "a second pass trims nothing");
}

#[test]
fn at_the_oldest_state_redo_states_go() {
    let (mut history, _, _) = edited(20, 10);
    assert!(history.jump(0).is_some(), "the oldest state");
    history.set_state_limit_for_test(3);
    assert!(history.apply_state_limit());
    assert_eq!(history.count(), 4);
    assert_eq!(history.index(), 0);
    assert_eq!(history.label(0), "Open");
    assert_eq!(history.label(3), "Edit 3");
    assert!(history.can_redo());
}
