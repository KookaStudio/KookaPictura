//! The described-damage policy: bounded edits use a region, unbounded ones a
//! full recomposite.

use crate::cxxqt_object::edit::{effect_of, Edit, Effect};
use crate::cxxqt_object::impl_layers::changed_property_region;
use crate::cxxqt_object::metrics::{set_for_this_thread, with};
use crate::cxxqt_object::tests::pixel_layer;
use crate::cxxqt_object::PictureViewRust;
use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack, PsdRect};

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
        effect_of(&Edit::Regions {
            rects: vec![PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            }],
            notify: false,
        }),
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
    let mut styled = pixel_layer("styled", 10, 10, (200, 10, 10));
    styled.rect = PsdRect {
        top: 20,
        left: 20,
        bottom: 30,
        right: 30,
    };
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.size", 6.0);
    let mut d = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    d.layers = LayerStack::from_layers(vec![styled]);
    // A general edit of an effect-bearing layer cannot be confined: full.
    assert_eq!(changed_property_region(&d, &["0"], &[None]), None);
    // The Layer Style preview uses the bounded paint reach instead.
    let bounds = pictura_render::layer_paint_bounds(&d, "0").expect("paint bounds");
    assert!(bounds.width() > 10 && bounds.width() < 64, "{bounds:?}");

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
