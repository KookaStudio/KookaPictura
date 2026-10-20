//! Layer > Create / Release Clipping Mask, and the Layers panel's Alt-click on
//! the line between two layers. A clipped layer shows only where the layer
//! under it (its base) has content; a run of clipped layers shares one base.
//! The compositor already honours [`Layer::clipping`]; these edit the flag.
//! Ported from photorust's `core/src/compositor.rs` clipping rules.

use pictura_core::{Document, Layer};

use super::paths::{container_of, parse_path};

/// Whether `path` can clip to the layer below it: the layer resolves, is not
/// the Background, is not already clipped, and has a non-Background base.
pub fn can_create_clipping_mask(doc: &Document, path: &str) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let Some((&index, parent)) = segments.split_last() else {
        return false;
    };
    let Some(container) = container_of(doc, parent) else {
        return false;
    };
    let Some(layer) = container.get(index) else {
        return false;
    };
    !layer.background
        && !layer.clipping
        && index > 0
        && base_below(container, index).is_some_and(|base| !container[base].background)
}

/// Clip each of `paths` to the layer below it. Returns how many changed.
pub fn create_clipping_mask(doc: &mut Document, paths: &[&str]) -> usize {
    let mut changed = 0;
    for path in paths {
        if !can_create_clipping_mask(doc, path) {
            continue;
        }
        let Some(segments) = parse_path(path) else {
            continue;
        };
        if doc.layers.set_clipping(&segments, true) {
            changed += 1;
        }
    }
    changed
}

/// The sibling indices Release Clipping Mask frees for `path`: a clipped
/// layer and every clipped layer above it in its run; a base, every layer
/// clipped to it. Empty when there is nothing to release.
fn release_set(doc: &Document, path: &str) -> Vec<String> {
    let Some(segments) = parse_path(path) else {
        return Vec::new();
    };
    let Some((&index, parent)) = segments.split_last() else {
        return Vec::new();
    };
    let Some(siblings) = container_of(doc, parent) else {
        return Vec::new();
    };
    let Some(layer) = siblings.get(index) else {
        return Vec::new();
    };
    let start = if layer.clipping { index } else { index + 1 };
    let prefix = parent
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("/");
    (start..siblings.len())
        .take_while(|&i| siblings[i].clipping)
        .map(|i| {
            if prefix.is_empty() {
                i.to_string()
            } else {
                format!("{prefix}/{i}")
            }
        })
        .collect()
}

/// Whether Release Clipping Mask would free anything for `path`.
pub fn can_release_clipping_mask(doc: &Document, path: &str) -> bool {
    !release_set(doc, path).is_empty()
}

/// Release Clipping Mask for each of `paths`. Returns how many layers changed.
pub fn release_clipping_mask(doc: &mut Document, paths: &[&str]) -> usize {
    let mut freed: Vec<String> = paths.iter().flat_map(|p| release_set(doc, p)).collect();
    freed.sort();
    freed.dedup();
    let mut changed = 0;
    for path in &freed {
        let Some(segments) = parse_path(path) else {
            continue;
        };
        if doc.layers.set_clipping(&segments, false) {
            changed += 1;
        }
    }
    changed
}

/// The nearest non-clipped sibling below `index`: the base of the run a clipped
/// layer at `index` belongs to.
fn base_below(container: &[Layer], index: usize) -> Option<usize> {
    (0..index).rev().find(|&i| !container[i].clipping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_ops::layer_ops::{
        add_group_in, add_layer, add_layer_full, add_layer_in, arrange_path, background_from_layer,
        can_background_from_layer, delete_paths, group_paths, move_path_to, resolve_path,
        reverse_paths, ungroup_layer, ungroup_paths, Arrange, NewLayerSpec,
    };
    use pictura_core::{BitDepth, ColorMode};

    /// Background plus four layers, bottom-first "0".."4".
    fn stack() -> Document {
        let mut doc = Document::from_rgba("Background", 4, 4, &[255; 64]);
        doc.layers[0].background = true;
        for i in 0..4 {
            add_layer(&mut doc, i, &format!("L{}", i + 1));
        }
        doc
    }

    fn doc_with(names: &[&str]) -> Document {
        let mut doc = Document::from_rgba(names[0], 4, 4, &[255; 64]);
        for name in &names[1..] {
            let above = doc.layers.len() as i32 - 1;
            add_layer(&mut doc, above, name);
        }
        doc
    }

    fn empty_doc() -> Document {
        Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight)
    }

    fn set_flags(doc: &mut Document, flags: &[bool]) {
        assert_eq!(doc.layers.len(), flags.len());
        for (layer, &flag) in doc.layers.iter_mut().zip(flags) {
            layer.clipping = flag;
        }
    }

    fn clipped(doc: &Document) -> Vec<bool> {
        doc.layers.iter().map(|l| l.clipping).collect()
    }

    fn names(doc: &Document) -> Vec<&str> {
        doc.layers.iter().map(|layer| layer.name.as_str()).collect()
    }

    #[test]
    fn create_clips_to_the_layer_below_but_never_the_bottom() {
        let mut doc = stack();
        assert!(!can_create_clipping_mask(&doc, "0"), "the Background");
        assert_eq!(create_clipping_mask(&mut doc, &["0", "2", "3"]), 2);
        assert_eq!(clipped(&doc), vec![false, false, true, true, false]);
        assert_eq!(create_clipping_mask(&mut doc, &["2"]), 0, "already clipped");

        let group = add_group_in(&mut doc, "4", "G");
        let child = format!("{group}/0");
        crate::add_layer_in(&mut doc, &group, "Inside");
        assert!(resolve_path(&doc, &child).is_some());
        assert!(
            !can_create_clipping_mask(&doc, &child),
            "bottom of its group"
        );
    }

    #[test]
    fn release_frees_the_run_above_a_clipped_layer_or_a_base() {
        let mut doc = stack();
        create_clipping_mask(&mut doc, &["2", "3", "4"]);
        assert_eq!(clipped(&doc), vec![false, false, true, true, true]);
        // From the middle of the run: it and the layers above.
        assert!(can_release_clipping_mask(&doc, "3"));
        assert_eq!(release_clipping_mask(&mut doc, &["3"]), 2);
        assert_eq!(clipped(&doc), vec![false, false, true, false, false]);
        // From the base: everything clipped to it.
        create_clipping_mask(&mut doc, &["3", "4"]);
        assert_eq!(release_clipping_mask(&mut doc, &["1"]), 3);
        assert_eq!(clipped(&doc), vec![false; 5]);
        assert!(!can_release_clipping_mask(&doc, "1"));
    }

    #[test]
    fn dragging_a_clipped_layer_below_its_base_releases_it() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert!(move_path_to(&mut doc, "1", "0", 1));
        assert_eq!(names(&doc), vec!["B", "A"]);
        assert_eq!(clipped(&doc), vec![false, false]);
    }

    #[test]
    fn dragging_a_clipped_layer_above_a_non_member_releases_it() {
        let mut doc = doc_with(&["A", "B", "B2", "C"]);
        set_flags(&mut doc, &[false, true, true, false]);
        assert!(move_path_to(&mut doc, "1", "3", 0));
        assert_eq!(names(&doc), vec!["A", "B2", "C", "B"]);
        assert_eq!(clipped(&doc), vec![false, true, false, false]);
    }

    #[test]
    fn moving_a_clipped_layer_into_another_run_joins_it() {
        let mut doc = doc_with(&["A", "B", "C", "D"]);
        set_flags(&mut doc, &[false, true, false, true]);
        assert!(move_path_to(&mut doc, "1", "3", 1));
        assert_eq!(names(&doc), vec!["A", "C", "B", "D"]);
        assert_eq!(clipped(&doc), vec![false, false, true, true]);
    }

    #[test]
    fn dropping_a_layer_inside_the_run_joins_it() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, false]);
        assert!(move_path_to(&mut doc, "2", "1", 1));
        assert_eq!(names(&doc), vec!["A", "C", "B"]);
        assert_eq!(clipped(&doc), vec![false, true, true]);
    }

    #[test]
    fn moving_a_clipped_layer_to_the_top_of_its_run_keeps_it_clipped() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, true]);
        assert!(move_path_to(&mut doc, "1", "2", 0));
        assert_eq!(names(&doc), vec!["A", "C", "B"]);
        assert_eq!(clipped(&doc), vec![false, true, true]);
    }

    #[test]
    fn moving_a_clipped_layer_to_the_bottom_of_its_run_keeps_it_clipped() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, true]);
        assert!(move_path_to(&mut doc, "2", "0", 0));
        assert_eq!(names(&doc), vec!["A", "C", "B"]);
        assert_eq!(clipped(&doc), vec![false, true, true]);
    }

    #[test]
    fn released_layers_stay_released_when_reordered_back() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert!(move_path_to(&mut doc, "0", "1", 0));
        assert_eq!(names(&doc), vec!["B", "A"]);
        assert_eq!(clipped(&doc), vec![false, false]);
        assert!(move_path_to(&mut doc, "0", "1", 0));
        assert_eq!(names(&doc), vec!["A", "B"]);
        assert_eq!(clipped(&doc), vec![false, false]);
    }

    #[test]
    fn moving_the_base_out_releases_its_run() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, true]);
        assert!(move_path_to(&mut doc, "0", "2", 0));
        assert_eq!(names(&doc), vec!["B", "C", "A"]);
        assert_eq!(clipped(&doc), vec![false, false, false]);
    }

    #[test]
    fn moving_the_only_child_out_of_a_group_is_safe() {
        let mut doc = empty_doc();
        let group = add_group_in(&mut doc, "", "G");
        assert_eq!(group, "0");
        assert_eq!(add_layer_in(&mut doc, &group, "A"), "0/0");
        assert!(move_path_to(&mut doc, "0/0", "0", 0));
        assert_eq!(names(&doc), vec!["G", "A"]);
    }

    #[test]
    fn moving_a_clipped_layer_to_the_top_of_the_document_is_safe() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert!(move_path_to(&mut doc, "1", "", 0));
        assert_eq!(names(&doc), vec!["A", "B"]);
        assert_eq!(clipped(&doc), vec![false, true]);
    }

    #[test]
    fn arranging_a_clipped_layer_to_the_back_releases_it() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert!(arrange_path(&mut doc, "1", Arrange::Back));
        assert_eq!(names(&doc), vec!["B", "A"]);
        assert_eq!(clipped(&doc), vec![false, false]);
    }

    #[test]
    fn deleting_the_base_releases_its_run() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, true]);
        assert_eq!(delete_paths(&mut doc, &["0"]), 1);
        assert_eq!(names(&doc), vec!["B", "C"]);
        assert_eq!(clipped(&doc), vec![false, false]);
    }

    #[test]
    fn deleting_a_clipped_layer_keeps_the_run() {
        let mut doc = doc_with(&["A", "B", "C"]);
        set_flags(&mut doc, &[false, true, true]);
        assert_eq!(delete_paths(&mut doc, &["1"]), 1);
        assert_eq!(names(&doc), vec!["A", "C"]);
        assert_eq!(clipped(&doc), vec![false, true]);
    }

    #[test]
    fn converting_the_base_to_background_releases_its_run() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert!(background_from_layer(&mut doc, "0"));
        assert_eq!(names(&doc), vec!["A", "B"]);
        assert!(doc.layers[0].background);
        assert_eq!(clipped(&doc), vec![false, false]);
    }

    #[test]
    fn converting_a_grouped_base_to_background_releases_its_run() {
        let mut doc = empty_doc();
        let group = add_group_in(&mut doc, "", "G");
        add_layer_in(&mut doc, &group, "A");
        add_layer_in(&mut doc, &group, "B");
        doc.layers[0].children[1].clipping = true;
        assert!(background_from_layer(&mut doc, "0/0"));
        assert_eq!(names(&doc), vec!["A", "G"]);
        assert!(doc.layers[0].background);
        assert!(!doc.layers[0].clipping);
        assert_eq!(doc.layers[1].children.len(), 1);
        assert!(!doc.layers[1].children[0].clipping);
    }

    #[test]
    fn reverse_clears_a_clipped_bottom() {
        let mut doc = empty_doc();
        let group = add_group_in(&mut doc, "", "G");
        add_layer_in(&mut doc, &group, "A");
        add_layer_in(&mut doc, &group, "B");
        doc.layers[0].children[1].clipping = true;
        assert!(reverse_paths(&mut doc, &["0/0", "0/1"]));
        assert_eq!(doc.layers[0].children[0].name, "B");
        assert!(!doc.layers[0].children[0].clipping);
    }

    #[test]
    fn ungroup_clears_a_clipped_bottom() {
        let mut doc = empty_doc();
        let group = add_group_in(&mut doc, "", "G");
        add_layer_in(&mut doc, &group, "A");
        add_layer_in(&mut doc, &group, "B");
        doc.layers[0].children[0].clipping = true;
        assert!(ungroup_layer(&mut doc, 0));
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[0].name, "A");
        assert!(!doc.layers[0].clipping);
    }

    #[test]
    fn ungrouping_a_base_group_releases_its_run() {
        let mut doc = doc_with(&["X"]);
        let group = add_group_in(&mut doc, "0", "G");
        add_layer_in(&mut doc, &group, "child");
        add_layer(&mut doc, 1, "C");
        doc.layers[2].clipping = true;
        assert_eq!(ungroup_paths(&mut doc, &[&group]), 1);
        assert_eq!(names(&doc), vec!["X", "child", "C"]);
        assert!(!doc.layers[2].clipping);
    }

    #[test]
    fn new_clipped_layer_at_container_bottom_is_released() {
        let mut doc = empty_doc();
        let group = add_group_in(&mut doc, "", "G");
        let spec = NewLayerSpec {
            name: "Clipped".into(),
            clipping: true,
            ..NewLayerSpec::default()
        };
        assert_eq!(add_layer_full(&mut doc, &group, &spec), "0/0");
        assert!(!doc.layers[0].children[0].clipping);
    }

    #[test]
    fn a_background_is_never_a_clipping_base() {
        let mut doc = stack();
        assert!(
            !can_create_clipping_mask(&doc, "1"),
            "the layer above the Background"
        );
        assert_eq!(create_clipping_mask(&mut doc, &["1"]), 0);
        assert!(!doc.layers[1].clipping);
        assert!(
            can_create_clipping_mask(&doc, "2"),
            "a normal base is still allowed"
        );

        let spec = NewLayerSpec {
            name: "Clipped".into(),
            clipping: true,
            ..NewLayerSpec::default()
        };
        let path = add_layer_full(&mut doc, "0", &spec);
        assert_eq!(path, "1");
        assert!(!resolve_path(&doc, &path).is_some_and(|layer| layer.clipping));

        let mut nested = empty_doc();
        let group = add_group_in(&mut nested, "", "G");
        add_layer_in(&mut nested, &group, "Bg");
        add_layer_in(&mut nested, &group, "Above");
        nested.layers[0].children[0].background = true;
        assert!(
            !can_create_clipping_mask(&nested, "0/1"),
            "the nested Background"
        );
    }

    #[test]
    fn reverse_releases_a_clip_on_the_background() {
        let mut doc = stack();
        create_clipping_mask(&mut doc, &["2"]);
        assert!(reverse_paths(&mut doc, &["1", "2"]));
        assert_eq!(names(&doc), vec!["Background", "L2", "L1", "L3", "L4"]);
        assert!(!doc.layers[1].clipping);
    }

    #[test]
    fn grouping_a_clipped_layer_clears_the_new_bottom() {
        let mut doc = doc_with(&["A", "B"]);
        set_flags(&mut doc, &[false, true]);
        assert_eq!(group_paths(&mut doc, &["1"]).as_deref(), Some("1"));
        assert!(doc.layers[1].is_group);
        assert!(!doc.layers[1].children[0].clipping);
    }

    #[test]
    fn background_from_layer_refuses_a_second_background() {
        let mut doc = stack();
        let before = doc.clone();
        assert!(!can_background_from_layer(&doc, "1"));
        assert!(!background_from_layer(&mut doc, "1"));
        assert_eq!(doc, before);
        let plain = doc_with(&["A", "B"]);
        assert!(can_background_from_layer(&plain, "1"));
    }
}
