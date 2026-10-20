//! Unit tests for the merge engine, split from `merge.rs` for the file-size budget.

use super::merge::{
    can_merge_scope, can_merge_target, flatten, is_visible_in_panel, merge_scope, stamp_scope,
    MergeError, MergeScope, StampScope,
};
use super::properties::{delete_hidden_layers, select_similar};
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
    PixelBuffer, PsdRect,
};

fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: h,
        right: w,
    }
}

fn at(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn solid(name: &str, bounds: PsdRect, r: u8, g: u8, b: u8, a: u8) -> Layer {
    let n = (bounds.width().max(0) * bounds.height().max(0)) as usize;
    Layer {
        name: name.into(),
        rect: bounds,
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
                data: vec![r; n].into(),
            },
            Channel {
                id: 1,
                data: vec![g; n].into(),
            },
            Channel {
                id: 2,
                data: vec![b; n].into(),
            },
            Channel {
                id: -1,
                data: vec![a; n].into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn adjustment(name: &str, bounds: PsdRect) -> Layer {
    let mut layer = solid(name, bounds, 0, 0, 0, 0);
    layer.channels.clear();
    layer.adjustment = Some(crate::encode_invert());
    layer
}

fn group(name: &str, children: Vec<Layer>) -> Layer {
    Layer {
        name: name.into(),
        rect: rect(0, 0),
        blend: BlendMode::PassThrough,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(children),
        is_group: true,
        background: false,
        ..Default::default()
    }
}

fn doc_with(w: u32, h: u32, layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

/// Independent crop of a full-document RGBA buffer to interleaved RGBA8.
fn crop(buffer: &PixelBuffer, bounds: PsdRect) -> Vec<u8> {
    let w = bounds.width().max(0) as usize;
    let h = bounds.height().max(0) as usize;
    let plane = buffer.width as usize * buffer.height as usize;
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let source =
                (bounds.top as usize + y) * buffer.width as usize + bounds.left as usize + x;
            for c in 0..4 {
                out.push(buffer.data[c * plane + source]);
            }
        }
    }
    out
}

/// Interleaved RGBA8 for a result node.
fn node_interleaved(layer: &Layer) -> Vec<u8> {
    let n = (layer.rect.width().max(0) * layer.rect.height().max(0)) as usize;
    let plane = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.clone())
            .unwrap_or_else(|| vec![255; n].into())
    };
    let (r, g, b, a) = (plane(0), plane(1), plane(2), plane(-1));
    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        out.extend_from_slice(&[r[i], g[i], b[i], a[i]]);
    }
    out
}

/// The oracle: `composite_rgba` of exactly the inputs, cropped to the node.
fn inputs_composite(inputs: Vec<Layer>, doc: &Document, bounds: PsdRect) -> Vec<u8> {
    let mut scratch = Document::new(doc.width, doc.height, doc.mode, doc.depth);
    scratch.layers = pictura_core::LayerStack::from_layers(inputs);
    crop(&crate::composite_rgba(&scratch), bounds)
}

#[test]
fn merge_down_replaces_pair_and_inherits_lower() {
    let mut lower = solid("lower", rect(4, 4), 40, 60, 80, 255);
    lower.blend = BlendMode::Screen;
    lower.opacity = 200;
    let upper = solid("upper", rect(4, 4), 120, 130, 140, 255);
    let mut doc = doc_with(4, 4, vec![lower.clone(), upper.clone()]);

    let outcome = merge_scope(&mut doc, MergeScope::Down("1")).unwrap();
    assert_eq!(outcome.replaced, 2);
    assert_eq!(doc.layers.len(), 1);
    let merged = &doc.layers[0];
    assert_eq!(merged.name, "lower");
    assert_eq!(merged.blend, BlendMode::Screen);
    assert_eq!(merged.opacity, 200);
    assert_eq!(merged.rect, rect(4, 4));
    assert_eq!(outcome.path, "0");

    let expected = inputs_composite(vec![lower, upper], &doc, rect(4, 4));
    assert_eq!(node_interleaved(merged), expected);
}

#[test]
fn merge_down_union_rect_covers_both_content_boxes() {
    let lower = solid("lower", at(0, 0, 2, 2), 10, 0, 0, 255);
    let upper = solid("upper", at(2, 2, 4, 4), 0, 20, 0, 255);
    let mut doc = doc_with(4, 4, vec![lower, upper]);

    let outcome = merge_scope(&mut doc, MergeScope::Down("1")).unwrap();
    let merged = &doc.layers[0];
    assert_eq!(merged.rect, at(0, 0, 4, 4));
    assert_eq!(outcome.path, "0");
}

#[test]
fn merge_down_applies_masks_into_alpha() {
    let mut lower = solid("lower", rect(4, 4), 10, 20, 30, 255);
    let lower_alpha = lower
        .channels
        .iter_mut()
        .find(|channel| channel.id == -1)
        .expect("lower alpha");
    lower_alpha.data = vec![
        255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0,
    ]
    .into();

    let mut upper = solid("upper", rect(4, 4), 200, 210, 220, 255);
    upper.mask = Some(LayerMask {
        rect: rect(4, 4),
        default_color: 255,
        data: Some(vec![0, 0, 255, 0, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255].into()),
        ..Default::default()
    });

    let mut doc = doc_with(4, 4, vec![lower.clone(), upper.clone()]);
    let outcome = merge_scope(&mut doc, MergeScope::Down("1")).unwrap();
    assert_eq!(outcome.replaced, 2);
    let merged = &doc.layers[0];
    let expected = inputs_composite(vec![lower, upper], &doc, rect(4, 4));
    assert_eq!(node_interleaved(merged), expected);
    let alpha = merged
        .channels
        .iter()
        .find(|channel| channel.id == -1)
        .expect("merged alpha");
    let expected_alpha = [
        255, 255, 255, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    ];
    assert_eq!(&alpha.data[..], &expected_alpha[..]);
    assert!(
        merged.mask.is_none(),
        "the result owns its pixels, not a mask"
    );
}

#[test]
fn merge_down_refuses_without_a_layer_below() {
    let mut doc = doc_with(4, 4, vec![solid("only", rect(4, 4), 1, 2, 3, 255)]);
    let before = doc.clone();
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Down("0")),
        Err(MergeError::NoLayerBelow)
    );
    assert_eq!(doc, before, "refusal leaves the document unchanged");
}

#[test]
fn merge_down_refuses_adjustment_targets() {
    let mut doc = doc_with(
        4,
        4,
        vec![
            adjustment("adj", rect(4, 4)),
            solid("upper", rect(4, 4), 1, 2, 3, 255),
        ],
    );
    let before = doc.clone();
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Down("1")),
        Err(MergeError::InvalidTarget)
    );
    assert_eq!(doc, before);
}

#[test]
fn merge_down_bakes_an_adjustment_source() {
    let lower = solid("lower", rect(4, 4), 40, 60, 80, 255);
    let adj = adjustment("adj", rect(4, 4));
    let mut doc = doc_with(4, 4, vec![lower.clone(), adj.clone()]);

    assert!(can_merge_scope(&doc, &MergeScope::Down("1")));
    merge_scope(&mut doc, MergeScope::Down("1")).unwrap();
    assert_eq!(doc.layers.len(), 1);
    let merged = &doc.layers[0];
    assert_eq!(merged.name, "lower");
    assert_eq!(merged.blend, lower.blend);
    assert_eq!(merged.opacity, lower.opacity);
    assert!(merged.adjustment.is_none());

    let expected = inputs_composite(vec![lower, adj], &doc, rect(4, 4));
    assert_eq!(node_interleaved(merged), expected);
}

#[test]
fn merge_layers_lands_topmost_and_resets_blend() {
    let mut a = solid("a", rect(4, 4), 10, 0, 0, 255);
    a.blend = BlendMode::Multiply;
    a.opacity = 100;
    let b = solid("b", rect(4, 4), 0, 20, 0, 255);
    let c = solid("c", rect(4, 4), 0, 0, 30, 255);
    let mut doc = doc_with(4, 4, vec![a.clone(), b.clone(), c.clone()]);

    let selected = vec!["0".to_string(), "2".to_string()];
    let outcome = merge_scope(&mut doc, MergeScope::Selected(&selected)).unwrap();
    assert_eq!(outcome.replaced, 2);
    assert_eq!(doc.layers.len(), 2, "unselected middle layer survives");
    assert_eq!(doc.layers[0].name, "b", "unselected kept its slot");
    let merged = &doc.layers[1];
    assert_eq!(merged.name, "c", "inherits the topmost selected name");
    assert_eq!(merged.blend, BlendMode::Normal);
    assert_eq!(merged.opacity, 255);
    assert_eq!(outcome.path, "1");

    let expected = inputs_composite(vec![a, c], &doc, rect(4, 4));
    assert_eq!(node_interleaved(merged), expected);
}

#[test]
fn merge_layers_requires_two_distinct_paths() {
    let mut doc = doc_with(4, 4, vec![solid("a", rect(4, 4), 1, 2, 3, 255)]);
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Selected(&["0".to_string()])),
        Err(MergeError::NoSelection)
    );
    assert_eq!(
        merge_scope(
            &mut doc,
            MergeScope::Selected(&["0".to_string(), "0".to_string()])
        ),
        Err(MergeError::NoSelection)
    );
}

#[test]
fn merge_layers_refuses_adjustment_bottom() {
    let mut doc = doc_with(
        4,
        4,
        vec![
            adjustment("adj", rect(4, 4)),
            solid("top", rect(4, 4), 1, 2, 3, 255),
        ],
    );
    let before = doc.clone();
    assert_eq!(
        merge_scope(
            &mut doc,
            MergeScope::Selected(&["0".to_string(), "1".to_string()])
        ),
        Err(MergeError::InvalidTarget)
    );
    assert_eq!(doc, before);
}

#[test]
fn merge_layers_bakes_an_adjustment_above_the_bottom() {
    let bottom = solid("bottom", rect(4, 4), 20, 40, 60, 255);
    let adj = adjustment("adj", rect(4, 4));
    let top = solid("top", rect(4, 4), 200, 10, 10, 255);
    let mut doc = doc_with(4, 4, vec![bottom.clone(), adj.clone(), top.clone()]);

    let selected = vec!["0".to_string(), "1".to_string(), "2".to_string()];
    merge_scope(&mut doc, MergeScope::Selected(&selected)).unwrap();
    assert_eq!(doc.layers.len(), 1);
    let merged = &doc.layers[0];
    assert_eq!(merged.name, "top");
    assert_eq!(merged.blend, BlendMode::Normal);
    assert_eq!(merged.opacity, 255);
    assert!(merged.adjustment.is_none());

    let expected = inputs_composite(vec![bottom, adj, top], &doc, rect(4, 4));
    assert_eq!(node_interleaved(merged), expected);
}

#[test]
fn merge_visible_skips_hidden_layers_in_place() {
    let mut hidden = solid("hidden", rect(4, 4), 9, 9, 9, 255);
    hidden.visible = false;
    let visible = solid("visible", rect(4, 4), 50, 60, 70, 255);
    let mut doc = doc_with(4, 4, vec![hidden, visible.clone()]);

    let outcome = merge_scope(&mut doc, MergeScope::Visible("1")).unwrap();
    assert_eq!(outcome.replaced, 1);
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.layers[0].name, "hidden");
    assert!(!doc.layers[0].visible, "hidden layer survives unchanged");
    let merged = &doc.layers[1];
    assert_eq!(merged.name, "visible");
    assert_eq!(
        node_interleaved(merged),
        inputs_composite(vec![visible], &doc, rect(4, 4))
    );
}

#[test]
fn merge_visible_refuses_hidden_active() {
    let mut hidden = solid("hidden", rect(4, 4), 9, 9, 9, 255);
    hidden.visible = false;
    let mut doc = doc_with(
        4,
        4,
        vec![hidden, solid("visible", rect(4, 4), 1, 2, 3, 255)],
    );
    let before = doc.clone();
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Visible("0")),
        Err(MergeError::NoSelection)
    );
    assert_eq!(doc, before);
}

#[test]
fn merge_visible_respects_ancestor_visibility() {
    let mut inner = solid("inner", rect(4, 4), 5, 5, 5, 255);
    inner.visible = false;
    let mut hidden_group = group("hidden group", vec![inner.clone()]);
    hidden_group.visible = false;
    let mut doc = doc_with(
        4,
        4,
        vec![hidden_group, solid("top", rect(4, 4), 1, 1, 1, 255)],
    );
    assert!(!is_visible_in_panel(&doc, "0/0"));
    let before = doc.clone();
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Visible("0/0")),
        Err(MergeError::NoSelection)
    );
    assert_eq!(doc, before);
}

#[test]
fn flatten_discards_hidden_and_fills_transparency_white() {
    let mut hidden = solid("hidden", rect(4, 4), 200, 0, 0, 255);
    hidden.visible = false;
    // A single transparent source pixel at (0,0) leaves the backdrop white.
    let mut partial = solid("partial", rect(4, 4), 10, 20, 30, 255);
    partial.channels[3].data[0] = 0;
    let mut doc = doc_with(4, 4, vec![hidden, partial]);

    let outcome = flatten(&mut doc).unwrap();
    assert_eq!(outcome.replaced, 2);
    assert_eq!(doc.layers.len(), 1);
    let background = &doc.layers[0];
    assert_eq!(background.name, "Background");
    assert!(background.background, "flatten sets the Background flag");
    assert_eq!(background.rect, rect(4, 4));
    assert_eq!(background.blend, BlendMode::Normal);
    assert_eq!(background.opacity, 255);
    assert_eq!(background.fill, 255);
    assert!(background.channels.iter().all(|c| c.data.len() == 16));
    assert!(
        background
            .channels
            .iter()
            .find(|c| c.id == -1)
            .unwrap()
            .data
            .iter()
            .all(|&a| a == 255),
        "flattened pixels are opaque"
    );
    // Transparent source becomes white (channel order 0,1,2,-1).
    assert_eq!(background.channels[0].data[0], 255);
    assert_eq!(background.channels[1].data[0], 255);
    assert_eq!(background.channels[2].data[0], 255);
}

#[test]
fn flatten_removes_groups_and_masks() {
    let mut masked = solid("masked", rect(4, 4), 1, 2, 3, 255);
    masked.mask = Some(pictura_core::LayerMask {
        rect: rect(4, 4),
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(vec![128; 16].into()),
        ..Default::default()
    });
    let doc_group = group("grp", vec![masked]);
    let mut doc = doc_with(4, 4, vec![doc_group]);

    flatten(&mut doc).unwrap();
    assert_eq!(doc.layers.len(), 1);
    assert!(!doc.layers[0].is_group);
    assert!(doc.layers[0].mask.is_none());
}

#[test]
fn clipping_mask_folds_base_alpha_into_siblings() {
    let mut base = solid("base", rect(4, 4), 100, 100, 100, 255);
    // Base alpha is a left/right split.
    for x in 0..4 {
        for y in 0..4 {
            base.channels[3].data[y * 4 + x] = if x < 2 { 255 } else { 0 };
        }
    }
    let mut clipped = solid("clipped", rect(4, 4), 200, 50, 0, 255);
    clipped.clipping = true;
    let mut doc = doc_with(4, 4, vec![base.clone(), clipped.clone()]);

    let outcome = merge_scope(&mut doc, MergeScope::ClippingMask("1")).unwrap();
    assert_eq!(outcome.replaced, 2);
    assert_eq!(doc.layers.len(), 1);
    let merged = &doc.layers[0];
    assert_eq!(merged.name, "base");
    assert_eq!(outcome.path, "0");

    // Right half was clipped away by the base's zero alpha.
    let alpha = merged.channels.iter().find(|c| c.id == -1).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            let expect = if x < 2 { 255 } else { 0 };
            assert_eq!(alpha.data[y * 4 + x], expect, "pixel ({x},{y})");
        }
    }
}

#[test]
fn clipping_mask_refuses_non_raster_base() {
    let mut doc = doc_with(
        4,
        4,
        vec![adjustment("adj base", rect(4, 4)), {
            let mut clipped = solid("clipped", rect(4, 4), 1, 2, 3, 255);
            clipped.clipping = true;
            clipped
        }],
    );
    let before = doc.clone();
    assert_eq!(
        merge_scope(&mut doc, MergeScope::ClippingMask("1")),
        Err(MergeError::NotClippable)
    );
    assert_eq!(doc, before);
}

#[test]
fn clipping_mask_refuses_without_clipped_siblings() {
    let mut doc = doc_with(4, 4, vec![solid("base", rect(4, 4), 1, 2, 3, 255)]);
    assert_eq!(
        merge_scope(&mut doc, MergeScope::ClippingMask("0")),
        Err(MergeError::NotClippable)
    );
}

#[test]
fn can_merge_target_rejects_adjustments_only() {
    assert!(can_merge_target(&solid("raster", rect(4, 4), 0, 0, 0, 255)));
    assert!(can_merge_target(&group("grp", vec![])));
    assert!(!can_merge_target(&adjustment("adj", rect(4, 4))));
}

#[test]
fn can_merge_scope_matches_the_merge_guards() {
    let pair = doc_with(
        4,
        4,
        vec![
            solid("lower", rect(4, 4), 1, 2, 3, 255),
            solid("upper", rect(4, 4), 4, 5, 6, 255),
        ],
    );
    assert!(can_merge_scope(&pair, &MergeScope::Down("1")));
    assert!(!can_merge_scope(&pair, &MergeScope::Down("0")));

    let adj = doc_with(
        4,
        4,
        vec![
            adjustment("adj", rect(4, 4)),
            solid("upper", rect(4, 4), 1, 2, 3, 255),
        ],
    );
    assert!(!can_merge_scope(&adj, &MergeScope::Down("1")));

    let adj_source = doc_with(
        4,
        4,
        vec![
            solid("lower", rect(4, 4), 1, 2, 3, 255),
            adjustment("adj", rect(4, 4)),
        ],
    );
    assert!(can_merge_scope(&adj_source, &MergeScope::Down("1")));

    let two = vec!["0".to_string(), "1".to_string()];
    assert!(can_merge_scope(&pair, &MergeScope::Selected(&two)));
    let one = vec!["0".to_string()];
    assert!(!can_merge_scope(&pair, &MergeScope::Selected(&one)));

    let mut clipped = solid("clipped", rect(4, 4), 9, 9, 9, 255);
    clipped.clipping = true;
    let base = doc_with(4, 4, vec![solid("base", rect(4, 4), 1, 2, 3, 255), clipped]);
    assert!(can_merge_scope(&base, &MergeScope::ClippingMask("1")));
    let lone = doc_with(4, 4, vec![solid("base", rect(4, 4), 1, 2, 3, 255)]);
    assert!(!can_merge_scope(&lone, &MergeScope::ClippingMask("0")));
}

#[test]
fn nested_merge_keeps_the_group_for_itself() {
    let inner = solid("inner", rect(4, 4), 30, 0, 0, 255);
    let inner2 = solid("inner2", rect(4, 4), 0, 40, 0, 255);
    let outer = solid("outer", rect(4, 4), 0, 0, 50, 255);
    let doc_group = group("grp", vec![inner, inner2]);
    let mut doc = doc_with(4, 4, vec![doc_group, outer]);

    let outcome = merge_scope(&mut doc, MergeScope::Down("0/1")).unwrap();
    assert_eq!(outcome.path, "0/0");
    assert_eq!(doc.layers.len(), 2, "top-level stack unchanged");
    assert_eq!(doc.layers[0].children.len(), 1);
    assert_eq!(doc.layers[0].children[0].name, "inner");
    assert!(doc.layers[0].is_group, "merge stays inside its group");
}

#[test]
fn flatten_empty_tree_yields_white_background() {
    let mut doc = doc_with(4, 4, Vec::new());
    let outcome = flatten(&mut doc).unwrap();
    assert_eq!(outcome.replaced, 0);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.layers[0].name, "Background");
    assert!(doc.layers[0].channels[0].data.iter().all(|&v| v == 255));
}

#[test]
fn merge_scope_on_empty_document_is_refused() {
    let mut doc = doc_with(4, 4, Vec::new());
    assert_eq!(
        merge_scope(&mut doc, MergeScope::Down("0")),
        Err(MergeError::NoDocument)
    );
}

#[test]
fn select_similar_matches_class_adjustment_kind_and_blend() {
    let mut background = solid("Background", rect(4, 4), 1, 1, 1, 255);
    background.background = true;
    let mut multiply = solid("mul", rect(4, 4), 1, 1, 1, 255);
    multiply.blend = BlendMode::Multiply;
    let mut posterize = solid("poster", rect(4, 4), 0, 0, 0, 0);
    posterize.channels.clear();
    posterize.adjustment = Some(crate::encode_posterize(4));

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        background,                            // 0
        solid("p1", rect(4, 4), 1, 1, 1, 255), // 1
        multiply,                              // 2
        solid("p2", rect(4, 4), 1, 1, 1, 255), // 3
        adjustment("adj", rect(4, 4)),         // 4
        adjustment("adj", rect(4, 4)),         // 5
        posterize,                             // 6
        group("g1", Vec::new()),               // 7
        group("g2", Vec::new()),               // 8
    ]);

    // Same pixel class and Normal blend, topmost-first, reference excluded.
    assert_eq!(select_similar(&doc, "3"), vec!["1".to_string()]);
    // Same adjustment kind (invert), not the posterize.
    assert_eq!(select_similar(&doc, "5"), vec!["4".to_string()]);
    // Same group class.
    assert_eq!(select_similar(&doc, "8"), vec!["7".to_string()]);
    // The only Background matches nothing.
    assert!(select_similar(&doc, "0").is_empty());
    // Unknown path matches nothing.
    assert!(select_similar(&doc, "bad").is_empty());
}

#[test]
fn delete_hidden_layers_removes_hidden_and_counts_nested_group_once() {
    let mut hidden = solid("h", rect(4, 4), 1, 1, 1, 255);
    hidden.visible = false;
    let mut nested = group("g", vec![solid("child", rect(4, 4), 1, 1, 1, 255)]);
    nested.visible = false;

    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        solid("v", rect(4, 4), 1, 1, 1, 255),
        hidden,
        nested,
    ]);

    assert_eq!(delete_hidden_layers(&mut doc), 2);
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.layers[0].name, "v");
}

#[test]
fn delete_hidden_layers_is_noop_without_hidden_layers() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        solid("a", rect(4, 4), 1, 1, 1, 255),
        solid("b", rect(4, 4), 1, 1, 1, 255),
    ]);
    let before = doc.clone();
    assert_eq!(delete_hidden_layers(&mut doc), 0);
    assert_eq!(doc, before);
}

fn stamp_index(doc: &Document) -> usize {
    doc.layers
        .iter()
        .position(|layer| layer.name.starts_with("Stamp"))
        .expect("a stamp layer")
}

#[test]
fn stamp_visible_adds_one_layer_and_keeps_originals() {
    let lower = solid("lower", rect(4, 4), 10, 20, 30, 255);
    let upper = solid("upper", rect(4, 4), 40, 50, 60, 255);
    let mut doc = doc_with(4, 4, vec![lower.clone(), upper.clone()]);
    let before = doc.layers.len();

    let added = stamp_scope(&mut doc, StampScope::Visible("1"), "1").expect("stamp");
    assert!(!added.is_empty());
    assert_eq!(
        doc.layers.len(),
        before + 1,
        "one new layer, originals kept"
    );
    assert!(doc.layers.iter().any(|layer| layer.name == "lower"));
    assert!(doc.layers.iter().any(|layer| layer.name == "upper"));

    // Higher index is the front, so the stamp sits directly above the anchor.
    let anchor = doc
        .layers
        .iter()
        .position(|layer| layer.name == "upper")
        .unwrap();
    assert_eq!(stamp_index(&doc), anchor + 1);
    assert_eq!(doc.layers[stamp_index(&doc)].rect, rect(4, 4));

    let expected = inputs_composite(vec![lower, upper], &doc, rect(4, 4));
    assert_eq!(node_interleaved(&doc.layers[stamp_index(&doc)]), expected);
}

#[test]
fn stamp_visible_skips_and_preserves_hidden_layers() {
    let mut hidden = solid("hidden", rect(4, 4), 200, 0, 0, 255);
    hidden.visible = false;
    let visible = solid("visible", rect(4, 4), 0, 200, 0, 255);
    let mut doc = doc_with(4, 4, vec![hidden, visible.clone()]);

    stamp_scope(&mut doc, StampScope::Visible("1"), "1").expect("stamp");
    assert_eq!(doc.layers.len(), 3);
    assert!(
        doc.layers[0].name == "hidden",
        "hidden layer survives in place"
    );

    let expected = inputs_composite(vec![visible], &doc, rect(4, 4));
    assert_eq!(node_interleaved(&doc.layers[stamp_index(&doc)]), expected);
}

#[test]
fn stamp_visible_refuses_hidden_active() {
    let mut hidden = solid("hidden", rect(4, 4), 200, 0, 0, 255);
    hidden.visible = false;
    let mut doc = doc_with(
        4,
        4,
        vec![hidden, solid("visible", rect(4, 4), 0, 0, 0, 255)],
    );
    let before = doc.clone();
    assert!(stamp_scope(&mut doc, StampScope::Visible("0"), "0").is_none());
    assert_eq!(doc, before, "refusal leaves the document unchanged");
}

#[test]
fn stamp_selected_composites_only_the_selection() {
    let lower = solid("a", rect(4, 4), 10, 0, 0, 255);
    let middle = solid("b", rect(4, 4), 0, 10, 0, 255);
    let top = solid("c", rect(4, 4), 0, 0, 10, 255);
    let mut doc = doc_with(4, 4, vec![lower.clone(), middle.clone(), top]);

    let paths = vec!["0".to_string(), "1".to_string()];
    stamp_scope(&mut doc, StampScope::Selected(&paths), "2").expect("stamp");
    assert_eq!(doc.layers.len(), 4);
    assert_eq!(stamp_index(&doc), 3, "inserted above the active top layer");

    let expected = inputs_composite(vec![lower, middle], &doc, rect(4, 4));
    assert_eq!(node_interleaved(&doc.layers[stamp_index(&doc)]), expected);
}

#[test]
fn stamp_above_a_clip_run_keeps_its_placement() {
    let base = solid("base", rect(4, 4), 10, 0, 0, 255);
    let mut clipped = solid("clipped", rect(4, 4), 0, 10, 0, 255);
    clipped.clipping = true;
    let mut doc = doc_with(4, 4, vec![base, clipped]);
    stamp_scope(&mut doc, StampScope::Visible("1"), "0").expect("stamp");
    let stamp = stamp_index(&doc);
    assert_eq!(stamp, 1, "the stamp sits directly above the base");
    assert!(doc.layers[stamp].clipping, "inside the run joins the clip");

    let base = solid("base", rect(4, 4), 10, 0, 0, 255);
    let mut clipped = solid("clipped", rect(4, 4), 0, 10, 0, 255);
    clipped.clipping = true;
    let mut doc = doc_with(4, 4, vec![base, clipped]);
    stamp_scope(&mut doc, StampScope::Visible("1"), "1").expect("stamp");
    let stamp = stamp_index(&doc);
    assert_eq!(stamp, 2, "the stamp sits above the clipped layer");
    assert!(
        !doc.layers[stamp].clipping,
        "above the run is outside the clip"
    );
}

#[test]
fn stamp_refuses_empty_selection_and_bad_anchor() {
    let mut doc = doc_with(4, 4, vec![solid("a", rect(4, 4), 1, 2, 3, 255)]);
    let before = doc.clone();
    assert!(stamp_scope(&mut doc, StampScope::Selected(&[]), "0").is_none());
    assert!(stamp_scope(&mut doc, StampScope::Visible("0"), "9").is_none());
    assert_eq!(doc, before, "both refusals leave the document unchanged");
}
