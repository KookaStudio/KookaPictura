use super::*;

fn layered_doc(count: usize) -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let layers = (0..count)
        .map(|index| Layer {
            name: format!("layer {index}"),
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            ..Layer::default()
        })
        .collect();
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

#[test]
fn undo_restores_layer_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("0");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("1"));
}

#[test]
fn undo_keeps_a_later_selection() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("2");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}

#[test]
fn redo_restores_post_operation_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Edit");
    let mut target = LayerTarget::only("0");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    history.redo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("0"));
}

#[test]
fn jump_restores_target_across_multiple_states() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("0"), "Open");
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Edit 1");
    history.capture_live(&mut live, &selection, &LayerTarget::only("2"), "Edit 2");
    let mut target = LayerTarget::only("2");
    let mut no_selection: Option<Selection> = None;
    let _ = history.jump_live(0, &mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("0"));
    let _ = history.jump_live(2, &mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}

#[test]
fn named_snapshot_restores_layer_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Open");
    history.add_snapshot("Named", snap(3), LayerTarget::only("1"));
    assert_eq!(history.snapshot_target(0), Some(LayerTarget::only("1")));
}

#[test]
fn undo_restores_structural_target() {
    let mut live = layered_doc(3);
    let selection = None;
    let mut history = History::default();
    history.capture_live(&mut live, &selection, &LayerTarget::only("2"), "Open");
    live.layers =
        pictura_core::LayerStack::from_layers(live.layers.iter().take(2).cloned().collect());
    history.capture_live(&mut live, &selection, &LayerTarget::only("1"), "Delete");
    let mut target = LayerTarget::only("1");
    let mut no_selection: Option<Selection> = None;
    history.undo_live(&mut live, &mut no_selection, &mut target);
    assert_eq!(target, LayerTarget::only("2"));
}
