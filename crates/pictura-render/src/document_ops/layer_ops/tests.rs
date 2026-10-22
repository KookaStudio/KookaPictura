use super::create::empty_group;
use super::*;
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
    PsdRect,
};

pub(super) fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h,
        right: w,
    }
}

pub(super) fn pixel_layer(name: &str, w: u32, h: u32, value: u8) -> Layer {
    let n = w as usize * h as usize;
    Layer {
        name: name.into(),
        rect: rect(w as i32, h as i32),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![value; n].into(),
            },
            Channel {
                id: 1,
                data: vec![value; n].into(),
            },
            Channel {
                id: 2,
                data: vec![value; n].into(),
            },
            Channel {
                id: -1,
                data: vec![255; n].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: name == "Background",
        ..Default::default()
    }
}

pub(super) fn doc_with(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

#[test]
fn add_layer_inserts_above_and_is_transparent() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let index = add_layer(&mut doc, 0, "Layer 1");

    assert_eq!(index, 1);
    assert_eq!(doc.layers.len(), 2);
    let added = &doc.layers[1];
    assert_eq!(added.name, "Layer 1");
    assert_eq!(added.rect, rect(4, 4));
    assert_eq!(added.blend, BlendMode::Normal);
    assert_eq!((added.opacity, added.fill), (255, 255));
    assert!(added.visible && !added.is_group);
    assert_eq!(added.channels.len(), 4);
    for (id, channel) in [(0i16, 0), (1, 1), (2, 2), (-1, 3)] {
        assert_eq!(added.channels[channel].id, id);
        assert_eq!(added.channels[channel].data, vec![0u8; 16]);
    }
}

#[test]
fn add_layer_without_selection_goes_to_top() {
    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 1), pixel_layer("b", 4, 4, 2)]);
    assert_eq!(
        add_layer(&mut doc, -1, "top"),
        2,
        "no selection inserts on top"
    );
    assert_eq!(doc.layers[2].name, "top");

    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 1)]);
    assert_eq!(
        add_layer(&mut doc, 99, "top"),
        1,
        "out-of-range inserts on top"
    );
}

#[test]
fn add_group_inserts_empty_group() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let index = add_group(&mut doc, 0, "Group 1");

    assert_eq!(index, 1);
    let group = &doc.layers[1];
    assert!(group.is_group);
    assert!(group.children.is_empty());
    assert_eq!(group.rect, rect(0, 0));
    assert_eq!(group.blend, BlendMode::Normal);
}

#[test]
fn duplicate_layer_deep_copies_above_the_source() {
    let mask = LayerMask {
        rect: rect(4, 4),
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(vec![128; 16].into()),
        ..Default::default()
    };
    let mut original = pixel_layer("base", 4, 4, 40);
    original.mask = Some(mask);
    original.color = ColorLabel::Red;
    original.opacity = 200;
    let mut doc = doc_with(vec![original, pixel_layer("top", 4, 4, 80)]);

    let index = duplicate_layer(&mut doc, 0);
    assert_eq!(index, 1);
    assert_eq!(doc.layers.len(), 3);
    assert_eq!(doc.layers[1].name, "base copy");
    assert_eq!(doc.layers[1].mask, doc.layers[0].mask);
    assert_eq!(doc.layers[1].color, ColorLabel::Red);
    assert_eq!(doc.layers[1].opacity, 200);

    doc.layers[1].channels[0].data[0] = 7;
    assert_eq!(doc.layers[0].channels[0].data[0], 40, "deep copy");
}

#[test]
fn duplicate_group_copies_children() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 30));
    let mut doc = doc_with(vec![group]);

    assert_eq!(duplicate_layer(&mut doc, 0), 1);
    assert_eq!(doc.layers[1].name, "Group 1 copy");
    assert!(doc.layers[1].is_group);
    assert_eq!(doc.layers[1].children.len(), 1);
    assert_eq!(doc.layers[1].children[0].name, "child");
}

#[test]
fn duplicate_rejects_out_of_range() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 1)]);
    assert_eq!(duplicate_layer(&mut doc, -1), -1);
    assert_eq!(duplicate_layer(&mut doc, 9), -1);
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn group_layer_wraps_in_place() {
    let mut doc = doc_with(vec![
        pixel_layer("a", 4, 4, 1),
        pixel_layer("b", 4, 4, 2),
        pixel_layer("c", 4, 4, 3),
    ]);
    let index = group_layer(&mut doc, 1);

    assert_eq!(index, 1);
    assert_eq!(doc.layers.len(), 3);
    let group = &doc.layers[1];
    assert!(group.is_group);
    assert_eq!(group.name, "Group 1");
    assert_eq!(group.children.len(), 1);
    assert_eq!(group.children[0].name, "b");
    assert_eq!(doc.layers[0].name, "a");
    assert_eq!(doc.layers[2].name, "c");
}

#[test]
fn group_layer_refuses_the_background() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("top", 4, 4, 1),
    ]);
    let before = doc.clone();
    assert_eq!(group_layer(&mut doc, 0), -1);
    assert_eq!(doc, before);
}

#[test]
fn group_layer_names_after_existing_groups() {
    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 1)]);
    group_layer(&mut doc, 0);
    // Wrap the new group again: the next name must skip the existing one.
    assert_eq!(doc.layers[0].name, "Group 1");
    let _ = doc.layers.push(pixel_layer("b", 4, 4, 2));
    group_layer(&mut doc, 1);
    assert_eq!(doc.layers[1].name, "Group 2");
}

#[test]
fn ungroup_splices_children_in_order() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("a", 4, 4, 1));
    let _ = group.children.push(pixel_layer("b", 4, 4, 2));
    let mut doc = doc_with(vec![
        pixel_layer("bottom", 4, 4, 0),
        group,
        pixel_layer("top", 4, 4, 3),
    ]);

    assert!(ungroup_layer(&mut doc, 1));
    assert_eq!(doc.layers.len(), 4);
    let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["bottom", "a", "b", "top"]);
}

#[test]
fn ungroup_rejects_non_group() {
    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 1)]);
    assert!(!ungroup_layer(&mut doc, 0));
    assert!(!ungroup_layer(&mut doc, -1));
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn next_layer_name_uses_highest_suffix() {
    let mut doc = doc_with(vec![
        pixel_layer("Layer 1", 4, 4, 1),
        pixel_layer("Layer 5", 4, 4, 2),
        pixel_layer("Group 2", 4, 4, 3),
    ]);
    let _ = doc.layers[1].children.push(pixel_layer("Layer 9", 4, 4, 4));
    assert_eq!(next_layer_name(&doc, "Layer"), "Layer 10");
    assert_eq!(next_layer_name(&doc, "Group"), "Group 3");

    let empty = doc_with(Vec::new());
    assert_eq!(next_layer_name(&empty, "Layer"), "Layer 1");
}

#[test]
fn transparent_layer_does_not_change_the_composite() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let before = crate::composite_rgba(&doc);

    let index = add_layer(&mut doc, 0, "Layer 1");
    assert!(index >= 0);
    let after = crate::composite_rgba(&doc);

    assert_eq!(before, after, "a fully transparent layer must be inert");
    assert!(after.data.iter().any(|&b| b != 0), "base layer is visible");
}

fn locked_layer(name: &str) -> Layer {
    let mut layer = pixel_layer(name, 4, 4, 1);
    layer.lock = LockFlags::all();
    layer
}

fn sample_doc() -> Document {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 2));
    doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        locked_layer("locked"),
        group,
        pixel_layer("normal", 4, 4, 3),
    ])
}

fn visible_names(doc: &Document) -> Vec<String> {
    fn walk(layers: &[Layer], out: &mut Vec<String>) {
        for layer in layers {
            if layer.visible {
                out.push(layer.name.clone());
            }
            walk(&layer.children, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.layers, &mut out);
    out
}

#[test]
fn path_resolve_nested() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("a", 4, 4, 1));
    let _ = group.children.push(pixel_layer("b", 4, 4, 2));
    let mut doc = doc_with(vec![pixel_layer("bottom", 4, 4, 0), group]);

    assert_eq!(resolve_path(&doc, "0").unwrap().name, "bottom");
    assert_eq!(resolve_path(&doc, "1/0").unwrap().name, "a");
    assert_eq!(resolve_path(&doc, "1/1").unwrap().name, "b");
    assert!(resolve_path(&doc, "1/2").is_none(), "out of range child");
    assert_eq!(parent_path("1/1"), Some("1"));
    assert_eq!(parent_path("0"), None);

    resolve_path_mut(&mut doc, "1/0").unwrap().opacity = 100;
    assert_eq!(doc.layers[1].children[0].opacity, 100);
}

#[test]
fn path_resolve_rejects() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("a", 4, 4, 1));
    let mut doc = doc_with(vec![pixel_layer("bottom", 4, 4, 0), group]);

    for bad in [
        "", "01", "-1", "1/", "/1", "1//2", "1/a", "+1", " 1", "1/999", "999",
    ] {
        assert!(resolve_path(&doc, bad).is_none(), "resolve {bad:?}");
        assert!(
            resolve_path_mut(&mut doc, bad).is_none(),
            "resolve_mut {bad:?}"
        );
    }
    assert_eq!(parent_path(""), None);
    assert_eq!(parent_path("01"), None);
}

#[test]
fn flatten_rows_is_topmost_first() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("c0", 4, 4, 1));
    let _ = group.children.push(pixel_layer("c1", 4, 4, 2));
    let doc = doc_with(vec![
        pixel_layer("bottom", 4, 4, 0),
        group,
        pixel_layer("top", 4, 4, 3),
    ]);

    let rows = flatten_rows(&doc);
    let expected: Vec<(String, u32)> = vec![
        ("2".into(), 0),
        ("1".into(), 0),
        ("1/1".into(), 1),
        ("1/0".into(), 1),
        ("0".into(), 0),
    ];
    assert_eq!(rows, expected);
}

#[test]
fn is_background_reads_the_flag_not_position_or_name() {
    // A flagged layer is the Background wherever it sits and whatever it is
    // called; the flag is the only source of truth.
    let mut flagged = pixel_layer("Not Background", 4, 4, 0);
    flagged.background = true;
    let doc = doc_with(vec![pixel_layer("Layer 1", 4, 4, 1), flagged]);
    assert!(is_background(&doc, "1"));
    assert!(!is_background(&doc, "0"));
    assert!(!is_background(&doc, "9"), "missing path");

    // An unflagged bottom layer named "Background" is not the Background.
    let mut bottom = pixel_layer("Background", 4, 4, 0);
    bottom.background = false;
    let doc = doc_with(vec![bottom]);
    assert!(!is_background(&doc, "0"));

    // A missing path resolves to false rather than panicking.
    assert!(!is_background(&doc, "1/2"));
}

#[test]
fn background_from_layer_flags_opaque_and_moves_to_bottom() {
    let mut top = pixel_layer("Base", 4, 4, 50);
    top.channels[3].data[0] = 0; // one fully transparent pixel
    let mut doc = doc_with(vec![pixel_layer("other", 4, 4, 10), top]);

    assert!(background_from_layer(&mut doc, "1"));
    assert_eq!(doc.layers.len(), 2);
    let moved = &doc.layers[0];
    assert_eq!(moved.name, "Base", "node keeps its pixels and name");
    assert!(moved.background);
    assert_eq!(doc.layers[1].name, "other");
    assert_eq!(
        moved.channels[3].data[0], 255,
        "transparent pixel becomes opaque"
    );
    assert_eq!(
        moved.channels[0].data[0], 255,
        "and takes the white background"
    );
    assert_eq!(moved.channels[1].data[0], 255);
    assert_eq!(moved.channels[2].data[0], 255);
    assert_eq!(moved.channels[0].data[1], 50, "opaque pixels are untouched");
    assert_eq!(moved.channels[3].data[1], 255);
}

#[test]
fn background_from_layer_refuses_non_raster_and_existing_background() {
    let mut group = empty_group("Group");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![group]);
    assert!(!background_from_layer(&mut doc, "0"));

    let mut adj = pixel_layer("adj", 4, 4, 1);
    adj.channels.clear();
    adj.adjustment = Some(crate::encode_invert());
    let mut doc = doc_with(vec![adj]);
    assert!(!background_from_layer(&mut doc, "0"));

    let bg = pixel_layer("Background", 4, 4, 1);
    let mut doc = doc_with(vec![bg]);
    assert!(!background_from_layer(&mut doc, "0"));
}

#[test]
fn layer_from_background_clears_flag_and_unlocks() {
    let mut flagged = pixel_layer("Background", 4, 4, 1);
    flagged.background = true;
    flagged.lock = LockFlags::all();
    let mut doc = doc_with(vec![flagged]);

    assert!(layer_from_background(&mut doc, "0"));
    assert!(!doc.layers[0].background);
    assert_eq!(doc.layers[0].lock.bits(), 0, "unlocked");
    assert_eq!(
        doc.layers[0].name, "Layer 1",
        "renamed to the next free name"
    );
    assert!(
        doc.layers[0].channels.iter().any(|c| c.id == -1),
        "it can take transparency"
    );

    let before = doc.clone();
    assert!(!layer_from_background(&mut doc, "0"), "not a background");
    assert_eq!(doc, before);
}

#[test]
fn visibility_applies_everywhere() {
    let mut doc = sample_doc();
    assert_eq!(set_visible_paths(&mut doc, &["0", "1", "2", "3"], false), 4);
    assert!(doc.layers.iter().all(|layer| !layer.visible));
    assert_eq!(set_visible_paths(&mut doc, &["0", "1", "2", "3"], false), 0);
}

#[test]
fn blend_and_opacity_skip_background_and_locked() {
    let mut doc = sample_doc();
    assert_eq!(
        set_blend_paths(&mut doc, &["0", "1", "2", "3"], BlendMode::Multiply),
        2
    );
    assert_eq!(doc.layers[0].blend, BlendMode::Normal, "Background skipped");
    assert_eq!(doc.layers[1].blend, BlendMode::Normal, "locked skipped");
    assert_eq!(doc.layers[2].blend, BlendMode::Multiply, "group applies");
    assert_eq!(doc.layers[3].blend, BlendMode::Multiply);

    let mut doc = sample_doc();
    assert_eq!(set_opacity_paths(&mut doc, &["0", "1", "2", "3"], 100), 2);
    assert_eq!(doc.layers[0].opacity, 255);
    assert_eq!(doc.layers[1].opacity, 255);
    assert_eq!(doc.layers[2].opacity, 100);
    assert_eq!(doc.layers[3].opacity, 100);
}

#[test]
fn fill_skips_groups() {
    let mut doc = sample_doc();
    assert_eq!(set_fill_paths(&mut doc, &["0", "1", "2", "3"], 100), 1);
    assert_eq!(doc.layers[0].fill, 255, "Background skipped");
    assert_eq!(doc.layers[1].fill, 255, "locked skipped");
    assert_eq!(doc.layers[2].fill, 255, "group has no Fill");
    assert_eq!(doc.layers[3].fill, 100);
}

#[test]
fn lock_skips_background_but_applies_to_locked() {
    let mut doc = sample_doc();
    assert_eq!(
        set_lock_paths(&mut doc, &["0", "1"], LockFlags::PIXELS, false),
        1
    );
    assert_eq!(doc.layers[0].lock.bits(), 0, "Background lock untouched");
    assert!(!doc.layers[1].lock.contains(LockFlags::PIXELS));
}

#[test]
fn color_skips_background_applies_to_locked() {
    let mut doc = sample_doc();
    assert_eq!(
        set_color_paths(&mut doc, &["0", "1", "3"], ColorLabel::Red),
        2
    );
    assert_eq!(doc.layers[0].color, ColorLabel::None, "Background skipped");
    assert_eq!(doc.layers[1].color, ColorLabel::Red, "locked applies");
    assert_eq!(doc.layers[3].color, ColorLabel::Red);
}

#[test]
fn lock_group_layers_locks_descendants_only() {
    let mut doc = sample_doc();
    // Group 1 is layer index 2 with one child at path "2/0".
    assert_eq!(lock_group_layers(&mut doc, "2"), 1);
    assert!(doc.layers[2].children[0].lock.is_all());
    assert_eq!(
        doc.layers[2].lock.bits(),
        0,
        "the group node itself is untouched"
    );
    assert_eq!(
        lock_group_layers(&mut doc, "0"),
        0,
        "a non-group has no descendants"
    );
    assert_eq!(
        lock_group_layers(&mut doc, "9"),
        0,
        "an unknown path is refused"
    );
}

#[test]
fn lock_group_layers_covers_nested_descendants() {
    let mut inner = empty_group("Inner");
    let _ = inner.children.push(pixel_layer("deep", 4, 4, 1));
    let mut outer = empty_group("Outer");
    let _ = outer.children.push(pixel_layer("mid", 4, 4, 2));
    let _ = outer.children.push(inner);
    let mut doc = doc_with(vec![outer]);
    assert_eq!(lock_group_layers(&mut doc, "0"), 3);
    assert!(
        !doc.layers[0].lock.is_all(),
        "the outer group is not locked"
    );
    assert!(doc.layers[0].children[0].lock.is_all());
    assert!(doc.layers[0].children[1].lock.is_all());
    assert!(doc.layers[0].children[1].children[0].lock.is_all());
}

#[test]
fn delete_skips_background_and_locked() {
    let mut doc = sample_doc();
    assert_eq!(delete_paths(&mut doc, &["0", "1"]), 0);
    assert_eq!(doc.layers.len(), 4);
    assert_eq!(delete_paths(&mut doc, &["2", "3"]), 2);
    assert_eq!(doc.layers.len(), 2);
}

#[test]
fn duplicate_applies_everywhere() {
    let mut doc = sample_doc();
    let created = duplicate_paths(&mut doc, &["0", "1"]);
    assert_eq!(created.len(), 2, "Background and locked both duplicate");
    let names: Vec<&str> = doc.layers.iter().map(|layer| layer.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Background",
            "Background copy",
            "locked",
            "locked copy",
            "Group 1",
            "normal"
        ]
    );
    assert!(doc.layers[0].background);
    let copy = &doc.layers[1];
    assert!(!copy.background, "one Background; its copy is ordinary");
    assert_eq!(copy.lock.bits(), 0, "and unlocked");
    assert!(copy.channels.iter().any(|c| c.id == -1));
    assert_eq!(
        doc.layers[3].lock, doc.layers[2].lock,
        "other copies keep locks"
    );
}

#[test]
fn duplicating_a_background_makes_an_unlocked_layer_with_alpha() {
    let mut background = pixel_layer("Background", 4, 4, 9);
    background.channels.retain(|c| c.id != -1);
    background.background = true;
    background.lock = LockFlags::default()
        .with(LockFlags::TRANSPARENCY, true)
        .with(LockFlags::POSITION, true);
    let mut doc = doc_with(vec![background]);
    assert_eq!(duplicate_layer(&mut doc, 0), 1);
    let copy = &doc.layers[1];
    assert!(!copy.background && copy.lock.bits() == 0);
    let alpha = copy.channels.iter().find(|c| c.id == -1).unwrap();
    assert_eq!(&alpha.data[..], &[255; 16]);
    assert!(doc.layers[0].background);
}

#[test]
fn ungroup_skips_non_groups() {
    let mut doc = sample_doc();
    assert_eq!(ungroup_paths(&mut doc, &["0", "3"]), 0);
    assert_eq!(ungroup_paths(&mut doc, &["2"]), 1);
    assert_eq!(doc.layers.len(), 4);
    assert_eq!(doc.layers[2].name, "child");
}

#[test]
fn group_paths_refuses_cross_container_and_succeeds_within_one() {
    let mut doc = sample_doc();
    assert_eq!(
        group_paths(&mut doc, &["0", "2/0"]),
        None,
        "cross container"
    );
    assert_eq!(
        group_paths(&mut doc, &["0", "3"]),
        None,
        "Background refuses"
    );
    assert_eq!(group_paths(&mut doc, &["1", "3"]), None, "locked refuses");

    let mut doc = doc_with(vec![
        pixel_layer("a", 4, 4, 0),
        pixel_layer("b", 4, 4, 1),
        pixel_layer("c", 4, 4, 2),
    ]);
    let path = group_paths(&mut doc, &["0", "2"]).unwrap();
    assert_eq!(path, "1");
    assert_eq!(doc.layers.len(), 2);
    assert!(doc.layers[1].is_group);
    let children: Vec<&str> = doc.layers[1]
        .children
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    assert_eq!(children, ["a", "c"]);
    assert_eq!(doc.layers[0].name, "b");
}

#[test]
fn apply_visibility_is_solo() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("c", 4, 4, 1));
    let _ = group.children.push(pixel_layer("d", 4, 4, 2));
    let mut doc = doc_with(vec![
        pixel_layer("a", 4, 4, 0),
        pixel_layer("b", 4, 4, 3),
        group,
    ]);

    assert_eq!(apply_visibility(&mut doc, &["1"]), 4);
    assert_eq!(visible_names(&doc), vec!["b"]);
    assert!(!doc.layers[2].visible);
    assert!(doc.layers[2].children.iter().all(|child| !child.visible));

    assert_eq!(apply_visibility(&mut doc, &["2", "2/0"]), 3);
    assert_eq!(visible_names(&doc), vec!["Group 1", "c"]);
}

#[test]
fn delete_paths_nested_pair_deletes_once() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![pixel_layer("below", 4, 4, 1), group]);

    assert_eq!(delete_paths(&mut doc, &["1", "1/0"]), 1);
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn delete_paths_keeps_the_last_layer() {
    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 1), pixel_layer("b", 4, 4, 2)]);
    assert_eq!(
        delete_paths(&mut doc, &["0", "1"]),
        1,
        "deleting every layer keeps one"
    );
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(delete_paths(&mut doc, &["0"]), 0, "the only layer stays");
    assert_eq!(doc.layers.len(), 1);

    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![group]);
    assert_eq!(delete_paths(&mut doc, &["0"]), 0, "the only group stays");
    assert_eq!(
        delete_paths(&mut doc, &["0/0"]),
        1,
        "a nested layer may go: the group remains"
    );
}

#[test]
fn can_delete_paths_agrees_with_a_real_delete() {
    let mut background = pixel_layer("bg", 4, 4, 0);
    background.background = true;
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let docs = [
        doc_with(vec![pixel_layer("only", 4, 4, 1)]),
        doc_with(vec![background.clone(), pixel_layer("top", 4, 4, 1)]),
        doc_with(vec![background, locked_layer("locked")]),
        doc_with(vec![pixel_layer("a", 4, 4, 1), pixel_layer("b", 4, 4, 2)]),
        doc_with(vec![group]),
    ];
    let selections: [&[&str]; 6] = [&[], &["0"], &["1"], &["0", "1"], &["0/0"], &["7"]];
    for (d, doc) in docs.iter().enumerate() {
        for paths in selections {
            let mut copy = doc.clone();
            assert_eq!(
                can_delete_paths(doc, paths),
                delete_paths(&mut copy, paths) > 0,
                "document {d}, paths {paths:?}"
            );
        }
    }
}

#[test]
fn add_layer_in_inside_group_and_on_top() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![group]);

    let path = add_layer_in(&mut doc, "0", "Inside");
    assert_eq!(path, "0/1");
    assert_eq!(doc.layers[0].children.len(), 2);
    assert_eq!(doc.layers[0].children[1].name, "Inside");

    let path = add_group_in(&mut doc, "", "New Group");
    assert_eq!(path, "1", "empty selection lands on top");
    assert!(doc.layers[1].is_group);

    let path = add_layer_in(&mut doc, "99", "Top");
    assert_eq!(path, "2", "missing selection lands on top");
    assert_eq!(doc.layers[2].name, "Top");
}

#[test]
fn renaming_a_background_converts_it() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        pixel_layer("top", 4, 4, 1),
    ]);
    doc.layers[0].lock = LockFlags::all();
    doc.layers[0].channels.retain(|channel| channel.id != -1);
    assert!(rename_path(&mut doc, "0", "Sky"));
    assert_eq!(doc.layers[0].name, "Sky");
    assert!(!doc.layers[0].background);
    assert_eq!(doc.layers[0].lock, LockFlags::default());
    assert!(doc.layers[0]
        .channels
        .iter()
        .any(|channel| channel.id == -1));
}

#[test]
fn move_path_clamps_and_rename_resolves() {
    let mut doc = doc_with(vec![pixel_layer("a", 4, 4, 0), pixel_layer("b", 4, 4, 1)]);
    assert!(!move_path(&mut doc, "0", -1), "boundary");
    assert!(move_path(&mut doc, "0", 1));
    assert_eq!(doc.layers[0].name, "b");
    assert_eq!(doc.layers[1].name, "a");
    assert!(!move_path(&mut doc, "1", 5), "clamped to itself");
    assert!(!move_path(&mut doc, "9", 1), "missing path");

    assert!(rename_path(&mut doc, "1", "renamed"));
    assert_eq!(doc.layers[1].name, "renamed");
    assert!(!rename_path(&mut doc, "9", "x"));

    assert!(rename_path(&mut doc, "0", "Background"));
    assert_eq!(doc.layers[0].name, "Background");
    assert!(
        !doc.layers[0].background,
        "renaming does not create a Background"
    );
}

#[test]
fn move_path_refuses_background_and_locked() {
    let mut doc = doc_with(vec![
        pixel_layer("Background", 4, 4, 0),
        locked_layer("locked"),
        pixel_layer("normal", 4, 4, 3),
    ]);

    assert!(!move_path(&mut doc, "0", 1), "Background refuses");
    assert_eq!(doc.layers[0].name, "Background", "Background unchanged");
    assert_eq!(doc.layers[1].name, "locked");
    assert!(!move_path(&mut doc, "1", -1), "fully locked refuses");
    assert_eq!(doc.layers[0].name, "Background", "locked state unchanged");
    assert_eq!(doc.layers[1].name, "locked");

    assert!(move_path(&mut doc, "2", -1), "ordinary layer moves");
    assert_eq!(doc.layers[1].name, "normal");
    assert_eq!(doc.layers[2].name, "locked");
}

#[test]
fn nesting_lock_refuses_grouping_but_allows_reorder() {
    let mut nested = pixel_layer("nested", 4, 4, 1);
    nested.lock = LockFlags::default().with(LockFlags::NESTING, true);

    let mut doc = doc_with(vec![
        pixel_layer("a", 4, 4, 0),
        nested,
        pixel_layer("c", 4, 4, 2),
    ]);
    let before = doc.clone();
    assert_eq!(
        group_paths(&mut doc, &["0", "1"]),
        None,
        "nesting-locked node refuses grouping"
    );
    assert_eq!(doc, before, "document unchanged");

    assert!(
        move_path(&mut doc, "1", 1),
        "nesting lock does not block a within-container reorder"
    );
    assert_eq!(doc.layers[2].name, "nested");

    let mut group = empty_group("Group 1");
    group.lock = LockFlags::default().with(LockFlags::NESTING, true);
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![group]);
    let before = doc.clone();
    assert_eq!(
        ungroup_paths(&mut doc, &["0"]),
        0,
        "nesting-locked group is skipped"
    );
    assert_eq!(doc, before, "document unchanged");
}

#[test]
fn nesting_lock_refuses_reparent_in_and_out() {
    let mut group = empty_group("Locked");
    group.lock = LockFlags::default().with(LockFlags::NESTING, true);
    group.children = pictura_core::LayerStack::from_layers(vec![
        pixel_layer("A", 4, 4, 1),
        pixel_layer("B", 4, 4, 2),
    ]);
    let mut doc = doc_with(vec![pixel_layer("top", 4, 4, 3), group]);

    // A drop into the nesting-locked group is refused in both forms.
    let before = doc.clone();
    assert!(!can_move_path_to(&doc, "0", "1", 2));
    assert!(!move_path_to(&mut doc, "0", "1", 2));
    assert_eq!(doc, before, "drop into a nesting-locked group is refused");

    // A drop out of the group is refused, to the root and next to a peer.
    let before = doc.clone();
    assert!(!can_move_path_to(&doc, "1/0", "", 0));
    assert!(!move_path_to(&mut doc, "1/0", "", 0));
    assert_eq!(doc, before, "drop out of a nesting-locked group is refused");
    let before = doc.clone();
    assert!(!can_move_path_to(&doc, "1/0", "0", 0));
    assert!(!move_path_to(&mut doc, "1/0", "0", 0));
    assert_eq!(doc, before, "reparent to a top-level peer is refused");

    // A within-container reorder keeps the same parent and stays allowed.
    assert!(can_move_path_to(&doc, "1/0", "1/1", 0));
    assert!(move_path_to(&mut doc, "1/0", "1/1", 0));
    assert_eq!(doc.layers[1].children[0].name, "B");
    assert_eq!(doc.layers[1].children[1].name, "A");
}

#[test]
fn move_path_to_reparents_refuses_and_dry_runs() {
    fn sample() -> Document {
        let mut group = empty_group("Group");
        group.children = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("A", 4, 4, 1),
            pixel_layer("B", 4, 4, 2),
        ]);
        doc_with(vec![
            pixel_layer("Background", 4, 4, 0),
            group,
            pixel_layer("top", 4, 4, 3),
        ])
    }
    fn child_names(group: &Layer) -> Vec<&str> {
        group.children.iter().map(|c| c.name.as_str()).collect()
    }

    // Valid moves agree with the dry run and land where expected.
    let mut doc = sample();
    assert!(can_move_path_to(&doc, "2", "1", 2));
    assert!(move_path_to(&mut doc, "2", "1", 2));
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.layers[1].children.len(), 3);
    assert_eq!(doc.layers[1].children[2].name, "top");

    // Below then above a sibling both put A on top of B (bottom-first stack).
    let mut doc = sample();
    assert!(can_move_path_to(&doc, "1/1", "1/0", 1));
    assert!(move_path_to(&mut doc, "1/1", "1/0", 1));
    assert_eq!(child_names(&doc.layers[1]), vec!["B", "A"]);
    let mut doc = sample();
    assert!(can_move_path_to(&doc, "1/0", "1/1", 0));
    assert!(move_path_to(&mut doc, "1/0", "1/1", 0));
    assert_eq!(child_names(&doc.layers[1]), vec!["B", "A"]);

    // Every refusal is identical in the dry run, which never mutates.
    let cases: &[(&str, &str, i32)] = &[
        ("1/0", "1/0", 0), // self
        ("1", "1/0", 0),   // descendant
        ("0", "2", 2),     // Background
        ("2", "1/0", 2),   // Into a non-group
        ("9", "", 0),      // missing source
        ("2", "9/0", 0),   // unknown target container
        ("1/0", "bad", 0), // malformed target
    ];
    for &(path, target, mode) in cases {
        let mut doc = sample();
        let before = doc.clone();
        assert!(
            !can_move_path_to(&doc, path, target, mode),
            "{path}->{target}"
        );
        assert_eq!(doc, before, "dry run mutated {path}->{target} m{mode}");
        assert!(!move_path_to(&mut doc, path, target, mode));
        assert_eq!(doc, before, "refusal mutated {path}->{target} m{mode}");
    }

    // Locked and nesting-locked sources refuse in both forms.
    for lock in [
        LockFlags::all(),
        LockFlags::default().with(LockFlags::NESTING, true),
    ] {
        let mut doc = sample();
        doc.layers[2].lock = lock;
        let before = doc.clone();
        assert!(!can_move_path_to(&doc, "2", "1", 2));
        assert_eq!(doc, before);
        assert!(!move_path_to(&mut doc, "2", "1", 2));
    }
}

#[test]
fn move_path_to_into_group_from_above_resolves_post_removal() {
    // [A, G]: A sits above the group it is dropped into.
    let group = empty_group("G");
    let mut doc = doc_with(vec![pixel_layer("A", 4, 4, 1), group]);
    assert!(can_move_path_to(&doc, "0", "1", 2));
    assert!(move_path_to(&mut doc, "0", "1", 2));
    assert_eq!(doc.layers.len(), 1, "A left the root");
    assert_eq!(doc.layers[0].name, "G");
    let children: Vec<&str> = doc.layers[0]
        .children
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    assert_eq!(children, ["A"], "A is G's last child");

    // [A, G, B]: the sibling below the group stays put.
    let group = empty_group("G");
    let mut doc = doc_with(vec![
        pixel_layer("A", 4, 4, 1),
        group,
        pixel_layer("B", 4, 4, 2),
    ]);
    assert!(can_move_path_to(&doc, "0", "1", 2));
    assert!(move_path_to(&mut doc, "0", "1", 2));
    let root: Vec<&str> = doc.layers.iter().map(|layer| layer.name.as_str()).collect();
    assert_eq!(root, ["G", "B"], "A left the root, B did not move");
    let children: Vec<&str> = doc.layers[0]
        .children
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    assert_eq!(children, ["A"], "A is G's last child");
}

#[test]
fn move_path_to_into_a_sibling_container_shifts_each_depth() {
    // Root G has children [A, H]; H shifts from 0/1 to 0/0 once A is removed.
    let mut outer = empty_group("G");
    outer.children =
        pictura_core::LayerStack::from_layers(vec![pixel_layer("A", 4, 4, 1), empty_group("H")]);
    let mut doc = doc_with(vec![outer]);

    assert!(can_move_path_to(&doc, "0/0", "0/1", 2));
    assert!(move_path_to(&mut doc, "0/0", "0/1", 2));
    assert_eq!(doc.layers[0].children.len(), 1);
    assert_eq!(doc.layers[0].children[0].name, "H");
    assert_eq!(doc.layers[0].children[0].children[0].name, "A");
}

#[test]
fn move_path_to_out_of_group_to_root_and_root_siblings() {
    fn sample() -> Document {
        let mut group = empty_group("G");
        group.children = pictura_core::LayerStack::from_layers(vec![pixel_layer("c", 4, 4, 1)]);
        doc_with(vec![
            pixel_layer("bottom", 4, 4, 0),
            group,
            pixel_layer("top", 4, 4, 3),
        ])
    }

    // Out to the top of the document (empty target appends at the root top).
    let mut doc = sample();
    assert!(can_move_path_to(&doc, "1/0", "", 0));
    assert!(move_path_to(&mut doc, "1/0", "", 0));
    let root: Vec<&str> = doc.layers.iter().map(|layer| layer.name.as_str()).collect();
    assert_eq!(root, ["bottom", "G", "top", "c"]);
    assert!(doc.layers[1].children.is_empty(), "the group is now empty");

    // Above a root sibling.
    let mut doc = sample();
    assert!(move_path_to(&mut doc, "1/0", "0", 0));
    let root: Vec<&str> = doc.layers.iter().map(|layer| layer.name.as_str()).collect();
    assert_eq!(root, ["bottom", "c", "G", "top"]);

    // Below a root sibling.
    let mut doc = sample();
    assert!(move_path_to(&mut doc, "1/0", "2", 1));
    let root: Vec<&str> = doc.layers.iter().map(|layer| layer.name.as_str()).collect();
    assert_eq!(root, ["bottom", "G", "c", "top"]);
}

#[test]
fn move_path_to_keeps_reorder_and_refusals() {
    fn sample() -> Document {
        let mut group = empty_group("G");
        group.children = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("A", 4, 4, 1),
            pixel_layer("B", 4, 4, 2),
        ]);
        doc_with(vec![
            pixel_layer("Background", 4, 4, 0),
            group,
            pixel_layer("top", 4, 4, 3),
        ])
    }

    // Same-parent reorder still resolves.
    let mut doc = sample();
    assert!(move_path_to(&mut doc, "1/1", "1/0", 1));
    let children: Vec<&str> = doc.layers[1]
        .children
        .iter()
        .map(|layer| layer.name.as_str())
        .collect();
    assert_eq!(children, ["B", "A"]);

    // Refusals are unchanged: Background, own descendant, non-group Into.
    let cases: &[(&str, &str, i32)] = &[
        ("0", "", 0),    // Background
        ("1", "1/0", 0), // own descendant
        ("2", "1/0", 2), // Into a non-group
    ];
    for &(path, target, mode) in cases {
        let mut doc = sample();
        let before = doc.clone();
        assert!(
            !move_path_to(&mut doc, path, target, mode),
            "{path}->{target}"
        );
        assert_eq!(doc, before, "refusal mutated {path}->{target} m{mode}");
    }

    // A fully-locked source refuses too.
    let mut locked = sample();
    locked.layers[2].lock = LockFlags::all();
    let before = locked.clone();
    assert!(!move_path_to(&mut locked, "2", "1", 2));
    assert_eq!(locked, before);
}

#[test]
fn neutral_color_lookup_table() {
    let white = Some([255, 255, 255, 255]);
    let black = Some([0, 0, 0, 255]);
    let gray = Some([128, 128, 128, 255]);
    for mode in [
        BlendMode::Darken,
        BlendMode::Multiply,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::DarkerColor,
        BlendMode::Divide,
    ] {
        assert_eq!(neutral_color(mode), white, "{mode:?}");
    }
    for mode in [
        BlendMode::Lighten,
        BlendMode::Screen,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::LighterColor,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
    ] {
        assert_eq!(neutral_color(mode), black, "{mode:?}");
    }
    for mode in [
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
    ] {
        assert_eq!(neutral_color(mode), gray, "{mode:?}");
    }
    for mode in [
        BlendMode::Normal,
        BlendMode::Dissolve,
        BlendMode::HardMix,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ] {
        assert_eq!(neutral_color(mode), None, "{mode:?}");
    }
    let colored = BlendMode::LAYER_MODES
        .iter()
        .filter(|mode| neutral_color(**mode).is_some())
        .count();
    assert_eq!(colored, 20, "27 layer modes minus the seven unlisted");
}

#[test]
fn add_layer_full_applies_attributes_and_neutral_fill() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 0)]);
    let spec = NewLayerSpec {
        name: "Neutral".into(),
        color: ColorLabel::Red,
        blend: BlendMode::Multiply,
        opacity: 128,
        fill: 200,
        clipping: true,
        neutral_fill: true,
    };
    let path = add_layer_full(&mut doc, "", &spec);
    assert_eq!(path, "1");
    let layer = &doc.layers[1];
    assert_eq!(layer.name, "Neutral");
    assert_eq!(layer.color, ColorLabel::Red);
    assert_eq!(layer.blend, BlendMode::Multiply);
    assert_eq!(layer.opacity, 128);
    assert_eq!(layer.fill, 200);
    assert!(layer.clipping);
    for channel in &layer.channels {
        let expected = match channel.id {
            -1..=2 => 255,
            _ => continue,
        };
        assert!(
            channel.data.iter().all(|&v| v == expected),
            "channel {} is the Multiply white",
            channel.id
        );
    }
}

#[test]
fn add_layer_full_without_a_neutral_color_is_transparent() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 0)]);
    let spec = NewLayerSpec {
        name: "Normal".into(),
        blend: BlendMode::Normal,
        neutral_fill: true,
        ..NewLayerSpec::default()
    };
    let path = add_layer_full(&mut doc, "", &spec);
    assert_eq!(path, "1");
    for channel in &doc.layers[1].channels {
        assert!(
            channel.data.iter().all(|&v| v == 0),
            "channel {} transparent",
            channel.id
        );
    }
}

#[test]
fn add_group_full_ignores_fill_clipping_and_neutral_fill() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 0)]);
    let spec = NewLayerSpec {
        name: "Grp".into(),
        color: ColorLabel::Blue,
        blend: BlendMode::Screen,
        opacity: 200,
        fill: 10,
        clipping: true,
        neutral_fill: true,
    };
    let path = add_group_full(&mut doc, "", &spec);
    assert_eq!(path, "1");
    let group = &doc.layers[1];
    assert!(group.is_group);
    assert_eq!(group.name, "Grp");
    assert_eq!(group.color, ColorLabel::Blue);
    assert_eq!(group.blend, BlendMode::Screen);
    assert_eq!(group.opacity, 200);
    assert_eq!(group.fill, 255, "groups ignore fill");
    assert!(!group.clipping, "groups ignore clipping");
    assert!(group.channels.is_empty());
}

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
    let _ = group.children.push(pixel_layer("c0", 4, 4, 0));
    let _ = group.children.push(pixel_layer("c1", 4, 4, 1));
    let mut doc = doc_with(vec![pixel_layer("outer", 4, 4, 2), group]);

    assert!(arrange_path(&mut doc, "1/0", Arrange::Front));
    assert_eq!(doc.layers[1].children[1].name, "c0");
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
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
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

#[test]
fn forced_locks_refuse_clearing_transparency_and_image() {
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    let mut doc = Document::new(20, 20, ColorMode::Rgb, BitDepth::Eight);
    let square = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (2.0, 2.0, 8.0, 8.0),
    )
    .unwrap();
    let shape = add_shape_layer(&mut doc, "", [0, 0, 255, 255], "Rectangle", &square, None);

    // A shape forces the two bits: turning them on works, clearing is refused.
    assert_eq!(
        set_lock_paths(&mut doc, &[&shape], LockFlags::TRANSPARENCY, true),
        1
    );
    assert_eq!(
        set_lock_paths(&mut doc, &[&shape], LockFlags::TRANSPARENCY, false),
        0
    );
    assert!(resolve_path(&doc, &shape)
        .unwrap()
        .lock
        .contains(LockFlags::TRANSPARENCY));

    // Position is not forced and clears normally.
    assert_eq!(
        set_lock_paths(&mut doc, &[&shape], LockFlags::POSITION, true),
        1
    );
    assert_eq!(
        set_lock_paths(&mut doc, &[&shape], LockFlags::POSITION, false),
        1
    );

    // A type layer starts with the two bits set and cannot clear them.
    let text = crate::add_type_layer(
        &mut doc,
        "",
        &pictura_core::TypeSpec::new("Aa", "Liberation Sans", 20.0),
    );
    assert_eq!(
        set_lock_paths(&mut doc, &[&text], LockFlags::TRANSPARENCY, false),
        0
    );
    let layer = resolve_path(&doc, &text).unwrap();
    assert!(layer.lock.contains(LockFlags::TRANSPARENCY));
    assert!(layer.lock.contains(LockFlags::PIXELS));
}
