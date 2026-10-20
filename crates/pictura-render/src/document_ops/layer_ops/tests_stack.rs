//! Stack-invariant regression tests over the structural operations.

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LayerStack,
    LockFlags, PsdRect,
};

use super::*;

fn layer(name: &str, clipping: bool) -> Layer {
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: vec![40; 16].into(),
            },
            Channel {
                id: 1,
                data: vec![40; 16].into(),
            },
            Channel {
                id: 2,
                data: vec![40; 16].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 16].into(),
            },
        ],
        children: LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn group(name: &str) -> Layer {
    Layer {
        name: name.into(),
        is_group: true,
        ..layer(name, false)
    }
}

fn doc(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(layers);
    doc
}

fn assert_valid(doc: &Document, op: &str) {
    assert!(
        doc.layers.well_formed().is_ok(),
        "{op}: {:?}",
        doc.layers.well_formed()
    );
}

#[test]
fn every_structural_op_leaves_a_well_formed_stack() {
    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", false)]);
    add_layer(&mut d, 0, "new");
    assert_valid(&d, "add_layer");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    add_group_in(&mut d, "", "G");
    assert_valid(&d, "add_group_in");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    duplicate_paths(&mut d, &["0"]);
    assert_valid(&d, "duplicate_paths");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    group_paths(&mut d, &["0"]);
    assert_valid(&d, "group_paths");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    ungroup_paths(&mut d, &["0"]);
    assert_valid(&d, "ungroup_paths");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    delete_paths(&mut d, &["0"]);
    assert_valid(&d, "delete_paths");

    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", false)]);
    move_path(&mut d, "2", -1);
    assert_valid(&d, "move_path");

    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", false)]);
    move_path_to(&mut d, "2", "0", 0);
    assert_valid(&d, "move_path_to");

    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", false)]);
    arrange_path(&mut d, "0", Arrange::Front);
    assert_valid(&d, "arrange_path");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    reverse_paths(&mut d, &["0", "1"]);
    assert_valid(&d, "reverse_paths");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    create_clipping_mask(&mut d, &["1"]);
    assert_valid(&d, "create_clipping_mask");
    release_clipping_mask(&mut d, &["1"]);
    assert_valid(&d, "release_clipping_mask");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    background_from_layer(&mut d, "0");
    assert_valid(&d, "background_from_layer");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    merge_scope(&mut d, MergeScope::Down("1")).unwrap();
    assert_valid(&d, "merge_down");

    let mut d = doc(vec![
        layer("A", false),
        layer("B", false),
        layer("C", false),
    ]);
    let selected = vec!["0".to_string(), "2".to_string()];
    merge_scope(&mut d, MergeScope::Selected(&selected)).unwrap();
    assert_valid(&d, "merge_selected");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    flatten(&mut d).unwrap();
    assert_valid(&d, "flatten");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        default_color: 255,
        data: Some(vec![255; 16].into()),
        ..Default::default()
    };
    layer_via_copy(&mut d, "0", &mask);
    assert_valid(&d, "via_copy");

    let mut d = doc(vec![layer("A", false)]);
    let floating = lift_selection(&mut d, "0", &[255; 16]);
    assert_valid(&d, "lift_selection");
    if !floating.is_empty() {
        merge_lifted(&mut d, &floating);
        assert_valid(&d, "merge_lifted");
    }

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    d.layers.retain_tree(&[], &mut |layer| layer.name != "B");
    assert_valid(&d, "retain_tree");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    d.layers.truncate(&[], 1);
    assert_valid(&d, "truncate");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    let _ = d.layers.replace(&[0], vec![layer("replacement", false)]);
    assert_valid(&d, "replace");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    let _ = d
        .layers
        .splice(&[vec![0]], &[], 0, vec![layer("spliced", false)]);
    assert_valid(&d, "splice");

    let mut d = doc(vec![layer("A", false)]);
    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        default_color: 255,
        data: Some(vec![255; 16].into()),
        ..Default::default()
    };
    layer_via_cut(&mut d, "0", &mask);
    assert_valid(&d, "via_cut");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    merge_scope(&mut d, MergeScope::ClippingMask("1")).unwrap();
    assert_valid(&d, "merge_clipping_mask");

    let d = doc(vec![layer("A", false), layer("B", false)]);
    let before = d.clone();
    assert!(can_move_path_to(&d, "1", "0", 0));
    assert_eq!(d, before);

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    duplicate_layer(&mut d, 0);
    assert_valid(&d, "duplicate_layer");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    add_group_full(&mut d, "", &NewLayerSpec::default());
    assert_valid(&d, "add_group_full");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    merge_scope(&mut d, MergeScope::Visible("1")).unwrap();
    assert_valid(&d, "merge_visible");

    let mut d = doc(vec![layer("A", false), layer("B", false)]);
    assert!(stamp_scope(&mut d, StampScope::Visible("1"), "1").is_some());
    assert_valid(&d, "stamp_scope");

    let d = doc(vec![layer("A", false), layer("B", false)]);
    let before = d.clone();
    assert!(can_delete_paths(&d, &["1"]));
    assert_eq!(d, before);
}

#[test]
fn merge_removing_a_base_releases_its_run() {
    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", false)]);
    let selected = vec!["0".to_string(), "2".to_string()];
    merge_scope(&mut d, MergeScope::Selected(&selected)).unwrap();
    assert_eq!(d.layers.len(), 2);
    assert_eq!(d.layers[0].name, "B");
    assert_eq!(
        d.layers[1].name, "C",
        "the merged node takes the anchor name"
    );
    assert!(
        !d.layers[0].clipping,
        "B must not re-base to the merged node"
    );
    assert_valid(&d, "merge base removal");
}

#[test]
fn smart_object_conversion_splices_a_valid_run() {
    let mut source = Document::from_rgba("source", 2, 2, &[255; 16]);
    source.layers[0].clipping = true;
    let bytes = pictura_codec::write_psd(&source).expect("write source");
    let mut host = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let path = place_smart_object(&mut host, "source.psd", &bytes).expect("place");
    assert!(convert_smart_object_to_layers(&mut host, &path));
    assert_valid(&host, "smart object conversion");
}

#[test]
fn insertion_and_lift_never_leave_a_baseless_clip() {
    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    let _ = d.layers.push(layer("clipped", true));
    assert_valid(&d, "clipped push at the top");

    let mut d = doc(vec![layer("A", false)]);
    let _ = d.layers.push(layer("clipped", true));
    assert_valid(&d, "clipped push above a base");

    let mut d = doc(vec![layer("A", false)]);
    let spec = NewLayerSpec {
        name: "clipped".into(),
        clipping: true,
        ..NewLayerSpec::default()
    };
    add_layer_full(&mut d, "", &spec);
    assert_valid(&d, "new clipped layer at the top");

    let mut d = doc(vec![layer("A", false)]);
    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        default_color: 255,
        data: Some(vec![255; 16].into()),
        ..Default::default()
    };
    layer_via_copy(&mut d, "0", &mask);
    assert_valid(&d, "via copy");

    let mut d = doc(vec![layer("A", false)]);
    lift_selection(&mut d, "0", &[255; 16]);
    assert_valid(&d, "lift");
}

#[test]
fn grouping_a_base_keeps_its_run_attached() {
    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    group_paths(&mut d, &["0"]);
    assert_eq!(d.layers.len(), 2);
    assert!(d.layers[0].is_group);
    assert!(d.layers[1].clipping, "the group becomes B's base");
    assert_valid(&d, "group base alone");

    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    group_paths(&mut d, &["0", "1"]);
    assert_eq!(d.layers.len(), 1);
    assert!(d.layers[0].children[1].clipping);
    assert_valid(&d, "group base with its run");
}

#[test]
fn grouping_a_middle_member_leaves_a_valid_stack() {
    let mut d = doc(vec![layer("A", false), layer("B", true), layer("C", true)]);
    group_paths(&mut d, &["1"]);
    assert_valid(&d, "group middle");
    assert!(d.layers[2].clipping, "C clips to the new group");
}

#[test]
fn ungrouping_a_base_releases_its_run() {
    let mut inner = group("G");
    inner.children = LayerStack::from_layers(vec![layer("child", false)]);
    let mut d = doc(vec![layer("A", false), inner, layer("C", true)]);
    ungroup_paths(&mut d, &["1"]);
    assert_eq!(d.layers.len(), 3);
    assert!(
        !d.layers[2].clipping,
        "C must not re-base to the spliced child"
    );
    assert_valid(&d, "ungroup base");
}

#[test]
fn reverse_keeps_the_container_bottom_unclipped() {
    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    reverse_paths(&mut d, &["0", "1"]);
    assert!(!d.layers[0].clipping);
    assert_valid(&d, "reverse");
}

#[test]
fn scratch_filtering_releases_a_clipped_base() {
    let mut d = doc(vec![layer("A", false), layer("B", false), layer("C", true)]);
    d.layers.retain_tree(&[], &mut |layer| layer.name != "B");
    assert_eq!(d.layers.len(), 2);
    assert!(!d.layers[1].clipping);
    assert_valid(&d, "retain base removal");
}

#[test]
fn merge_selected_maps_a_shifting_ancestor() {
    let a = layer("A", false);
    let p = layer("P", false);
    let mut g = group("G");
    g.children = LayerStack::from_layers(vec![p]);
    let b = layer("B", false);
    let mut d = doc(vec![a, g, b]);
    let selected = vec!["0".to_string(), "1/0".to_string()];
    let outcome = merge_scope(&mut d, MergeScope::Selected(&selected)).expect("merge");
    assert_eq!(outcome.path, "0/0");
    assert_eq!(d.layers.len(), 2);
    assert!(d.layers[0].is_group);
    assert_eq!(d.layers[0].children[0].name, "P");
    assert_eq!(d.layers[1].name, "B");
    assert_valid(&d, "merge shifting ancestor");
}

#[test]
fn lift_refusal_keeps_source_pixels() {
    let src = layer("src", false);
    let mut background = layer("Background", false);
    background.background = true;
    let mut d = doc(vec![src, background]);
    let before = d.layers[0].channels[3].data.to_vec();
    assert!(lift_selection(&mut d, "0", &[255; 16]).is_empty());
    assert_eq!(&d.layers[0].channels[3].data[..], &before[..]);
}

#[test]
fn via_cut_refusal_keeps_source_pixels() {
    let src = layer("src", false);
    let mut background = layer("Background", false);
    background.background = true;
    let mut d = doc(vec![src, background]);
    let before = d.layers[0].channels[3].data.to_vec();
    let mask = LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 4,
            right: 4,
        },
        default_color: 255,
        data: Some(vec![255; 16].into()),
        ..Default::default()
    };
    assert!(layer_via_cut(&mut d, "0", &mask).is_empty());
    assert_eq!(&d.layers[0].channels[3].data[..], &before[..]);
}

#[test]
fn move_path_to_can_leave_a_source_below_the_background() {
    let mut background = layer("Background", false);
    background.background = true;
    let mut group = group("G");
    group.is_group = true;
    let mut d = doc(vec![layer("below", false), background, group]);
    assert!(can_move_path_to(&d, "0", "2", 2));
    assert!(move_path_to(&mut d, "0", "2", 2));
    assert!(d.layers[0].background);
    assert_eq!(d.layers[1].children[0].name, "below");
    assert_valid(&d, "move out of a below-background source");
}

#[test]
fn smart_object_conversion_with_a_background_host() {
    let mut source = Document::from_rgba("source", 2, 2, &[255; 16]);
    source.layers[0].clipping = true;
    let bytes = pictura_codec::write_psd(&source).expect("write source");
    let mut background = layer("Background", false);
    background.background = true;
    let mut host = doc(vec![background]);
    let path = place_smart_object(&mut host, "source.psd", &bytes).expect("place");
    assert!(convert_smart_object_to_layers(&mut host, &path));
    assert_eq!(host.layers.len(), 2);
    assert!(!host.layers[1].clipping);
    assert_valid(&host, "smart object over a Background");
}

#[test]
fn merge_lifted_keeps_a_run_clipped_to_the_source() {
    let mut d = doc(vec![layer("A", false), layer("B", true)]);
    let floating = lift_selection(&mut d, "0", &[255; 16]);
    assert!(!floating.is_empty());
    assert_eq!(d.layers.len(), 3);
    assert!(d.layers[2].clipping);
    assert!(merge_lifted(&mut d, &floating));
    assert_eq!(d.layers.len(), 2);
    assert!(d.layers[1].clipping, "the run stays clipped to the source");
    assert_valid(&d, "merge lifted");
}

#[test]
fn empty_container_queries_do_not_underflow() {
    let mut d = doc(vec![layer("A", false), group("G")]);
    assert!(!can_move_path(&d, "1/0", -1));
    assert!(!move_path(&mut d, "1/0", -1));
}

#[test]
fn unresolved_selection_path_falls_back_to_the_top() {
    let mut d = doc(vec![layer("A", false)]);
    let path = add_layer_full(&mut d, "9/0", &NewLayerSpec::default());
    assert_eq!(path, "1");
    assert_eq!(d.layers.len(), 2);
}

#[test]
fn ungrouping_a_bottom_group_with_a_nested_background_is_allowed() {
    let mut background = layer("bg", false);
    background.background = true;
    let mut inner = group("G");
    inner.children = LayerStack::from_layers(vec![background]);
    let mut d = doc(vec![inner, layer("A", false)]);
    assert_eq!(ungroup_paths(&mut d, &["0"]), 1);
    assert_eq!(d.layers.len(), 2);
    assert!(d.layers[0].background);
    assert_valid(&d, "ungroup bottom background group");
}

#[test]
fn ungrouping_a_non_bottom_group_with_a_nested_background_is_refused() {
    let mut background = layer("bg", false);
    background.background = true;
    let mut inner = group("G");
    inner.children = LayerStack::from_layers(vec![background]);
    let mut d = doc(vec![layer("A", false), inner]);
    let before = d.clone();
    assert_eq!(ungroup_paths(&mut d, &["1"]), 0);
    assert_eq!(d, before);
}
