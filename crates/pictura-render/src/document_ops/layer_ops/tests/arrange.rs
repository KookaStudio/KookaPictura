use super::*;

#[test]
fn arrange_moves_within_container_and_refuses_edges() {
    fn names(doc: &Document) -> Vec<&str> {
        doc.layers.iter().map(|l| l.name.as_str()).collect()
    }
    let layers = || {
        vec![
            pixel_layer("bottom", 4, 4, 0),
            pixel_layer("middle", 4, 4, 1),
            pixel_layer("top", 4, 4, 2),
        ]
    };

    let mut doc = doc_with(layers());
    assert!(arrange_path(&mut doc, "0", Arrange::Front));
    assert_eq!(names(&doc), ["middle", "top", "bottom"]);

    let mut doc = doc_with(layers());
    assert!(arrange_path(&mut doc, "2", Arrange::Back));
    assert_eq!(names(&doc), ["top", "bottom", "middle"]);

    let mut doc = doc_with(layers());
    assert!(arrange_path(&mut doc, "0", Arrange::Forward));
    assert_eq!(names(&doc), ["middle", "bottom", "top"]);

    let mut doc = doc_with(layers());
    assert!(arrange_path(&mut doc, "2", Arrange::Backward));
    assert_eq!(names(&doc), ["bottom", "top", "middle"]);

    // Forward then Backward on the same middle layer is an inverse pair.
    let mut doc = doc_with(layers());
    assert!(arrange_path(&mut doc, "1", Arrange::Forward));
    assert_eq!(names(&doc), ["bottom", "top", "middle"]);
    assert!(arrange_path(&mut doc, "2", Arrange::Backward));
    assert_eq!(names(&doc), ["bottom", "middle", "top"]);

    // The front layer cannot move further forward; the command is disabled.
    let mut doc = doc_with(layers());
    assert!(!arrange_path(&mut doc, "2", Arrange::Forward));
    assert!(!can_arrange_path(&doc, "2", Arrange::Forward));
    assert!(!arrange_path(&mut doc, "2", Arrange::Front));
    assert!(!can_arrange_path(&doc, "2", Arrange::Front));
    assert!(can_arrange_path(&doc, "0", Arrange::Front));
}

#[test]
fn arrange_refuses_background_and_locked() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        locked_layer("locked"),
        pixel_layer("normal", 4, 4, 3),
    ]);
    let before = doc.clone();
    for arrange in [
        Arrange::Front,
        Arrange::Forward,
        Arrange::Backward,
        Arrange::Back,
    ] {
        assert!(!arrange_path(&mut doc, "0", arrange));
        assert!(!can_arrange_path(&doc, "0", arrange));
    }
    assert!(!arrange_path(&mut doc, "1", Arrange::Front));
    assert!(!can_arrange_path(&doc, "1", Arrange::Front));
    assert_eq!(doc, before, "Background and locked nodes do not reorder");

    assert!(arrange_path(&mut doc, "2", Arrange::Back));
    assert_eq!(doc.layers[0].name, "Background");
    assert_eq!(doc.layers[1].name, "normal");
    assert_eq!(doc.layers[2].name, "locked");
}

#[test]
fn arrange_stays_within_the_group() {
    let mut group = empty_group("Group 1");
    let _ = group
        .children_mut()
        .expect("group")
        .push(pixel_layer("c0", 4, 4, 0));
    let _ = group
        .children_mut()
        .expect("group")
        .push(pixel_layer("c1", 4, 4, 1));
    let mut doc = doc_with(vec![pixel_layer("outer", 4, 4, 2), group]);

    assert!(arrange_path(&mut doc, "1/0", Arrange::Front));
    assert_eq!(doc.layers[1].children()[1].name, "c0");
    assert_eq!(
        doc.layers[0].name, "outer",
        "the layer did not leave its group"
    );
}

#[test]
fn reverse_flips_a_contiguous_run_and_refuses_otherwise() {
    let mut doc = doc_with(vec![
        pixel_layer("a", 4, 4, 0),
        pixel_layer("b", 4, 4, 1),
        pixel_layer("c", 4, 4, 2),
        pixel_layer("d", 4, 4, 3),
    ]);
    assert!(reverse_paths(&mut doc, &["0", "1", "2"]));
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["c", "b", "a", "d"]);

    let before = doc.clone();
    assert!(!reverse_paths(&mut doc, &["0", "2"]), "non-contiguous");
    assert_eq!(doc, before);
    assert!(!reverse_paths(&mut doc, &["0"]), "fewer than two nodes");
    assert!(!reverse_paths(&mut doc, &["0", "9"]), "only one resolves");
    assert_eq!(doc, before);
}

#[test]
fn reverse_refuses_cross_container_and_background() {
    let mut group = empty_group("Group 1");
    let _ = group
        .children_mut()
        .expect("group")
        .push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![pixel_layer("outer", 4, 4, 0), group]);
    let before = doc.clone();
    assert!(!reverse_paths(&mut doc, &["0", "1/0"]), "cross-container");
    assert_eq!(doc, before);

    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("b", 4, 4, 1),
    ]);
    let before = doc.clone();
    assert!(!reverse_paths(&mut doc, &["0", "1"]), "Background refuses");
    assert_eq!(doc, before);
}
