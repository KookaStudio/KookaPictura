use super::*;
use crate::cxxqt_object::impl_filters::{
    apply_filter_active_region, cancel_filter_preview, pixel_filter_target,
};
use crate::cxxqt_object::tests::pixel_layer;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document};

/// A 16×16 embedded Smart Object with a left/right step, so a blur shows.
fn smart_state() -> PictureViewRust {
    let mut doc = Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel_layer("px", 16, 16, (40, 120, 200));
    for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
        ch.data = (0..16 * 16)
            .map(|i| if i % 16 < 8 { 40u8 } else { 200 })
            .collect();
    }
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    assert!(pictura_render::convert_to_smart_object(&mut doc, "0"));
    PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        gpu_compute: false,
        ..Default::default()
    }
}

fn filters(rust: &PictureViewRust) -> Vec<pictura_core::SmartFilter> {
    rust.session.doc.as_ref().unwrap().layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .smart_filters
        .clone()
}

#[test]
fn preview_cancel_restores_layer() {
    let mut rust = smart_state();
    let before = rust.session.doc.as_ref().unwrap().layers[0].clone();
    rust.session.selection = Some(pictura_select::Selection {
        width: 16,
        height: 16,
        data: vec![255; 256],
    });
    assert!(apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, None).is_some());
    assert_eq!(filters(&rust).len(), 1, "the preview pushes the record");
    assert!(rust.session.doc.as_ref().unwrap().layers[0]
        .smart_object
        .as_ref()
        .unwrap()
        .filter_mask
        .is_some());
    assert!(cancel_filter_preview(&mut rust).is_some());
    let after = &rust.session.doc.as_ref().unwrap().layers[0];
    let (a, b) = (
        after.smart_object.as_ref().unwrap(),
        before.smart_object.as_ref().unwrap(),
    );
    assert_eq!(a.smart_filters, b.smart_filters);
    assert_eq!(a.filter_mask, b.filter_mask);
    assert_eq!(a.filter_mask_enabled, b.filter_mask_enabled);
    assert_eq!(after.channels, before.channels);
}

#[test]
fn preview_then_commit_appends_once() {
    let mut rust = smart_state();
    for radius in [1.0, 4.0] {
        assert!(
            apply_filter_active_region(&mut rust, "gaussian-blur", &[radius], false, None)
                .is_some()
        );
    }
    assert_eq!(commit_kind(&mut rust, "gaussian-blur", &[4.0]), Some(true));
    let list = filters(&rust);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Gaussian Blur");
    assert!(rust.session.live.filter_preview().is_none());
}

#[test]
fn gallery_commits_one_entry() {
    let mut rust = smart_state();
    let kinds = vec!["gaussian-blur".to_string(), "emboss".to_string()];
    let mut params = vec![2.0];
    params.extend(catalogue::descriptor("emboss").unwrap().defaults());
    let steps = stack_steps(&kinds, &params).expect("steps");
    assert!(smart_commit(&mut rust, &steps, true));
    let list = filters(&rust);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, pictura_codec::FILTER_GALLERY_NAME);
    let (decoded, gallery) = pictura_codec::decode_pictura_filter(&list[0].options).unwrap();
    assert!(gallery);
    assert_eq!(decoded.len(), 2);
}

#[test]
fn pixel_target_refuses_smart_object() {
    let mut rust = smart_state();
    assert!(pixel_filter_target(&mut rust).is_none());
    assert!(rust
        .filter_error
        .as_deref()
        .is_some_and(|e| e.contains("smart object")));

    let mut linked = smart_state();
    linked.session.doc.as_mut().unwrap().layers[0]
        .smart_object
        .as_mut()
        .unwrap()
        .kind = pictura_core::SmartObjectKind::External;
    assert_eq!(
        commit_kind(&mut linked, "gaussian-blur", &[2.0]),
        Some(false)
    );
    assert!(filters(&linked).is_empty());

    let mut excluded = smart_state();
    assert_eq!(commit_kind(&mut excluded, "liquify", &[]), Some(false));
    assert!(excluded
        .filter_error
        .as_deref()
        .is_some_and(|e| e.starts_with("Rasterize the Smart Object")));
}

#[test]
fn last_filter_appends() {
    let mut rust = smart_state();
    assert_eq!(commit_kind(&mut rust, "gaussian-blur", &[2.0]), Some(true));
    let (kind, params) = rust.last_filter.clone().expect("last filter");
    assert_eq!(commit_kind(&mut rust, &kind, &params), Some(true));
    assert_eq!(filters(&rust).len(), 2);
}
