//! `Layer via Copy` / `Layer via Cut` tests, split from `tests.rs` to keep each
//! translation unit under the size cap.

use super::create::empty_group;
use super::tests::{doc_with, pixel_layer, rect};
use super::via::{layer_via_copy, layer_via_cut};
use pictura_core::LayerMask;

fn selection_mask(data: Vec<u8>) -> LayerMask {
    LayerMask {
        rect: rect(4, 4),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some(data.into()),
        ..Default::default()
    }
}

#[test]
fn layer_via_copy_extracts_selection_and_keeps_source() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let mut data = vec![0u8; 16];
    data[..8].fill(255);
    let path = layer_via_copy(&mut doc, "0", &selection_mask(data));

    assert_eq!(path, "1");
    assert_eq!(doc.layers.len(), 2);
    let copy = &doc.layers[1];
    assert_eq!(copy.name, "base copy");
    assert!(!copy.is_group && copy.adjustment.is_none());
    assert_eq!(copy.rect, rect(4, 4));

    let alpha = copy.channels.iter().find(|c| c.id == -1).unwrap();
    assert_eq!(&alpha.data[..8], &[255u8; 8]);
    assert!(
        alpha.data[8..].iter().all(|&v| v == 0),
        "outside is transparent"
    );
    let color = copy.channels.iter().find(|c| c.id == 0).unwrap();
    assert!(color.data[..8].iter().all(|&v| v == 40));
    assert!(
        color.data[8..].iter().all(|&v| v == 0),
        "outside not copied"
    );

    let source_alpha = doc.layers[0].channels.iter().find(|c| c.id == -1).unwrap();
    assert!(source_alpha.data.iter().all(|&v| v == 255), "source intact");
}

#[test]
fn layer_via_copy_scales_feathered_alpha() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let path = layer_via_copy(&mut doc, "0", &selection_mask(vec![128; 16]));

    assert_eq!(path, "1");
    let alpha = doc.layers[1].channels.iter().find(|c| c.id == -1).unwrap();
    assert!(
        alpha.data.iter().all(|&v| v == 128),
        "255 * 128/255 rounds to 128"
    );
    let source_alpha = doc.layers[0].channels.iter().find(|c| c.id == -1).unwrap();
    assert!(
        source_alpha.data.iter().all(|&v| v == 255),
        "copy leaves source"
    );
}

#[test]
fn layer_via_cut_clears_full_and_scales_partial_coverage() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    assert_eq!(
        layer_via_cut(&mut doc, "0", &selection_mask(vec![255; 16])),
        "1"
    );
    let source_alpha = doc.layers[0].channels.iter().find(|c| c.id == -1).unwrap();
    assert!(
        source_alpha.data.iter().all(|&v| v == 0),
        "full coverage clears"
    );

    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    layer_via_cut(&mut doc, "0", &selection_mask(vec![128; 16]));
    let source_alpha = doc.layers[0].channels.iter().find(|c| c.id == -1).unwrap();
    assert!(
        source_alpha.data.iter().all(|&v| v == 127),
        "255 * 127/255 = 127"
    );
}

#[test]
fn layer_via_copy_lands_above_the_source_inside_its_group() {
    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 40));
    let mut doc = doc_with(vec![group]);

    let path = layer_via_copy(&mut doc, "0/0", &selection_mask(vec![255; 16]));
    assert_eq!(path, "0/1");
    assert_eq!(doc.layers[0].children.len(), 2);
    assert_eq!(doc.layers[0].children[1].name, "child copy");
}

#[test]
fn layer_via_refuses_missing_group_and_empty_mask() {
    let mut doc = doc_with(vec![pixel_layer("base", 4, 4, 40)]);
    let before = doc.clone();
    assert!(layer_via_copy(&mut doc, "9", &selection_mask(vec![255; 16])).is_empty());
    assert!(layer_via_copy(&mut doc, "bad", &selection_mask(vec![255; 16])).is_empty());
    let empty = LayerMask {
        rect: rect(4, 4),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: None,
        ..Default::default()
    };
    assert!(layer_via_copy(&mut doc, "0", &empty).is_empty());
    assert_eq!(doc, before);

    let mut group = empty_group("Group 1");
    let _ = group.children.push(pixel_layer("child", 4, 4, 1));
    let mut doc = doc_with(vec![group]);
    let before = doc.clone();
    assert!(layer_via_copy(&mut doc, "0", &selection_mask(vec![255; 16])).is_empty());
    assert!(layer_via_cut(&mut doc, "0", &selection_mask(vec![255; 16])).is_empty());
    assert_eq!(doc, before);
}
