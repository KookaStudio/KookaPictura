//! The described-damage policy: bounded edits use a region, unbounded ones a
//! full recomposite.

use crate::cxxqt_object::edit::{
    effect_of, insertion_edit, regions_influence, structural_paths, Edit, Effect,
};
use crate::cxxqt_object::impl_layers::changed_property_region;
use crate::cxxqt_object::metrics::{set_for_this_thread, with};
use crate::cxxqt_object::tests::pixel_layer;
use crate::cxxqt_object::PictureViewRust;
use crate::layer_target::LayerTarget;
use pictura_core::{
    BitDepth, ColorMode, Document, Knockout, Layer, LayerBlock, LayerMask, LayerStack, PsdRect,
    TypeSpec,
};

fn view(layers: Vec<Layer>) -> PictureViewRust {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(layers);
    doc.composite = pictura_render::composite_rgba(&doc);
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        gpu_compute: false,
        ..Default::default()
    }
}

fn full_composites() -> u32 {
    with(|metrics| {
        metrics
            .phases()
            .iter()
            .find(|(label, _)| label == "recomposite_full_composite")
            .map_or(0, |(_, phase)| phase.count)
    })
}

#[test]
fn dimension_change_recomposites_fully() {
    set_for_this_thread(true);
    // The policy maps an unbounded edit to a full rebuild and a bounded edit to
    // a blit; a canvas-size change is the former.
    assert_eq!(effect_of(&Edit::Full), Effect::Rebuild);
    assert_eq!(
        effect_of(&Edit::regions(
            vec![PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            }],
            false
        )),
        Effect::Blit { notify: false }
    );
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    let before = full_composites();
    view.apply_edit("Canvas Size", Edit::Full);
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn unbounded_adjustment_recomposites_fully() {
    let adjustment = Layer {
        name: "adjustment".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        adjustment: Some(pictura_render::encode_brightness_contrast(0, 0)),
        ..Default::default()
    };
    let mut group = pixel_layer("group", 8, 8, (9, 9, 9));
    group.is_group = true;
    let d = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    let mut d = d;
    d.layers = LayerStack::from_layers(vec![adjustment, group]);
    assert_eq!(
        changed_property_region(&d, &["0"], &[None]),
        None,
        "an unmasked adjustment is unbounded"
    );
    assert_eq!(
        changed_property_region(&d, &["1"], &[None]),
        None,
        "a group is unbounded"
    );
    assert_eq!(
        changed_property_region(&d, &["0", "1"], &[None, None]),
        None,
        "a mixed stack with an unbounded member is fully recomposited"
    );
}

#[test]
fn unbounded_effect_recomposites_fully() {
    let mut knockout = pixel_layer("knockout", 10, 10, (200, 10, 10));
    knockout.rect = PsdRect {
        top: 20,
        left: 20,
        bottom: 30,
        right: 30,
    };
    knockout.knockout = Knockout::Shallow;
    knockout.extra_blocks.push(LayerBlock {
        key: *b"lfx2",
        data: Vec::new(),
    });
    let mut d = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![pixel_layer("base", 64, 64, (0, 0, 0)), knockout]);
    // A knockout base is global, so the layer-local reach cannot prove a bound.
    assert_eq!(changed_property_region(&d, &["1"], &[None]), None);

    let bounded = pixel_layer("bounded", 8, 8, (0, 0, 255));
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![bounded]);
    let rect = PsdRect {
        top: 2,
        left: 2,
        bottom: 6,
        right: 6,
    };
    doc.layers[0].rect = rect;
    assert_eq!(
        changed_property_region(&doc, &["0"], &[None]),
        Some(Some(rect))
    );
}

fn tiny() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    }
}

fn placed(name: &str, x: i32, y: i32, rgb: (u8, u8, u8)) -> Layer {
    let mut layer = pixel_layer(name, 6, 5, rgb);
    layer.rect = PsdRect {
        top: y,
        left: x,
        bottom: y + 5,
        right: x + 6,
    };
    layer
}

fn bounded_view() -> PictureViewRust {
    view(vec![
        pixel_layer("base", 8, 8, (0, 0, 0)),
        placed("a", 1, 1, (20, 0, 0)),
        placed("b", 2, 2, (0, 20, 0)),
    ])
}

fn assert_bounded(label: &str, apply: fn(&mut PictureViewRust) -> Edit) {
    set_for_this_thread(true);
    let mut view = bounded_view();
    let edit = apply(&mut view);
    let before = full_composites();
    let applied = view.apply_edit(label, edit);
    assert_eq!(full_composites(), before, "{label} used a full composite");
    assert!(applied.blit.is_some(), "{label} did not use a region");
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
}

fn delete_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let damage = pictura_render::damage::influence(doc, "1");
    assert_eq!(pictura_render::delete_paths(doc, &["1"]), 1);
    Edit::Structural {
        damage: damage.map(|rect| vec![rect]),
    }
}

fn reorder_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let before = pictura_render::damage::influence(doc, "1");
    assert!(pictura_render::move_path(doc, "1", 1));
    let after = pictura_render::damage::influence(doc, "2");
    Edit::Structural {
        damage: pictura_render::damage::union(before, after).map(|rect| vec![rect]),
    }
}

fn duplicate_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let before = pictura_render::damage::influence(doc, "1");
    let created = pictura_render::duplicate_layer(doc, 1);
    assert!(created >= 0);
    let after = pictura_render::damage::influence(doc, &created.to_string());
    Edit::Structural {
        damage: pictura_render::damage::union(before, after).map(|rect| vec![rect]),
    }
}

fn group_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let before = pictura_render::damage::influence_paths(doc, &["1", "2"]);
    let group = pictura_render::group_paths(doc, &["1", "2"]).expect("group");
    let after = pictura_render::damage::influence(doc, &group);
    structural_paths(before, after.map(|rect| vec![rect]))
}

fn merge_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let before = pictura_render::damage::influence_paths(doc, &["1", "2"]);
    let outcome = pictura_render::merge_scope(doc, pictura_render::MergeScope::Down("2"))
        .expect("merge down");
    let after = pictura_render::damage::influence(doc, &outcome.path);
    structural_paths(before, after.map(|rect| vec![rect]))
}

fn type_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let spec = TypeSpec::new("Type", "Liberation Sans", 32.0);
    let path = pictura_render::add_type_layer(doc, "", &spec);
    assert!(!path.is_empty());
    let before = pictura_render::damage::influence(doc, &path);
    assert!(pictura_render::render_text_layer(doc, &path));
    let after = pictura_render::damage::influence(doc, &path);
    Edit::Structural {
        damage: pictura_render::damage::union(before, after).map(|rect| vec![rect]),
    }
}

fn batch_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let before = pictura_render::damage::influence_paths(doc, &["1", "2"]);
    assert!(pictura_render::set_opacity_paths(doc, &["1", "2"], 128) > 0);
    let after = pictura_render::damage::influence_paths(doc, &["1", "2"]);
    regions_influence(before, after, true)
}

fn insert_edit(view: &mut PictureViewRust) -> Edit {
    let doc = view.session.doc.as_mut().expect("document");
    let rgba = [255u8; 4 * 4 * 4];
    let created = pictura_render::add_raster_layer_from_rgba(doc, "inserted", 4, 4, &rgba);
    assert!(!created.is_empty());
    let damage = pictura_render::damage::influence(doc, &created);
    Edit::Structural {
        damage: damage.map(|rect| vec![rect]),
    }
}

#[test]
fn bounded_delete_refreshes_only_its_region() {
    assert_bounded("Delete", delete_edit);
}

#[test]
fn bounded_reorder_refreshes_only_its_region() {
    assert_bounded("Reorder", reorder_edit);
}

#[test]
fn bounded_duplicate_refreshes_only_its_region() {
    assert_bounded("Duplicate", duplicate_edit);
}

#[test]
fn bounded_group_and_ungroup_refresh_only_their_region() {
    assert_bounded("Group", group_edit);
    // Ungrouping the created group changes only its children's union.
    set_for_this_thread(true);
    let mut view = bounded_view();
    let edit = {
        let doc = view.session.doc.as_mut().expect("document");
        let group = pictura_render::group_paths(doc, &["1", "2"]).expect("group");
        let group_rect = pictura_render::damage::influence(doc, &group);
        let ungrouped = pictura_render::ungroup_paths(doc, &[&group]);
        assert_eq!(ungrouped, 1);
        let after = pictura_render::damage::influence_paths(doc, &["1", "2"]);
        structural_paths(group_rect.map(|rect| vec![rect]), after)
    };
    let before = full_composites();
    let applied = view.apply_edit("Ungroup", edit);
    assert_eq!(full_composites(), before);
    assert!(applied.blit.is_some());
    let doc = view.session.doc.as_ref().expect("document");
    assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
}

#[test]
fn bounded_merge_refreshes_only_its_region() {
    assert_bounded("Merge Down", merge_edit);
}

#[test]
fn bounded_type_rasterize_refreshes_only_its_region() {
    assert_bounded("Rasterize Type", type_edit);
}

#[test]
fn bounded_batch_refreshes_only_its_region() {
    assert_bounded("Opacity", batch_edit);
}

#[test]
fn bounded_non_neutral_insertion_refreshes_only_its_region() {
    assert_bounded("Insert", insert_edit);
}

#[test]
fn unbounded_structural_rebuilds() {
    set_for_this_thread(true);
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    let before = full_composites();
    view.apply_edit("Delete", Edit::Structural { damage: None });
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn large_bounded_structural_edit_rebuilds_fully() {
    set_for_this_thread(true);
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    let canvas = PsdRect {
        top: 0,
        left: 0,
        bottom: 8,
        right: 8,
    };
    let before = full_composites();
    view.apply_edit(
        "Delete",
        Edit::Structural {
            damage: Some(vec![canvas]),
        },
    );
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn structural_edit_during_live_session_rebuilds() {
    set_for_this_thread(true);
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    view.session.force_test_session();
    let before = full_composites();
    view.apply_edit(
        "Delete",
        Edit::Structural {
            damage: Some(vec![tiny()]),
        },
    );
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn structural_edit_with_pending_preview_rebuilds() {
    set_for_this_thread(true);
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    view.pending_previews.opacity_changed = true;
    let before = full_composites();
    view.apply_edit(
        "Delete",
        Edit::Structural {
            damage: Some(vec![tiny()]),
        },
    );
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn structural_region_edit_with_solo_mask_rebuilds() {
    set_for_this_thread(true);
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    view.mask_solo = Some("0".into());
    let before = full_composites();
    view.apply_edit(
        "Delete",
        Edit::Structural {
            damage: Some(vec![tiny()]),
        },
    );
    assert_eq!(full_composites(), before + 1);
}

#[test]
fn effect_reach_is_bounded() {
    let mut styled = pixel_layer("styled", 10, 10, (200, 10, 10));
    styled.rect = PsdRect {
        top: 20,
        left: 20,
        bottom: 30,
        right: 30,
    };
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.size", 6.0);
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![styled]);
    let bounds = pictura_render::damage::influence(&doc, "0").expect("bounded");
    assert!(bounds.width() > 10 && bounds.width() < 64, "{bounds:?}");
}

#[test]
fn group_and_effect_visibility_are_bounded() {
    let mut child = pixel_layer("child", 4, 4, (20, 20, 20));
    pictura_render::set_layer_style_value(&mut child, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut child, "dropShadow.size", 6.0);
    let mut group = pixel_layer("group", 8, 8, (9, 9, 9));
    group.is_group = true;
    group.children = LayerStack::from_layers(vec![child]);
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![group]);
    let bounds = pictura_render::damage::influence(&doc, "0").expect("bounded group");
    assert!(bounds.width() > 4, "the child effect reach is included");
}

#[test]
fn knockout_or_passthrough_effect_is_unbounded() {
    let mut knockout = pixel_layer("knockout", 8, 8, (20, 20, 20));
    knockout.knockout = Knockout::Shallow;
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 8, 8, (0, 0, 0)), knockout]);
    assert_eq!(pictura_render::damage::influence(&doc, "1"), None);

    let adjustment = Layer {
        name: "adjustment".into(),
        adjustment: Some(pictura_render::encode_brightness_contrast(10, 0)),
        ..Default::default()
    };
    let mut pass = pixel_layer("pass", 8, 8, (9, 9, 9));
    pass.is_group = true;
    pass.blend = pictura_core::BlendMode::PassThrough;
    pass.children = LayerStack::from_layers(vec![adjustment]);
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pass]);
    assert_eq!(pictura_render::damage::influence(&doc, "0"), None);
}

#[test]
fn neutral_insertion_still_yields_inserted() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    let created = pictura_render::add_layer(&mut doc, 0, "Layer");
    assert!(created >= 0);
    assert_eq!(
        insertion_edit(&doc, &created.to_string()),
        Edit::Inserted,
        "a content-neutral insertion must not recomposite"
    );
}

#[test]
fn bounded_panel_commit_notifies() {
    assert_eq!(
        effect_of(&Edit::regions(vec![tiny()], true)),
        Effect::Blit { notify: true }
    );
}

#[test]
fn bounded_edits_equal_full_recomposite() {
    assert_bounded("Delete", delete_edit);
    assert_bounded("Reorder", reorder_edit);
    assert_bounded("Duplicate", duplicate_edit);
    assert_bounded("Group", group_edit);
    assert_bounded("Merge Down", merge_edit);
    assert_bounded("Rasterize Type", type_edit);
    assert_bounded("Opacity", batch_edit);
    assert_bounded("Insert", insert_edit);
}

#[test]
fn shrinking_feather_equals_full_recomposite() {
    let mask = |feather: f64| LayerMask {
        rect: PsdRect {
            top: 14,
            left: 14,
            bottom: 18,
            right: 18,
        },
        default_color: 0,
        disabled: false,
        data: Some(vec![255u8; 16].into()),
        density: 255,
        feather_bits: feather.to_bits(),
        ..Default::default()
    };
    let adjustment = Layer {
        name: "adjustment".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 32,
            right: 32,
        },
        adjustment: Some(pictura_render::encode_brightness_contrast(30, 0)),
        mask: Some(mask(8.0)),
        ..Default::default()
    };
    let mut doc = Document::new(32, 32, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![adjustment]);
    let before = pictura_render::damage::influence(&doc, "0");
    doc.layers[0].mask.as_mut().expect("mask").set_feather(2.0);
    let after = pictura_render::damage::influence(&doc, "0");
    let damage = pictura_render::damage::union(before, after).expect("bounded");
    let full = pictura_render::composite_rgba(&doc);
    let region = pictura_render::composite_rgba_region(
        &doc,
        damage.left as u32,
        damage.top as u32,
        damage.width() as u32,
        damage.height() as u32,
    );
    let fw = full.width as usize;
    let fp = fw * full.height as usize;
    let rw = region.width as usize;
    let rp = rw * region.height as usize;
    for y in 0..damage.height() as usize {
        for x in 0..damage.width() as usize {
            let fi = (damage.top as usize + y) * fw + damage.left as usize + x;
            let ri = y * rw + x;
            for c in 0..4 {
                assert_eq!(
                    region.data.as_slice()[c * rp + ri],
                    full.data.as_slice()[c * fp + fi],
                    "pixel ({x}, {y}) channel {c}"
                );
            }
        }
    }
}

fn assert_pending_preview_forces_full_capture(mask_refine: bool) {
    let mut view = view(vec![pixel_layer("base", 8, 8, (0, 0, 0))]);
    view.session.record(&LayerTarget::default(), "Open");
    if let Some(doc) = view.session.doc.as_mut() {
        doc.composite.data.as_mut_slice()[45] = 200;
    }
    if mask_refine {
        view.pending_previews.mask_refine = Some("Mask Feather");
    } else {
        view.pending_previews.opacity_changed = true;
    }
    view.apply_edit("Opacity", Edit::regions(vec![tiny()], true));
    let live = view
        .session
        .doc
        .as_ref()
        .expect("document")
        .composite
        .data
        .as_slice()
        .to_vec();
    let captured = view
        .session
        .history
        .current_snapshot()
        .expect("state")
        .doc
        .composite
        .data
        .as_slice()
        .to_vec();
    assert_eq!(
        captured, live,
        "the pending preview was not captured in full"
    );
}

#[test]
fn bounded_region_after_pending_preview_passes_no_hint() {
    assert_pending_preview_forces_full_capture(false);
}

#[test]
fn mask_refine_commit_after_preview_passes_no_hint() {
    assert_pending_preview_forces_full_capture(true);
}
