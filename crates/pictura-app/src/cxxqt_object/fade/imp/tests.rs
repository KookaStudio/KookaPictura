use super::*;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::tests::pixel_layer;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack};
use pictura_render::composite_rgba;

fn view(color: (u8, u8, u8), mode: ColorMode) -> PictureViewRust {
    let mut doc = Document::new(4, 4, mode, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 4, 4, color)]);
    doc.composite = composite_rgba(&doc);
    PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        ..Default::default()
    }
}

fn set_color(rust: &mut PictureViewRust, color: (u8, u8, u8)) {
    let layer = pictura_render::resolve_path_mut(rust.session.doc.as_mut().unwrap(), "0").unwrap();
    for (id, value) in [0i16, 1, 2].into_iter().zip([color.0, color.1, color.2]) {
        let channel = layer.channels.iter_mut().find(|c| c.id == id).unwrap();
        channel.data = vec![value; 16].into();
    }
}

fn color(rust: &PictureViewRust) -> (u8, u8, u8) {
    let layer = pictura_render::resolve_path(rust.session.doc.as_ref().unwrap(), "0").unwrap();
    let plane = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .data
            .as_slice()[0]
    };
    (plane(0), plane(1), plane(2))
}

fn record_filter(rust: &mut PictureViewRust) {
    let capture = capture(rust).expect("a pixel layer");
    set_color(rust, (220, 140, 100));
    record(rust, capture, "Gaussian Blur");
}

#[test]
fn capture_and_record_keep_the_original_and_effect() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    assert!(available(&rust));
    assert_eq!(name(&rust), "Gaussian Blur");
    assert_eq!(opacity(&rust), 100);
    assert_eq!(mode(&rust), BlendMode::Normal);
    let state = rust.fade.as_ref().expect("record");
    assert_eq!(state.original.channels[0].data.as_slice()[0], 20);
    assert_eq!(state.effected.channels[0].data.as_slice()[0], 220);
}

#[test]
fn a_no_op_effect_is_not_recorded() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    let capture = capture(&rust).expect("a pixel layer");
    record(&mut rust, capture, "Gaussian Blur");
    assert!(rust.fade.is_none());
}

#[test]
fn a_mask_target_is_not_captured() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    rust.mask_edit_target = Some("0".to_string());
    assert!(capture(&rust).is_none());
}

#[test]
fn lab_refuses_the_six_restricted_modes() {
    let mut rust = view((20, 40, 60), ColorMode::Lab);
    record_filter(&mut rust);
    for mode in [
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::Lighten,
        BlendMode::Darken,
        BlendMode::Difference,
        BlendMode::Exclusion,
    ] {
        assert!(!mode_allowed(&rust, mode), "{mode:?}");
    }
    assert!(mode_allowed(&rust, BlendMode::Normal));
    assert!(mode_allowed(&rust, BlendMode::Multiply));
}

#[test]
fn a_pixel_edit_clears_the_record() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    rust.apply_edit("Fill", Edit::Full);
    assert!(rust.fade.is_none());
}

#[test]
fn a_document_replacement_clears_the_record() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("other", 2, 2, (1, 2, 3))]);
    rust.replace_document(
        Some(doc),
        crate::history::History::default(),
        LayerTarget::only("0"),
        None,
    );
    assert!(rust.fade.is_none());
    assert!(!available(&rust));
}

#[test]
fn a_metadata_edit_keeps_the_record() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    rust.apply_edit("Rename", Edit::Meta);
    assert!(rust.fade.is_some());
}

#[test]
fn preview_does_not_change_the_retained_values() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    preview(&mut rust, 25, BlendMode::Normal).expect("preview");
    assert_eq!(color(&rust), (70, 65, 70));
    assert_eq!(opacity(&rust), 100);
    cancel(&mut rust).expect("cancel");
    assert_eq!(color(&rust), (220, 140, 100));
}

#[test]
fn apply_updates_the_record_and_reopening_recomputes() {
    let mut rust = view((20, 40, 60), ColorMode::Rgb);
    record_filter(&mut rust);
    let applied = apply(&mut rust, 50, BlendMode::Normal).expect("apply");
    assert_eq!(applied.label, "Fade Gaussian Blur");
    assert_eq!(applied.state.opacity, 50);
    assert_eq!(color(&rust), (120, 90, 80));
    preview(&mut rust, 25, BlendMode::Normal).expect("reopen");
    assert_eq!(color(&rust), (70, 65, 70));
}
