use super::create::empty_group;
use super::tests::{doc_with, pixel_layer};
use crate::document_ops::layer_ops::{
    arrange_path, can_arrange_path, can_move_path_to, move_path, move_path_to, Arrange,
};

#[test]
fn move_path_to_refuses_below_background() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("mid", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    let before = doc.clone();
    assert!(!can_move_path_to(&doc, "2", "0", 1), "below the Background");
    assert!(!move_path_to(&mut doc, "2", "0", 1));
    assert_eq!(doc, before);
    assert!(can_move_path_to(&doc, "2", "0", 0), "above the Background");
    assert!(move_path_to(&mut doc, "2", "0", 0));
    assert_eq!(doc.layers[1].name, "top");
}

#[test]
fn move_path_clamps_at_background_floor() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("mid", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    assert!(!move_path(&mut doc, "1", -1), "already at the floor");
    assert!(move_path(&mut doc, "2", -5), "clamps to the floor");
    assert_eq!(doc.layers[0].name, "Background");
    assert_eq!(doc.layers[1].name, "top");
    assert_eq!(doc.layers[2].name, "mid");
}

#[test]
fn move_path_refuses_a_source_below_the_background() {
    let mut doc = doc_with(vec![
        pixel_layer("below", 4, 4, 0),
        pixel_layer("Background", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    let before = doc.clone();
    assert!(!move_path(&mut doc, "0", 1));
    assert_eq!(doc, before);
}

#[test]
fn arrange_stops_above_background() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("mid", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    assert!(arrange_path(&mut doc, "2", Arrange::Back));
    assert_eq!(doc.layers[0].name, "Background");
    assert_eq!(doc.layers[1].name, "top");
    assert!(
        !arrange_path(&mut doc, "1", Arrange::Backward),
        "at the floor"
    );
    assert!(!can_arrange_path(&doc, "1", Arrange::Backward));
    assert!(!arrange_path(&mut doc, "1", Arrange::Back), "at the floor");
    assert!(!can_arrange_path(&doc, "1", Arrange::Back));
}

#[test]
fn a_group_with_a_background_keeps_it_bottom() {
    let mut group = empty_group("Group");
    group.children = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("above", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    let mut doc = doc_with(vec![group]);
    let before = doc.clone();
    assert!(
        !can_move_path_to(&doc, "0/2", "0/0", 1),
        "below the nested Background"
    );
    assert!(!move_path_to(&mut doc, "0/2", "0/0", 1));
    assert_eq!(doc, before);
    assert!(arrange_path(&mut doc, "0/2", Arrange::Back));
    assert_eq!(doc.layers[0].children[1].name, "top");
    assert!(!move_path(&mut doc, "0/1", -1), "cannot cross");
}

#[test]
fn move_path_to_refuses_a_source_below_the_background() {
    let mut doc = doc_with(vec![
        pixel_layer("below", 4, 4, 0),
        pixel_layer("Background", 4, 4, 1),
        pixel_layer("top", 4, 4, 2),
    ]);
    let before = doc.clone();
    assert!(!can_move_path_to(&doc, "0", "", 0));
    assert!(!move_path_to(&mut doc, "0", "", 0));
    assert_eq!(doc, before);
}
