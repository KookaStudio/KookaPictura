//! The Move tool's Auto-Select hit-test: which layer a press picks, including
//! the isolation scope the Layers panel's Selected filter pushes.

use super::helpers_composite::auto_select_path;
use super::tests::pixel_layer;
use pictura_core::{BitDepth, ColorMode, Document, Layer, PsdRect};

/// `layer` moved so its rect's top-left is `(left, top)`.
fn placed(mut layer: Layer, left: i32, top: i32) -> Layer {
    layer.rect = PsdRect {
        top,
        left,
        bottom: top + layer.rect.height(),
        right: left + layer.rect.width(),
    };
    layer
}

#[test]
fn auto_select_picks_the_topmost_covering_layer() {
    let mut doc = Document::new(6, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("bottom", 6, 4, (255, 0, 0)),
        placed(pixel_layer("top", 2, 2, (0, 255, 0)), 1, 1),
    ];

    assert_eq!(
        auto_select_path(&doc, 1, 1, false, None),
        Some("1".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 4, 1, false, None),
        Some("0".to_string())
    );
    assert_eq!(auto_select_path(&doc, -1, 1, false, None), None);
}

#[test]
fn auto_select_skips_hidden_transparent_and_adjustment_layers() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut hidden = pixel_layer("hidden", 2, 2, (0, 255, 0));
    hidden.visible = false;
    let mut faded = pixel_layer("faded", 2, 2, (0, 255, 0));
    faded.opacity = 0;
    let mut clear = pixel_layer("clear", 2, 2, (0, 255, 0));
    for channel in clear.channels.iter_mut().filter(|c| c.id == -1) {
        channel.data = vec![0u8; 4].into();
    }
    let mut adjustment = pixel_layer("adjust", 2, 2, (0, 0, 255));
    adjustment.adjustment = Some(pictura_core::AdjustmentData {
        key: *b"inv ",
        data: Vec::new(),
    });
    doc.layers = vec![
        pixel_layer("bottom", 4, 4, (255, 0, 0)),
        hidden,
        faded,
        clear,
        adjustment,
    ];

    assert_eq!(
        auto_select_path(&doc, 0, 0, false, None),
        Some("0".to_string())
    );
}

#[test]
fn auto_select_group_resolves_the_top_level_ancestor() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let group = Layer {
        name: "group".into(),
        is_group: true,
        children: vec![placed(pixel_layer("inner", 2, 2, (0, 255, 0)), 0, 0)],
        ..Default::default()
    };
    doc.layers = vec![pixel_layer("bottom", 4, 4, (255, 0, 0)), group];

    assert_eq!(
        auto_select_path(&doc, 1, 1, false, None),
        Some("1/0".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 1, 1, true, None),
        Some("1".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 3, 3, false, None),
        Some("0".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 3, 3, true, None),
        Some("0".to_string())
    );
}

#[test]
fn auto_select_isolation_scopes_the_hit() {
    let mut doc = Document::new(6, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("bottom", 6, 4, (255, 0, 0)),
        placed(pixel_layer("top", 2, 2, (0, 255, 0)), 1, 1),
    ];
    let only_bottom = vec!["0".to_string()];
    let only_top = vec!["1".to_string()];

    // The top layer covers (1,1) but is not isolated, so the bottom shows.
    assert_eq!(
        auto_select_path(&doc, 1, 1, false, Some(&only_bottom)),
        Some("0".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 1, 1, false, Some(&only_top)),
        Some("1".to_string())
    );
    // (4,1) only hits the bottom layer, which is outside this set.
    assert_eq!(auto_select_path(&doc, 4, 1, false, Some(&only_top)), None);
}

#[test]
fn auto_select_isolation_reaches_a_group_descendant() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let group = Layer {
        name: "group".into(),
        is_group: true,
        children: vec![placed(pixel_layer("inner", 2, 2, (0, 255, 0)), 0, 0)],
        ..Default::default()
    };
    doc.layers = vec![pixel_layer("bottom", 4, 4, (255, 0, 0)), group];
    let group_set = vec!["1".to_string()];
    let child_set = vec!["1/0".to_string()];

    assert_eq!(
        auto_select_path(&doc, 1, 1, false, Some(&group_set)),
        Some("1/0".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 1, 1, true, Some(&group_set)),
        Some("1".to_string())
    );
    assert_eq!(
        auto_select_path(&doc, 1, 1, false, Some(&child_set)),
        Some("1/0".to_string())
    );
    // Group mode with a child-only set keeps the child, not its unisolated
    // parent group.
    assert_eq!(
        auto_select_path(&doc, 1, 1, true, Some(&child_set)),
        Some("1/0".to_string())
    );
    // The bottom layer is outside the group set.
    assert_eq!(auto_select_path(&doc, 3, 3, false, Some(&group_set)), None);
}
