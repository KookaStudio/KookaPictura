//! Cross-document layer copy tests.

use super::create::empty_group;
use super::tests::{doc_with, pixel_layer};
use super::transfer::copy_path_to_document;
use pictura_core::{BitDepth, BlendMode, ColorMode, Document, LockFlags};

#[test]
fn copy_keeps_attributes_and_lands_above_the_selection() {
    let mut layer = pixel_layer("Sky", 4, 4, 90);
    layer.blend = BlendMode::Multiply;
    layer.opacity = 128;
    let src = doc_with(vec![pixel_layer("base", 4, 4, 10), layer.clone()]);
    let mut dst = doc_with(vec![pixel_layer("a", 4, 4, 1), pixel_layer("b", 4, 4, 2)]);

    let created = copy_path_to_document(&src, "1", &mut dst, "0");

    assert_eq!(created.as_deref(), Some("1"));
    assert_eq!(dst.layers.len(), 3);
    assert_eq!(dst.layers[1], layer, "a deep copy with every attribute");
    assert_eq!(dst.layers[2].name, "b");
    assert_eq!(src.layers.len(), 2, "the source document is untouched");
}

#[test]
fn copy_without_a_selection_goes_on_top_and_groups_carry_children() {
    let mut group = empty_group("G");
    group.children = pictura_core::LayerStack::from_layers(vec![pixel_layer("child", 4, 4, 7)]);
    let src = doc_with(vec![group]);
    let mut dst = doc_with(vec![pixel_layer("a", 4, 4, 1)]);

    let created = copy_path_to_document(&src, "0", &mut dst, "");

    assert_eq!(created.as_deref(), Some("1"));
    assert!(dst.layers[1].is_group);
    assert_eq!(dst.layers[1].children[0].name, "child");
}

#[test]
fn copy_of_the_background_is_an_ordinary_layer() {
    let mut background = pixel_layer("Background", 4, 4, 255);
    background.background = true;
    background.channels.retain(|c| c.id != -1);
    let src = doc_with(vec![background]);
    let mut dst = doc_with(vec![pixel_layer("a", 4, 4, 1)]);

    let created = copy_path_to_document(&src, "0", &mut dst, "0").expect("copied");

    let copy = &dst.layers[created.parse::<usize>().unwrap()];
    assert!(!copy.background && copy.lock == LockFlags::default());
    assert!(copy.channels.iter().any(|c| c.id == -1), "gains an alpha");
}

#[test]
fn copy_refuses_another_mode_or_depth_and_unknown_paths() {
    let src = doc_with(vec![pixel_layer("a", 4, 4, 1)]);
    let mut gray = Document::new(4, 4, ColorMode::Grayscale, BitDepth::Eight);
    assert_eq!(copy_path_to_document(&src, "0", &mut gray, ""), None);
    assert!(gray.layers.is_empty());

    let mut dst = doc_with(Vec::new());
    assert_eq!(copy_path_to_document(&src, "7", &mut dst, ""), None);
    assert_eq!(copy_path_to_document(&src, "x", &mut dst, ""), None);
    assert!(dst.layers.is_empty());
}
