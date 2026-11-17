use super::*;
use crate::cxxqt_object::{document_session::DocumentSession, tests::pixel_layer};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LockFlags};
use pictura_select::Selection;

#[test]
fn clipboard_coverage_preserves_history() {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers
        .push(pixel_layer("pixels", 8, 8, (20, 40, 60)))
        .unwrap();
    let mut view = PictureViewRust {
        session: DocumentSession::with_document(doc),
        layer_target: LayerTarget::only("0"),
        ..Default::default()
    };
    view.session.selection = Some(Selection::rect(8, 8, 2, 2, 3, 3).feather(1.0));
    view.session.record(&view.layer_target, "Initial");
    let count = view.session.history.count();
    let clip = copy_of(&view, "0", false).unwrap();
    assert_eq!(view.session.history.count(), count);
    let coverage = clip_selection(&view).unwrap();
    let before = view.session.doc.clone().unwrap();
    assert!(pictura_render::clear_layer(
        view.session.doc.as_mut().unwrap(),
        "0",
        Some(&coverage)
    ));
    view.session.record(&view.layer_target, "Cut");
    assert_eq!(view.session.history.count(), count + 1);
    let after = view.session.doc.clone().unwrap();
    assert_eq!(view.session.history.undo().unwrap().doc, before);
    assert_eq!(view.session.history.redo().unwrap().doc, after);
    let doc = view.session.doc.as_mut().unwrap();
    assert!(
        !pictura_render::paste_clip(doc, "0", &clip, (2, 2), PasteMode::Into, Some(&coverage))
            .is_empty()
    );
    view.session.record(&view.layer_target, "Paste Into");
    assert_eq!(view.session.history.count(), count + 2);
    let doc = view.session.doc.as_mut().unwrap();
    doc.layers[0].lock = LockFlags::default().with(LockFlags::PIXELS, true);
    let before = doc.clone();
    assert!(!pictura_render::clear_layer(doc, "0", Some(&coverage)));
    assert_eq!(*doc, before);
}
