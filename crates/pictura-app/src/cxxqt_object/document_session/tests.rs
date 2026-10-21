//! The document session's own checks: the live session, the cancel path, and
//! the replacement of the document.

use super::super::state::PictureViewRust;
use super::super::state::TransformMode;
use super::session::{SessionChange, SessionKind};
use crate::cxxqt_object::impl_filters::{apply_filter_active, cancel_filter_preview};
use crate::cxxqt_object::tests::pixel_layer;
use crate::history::History;
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PixelBuffer, PsdRect};
use pictura_paint::StrokeConfig;
use pictura_select::Selection;

/// A document with one opaque layer that covers the whole canvas.
fn doc() -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 8, 8, (255, 0, 0))]);
    doc.composite = PixelBuffer {
        width: 8,
        height: 8,
        channels: 4,
        data: vec![0u8; 8 * 8 * 4].into(),
    };
    doc
}

/// A layer whose visible pixels are smaller than its rect, so a transform
/// begin trims it and a refused begin has something to restore.
fn trimmed_layer() -> pictura_core::Layer {
    let mut layer = pixel_layer("px", 8, 8, (255, 0, 0));
    let alpha = layer
        .channels
        .iter_mut()
        .find(|c| c.id == -1)
        .expect("the alpha channel");
    for (i, a) in alpha.data.iter_mut().enumerate() {
        *a = if i < 4 { 255 } else { 0 };
    }
    layer
}

fn view() -> PictureViewRust {
    let mut view = PictureViewRust {
        doc: Some(doc()),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

fn view_with_trimmable_layer() -> PictureViewRust {
    let mut doc = doc();
    doc.layers = LayerStack::from_layers(vec![trimmed_layer()]);
    let mut view = PictureViewRust {
        doc: Some(doc),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

fn view_with_two_layers() -> PictureViewRust {
    let mut doc = doc();
    doc.layers = LayerStack::from_layers(vec![
        pixel_layer("px", 8, 8, (255, 0, 0)),
        pixel_layer("px2", 8, 8, (0, 255, 0)),
    ]);
    let mut view = PictureViewRust {
        doc: Some(doc),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

#[test]
fn a_second_session_is_refused() {
    let mut view = view();
    assert!(view.begin_heal(20, 50), "the healing gesture starts");
    assert_eq!(view.active_session_kind(), Some(SessionKind::Healing));
    let history = view.history.count();

    assert!(
        !view.begin_paint(StrokeConfig::default(), "Brush"),
        "a paint begin is refused while the healing gesture is live"
    );
    assert!(view.stroke.is_none(), "no paint stroke was begun");
    assert_eq!(view.history.count(), history, "no history state was added");
    assert_eq!(
        view.active_session_kind(),
        Some(SessionKind::Healing),
        "the healing gesture is still live"
    );
}

#[test]
fn update_begins_keep_one_session() {
    let mut view = view();

    // A filter preview updates the live preview instead of stacking a second.
    assert!(apply_filter_active(&mut view, "gaussian-blur", &[2.0], false).is_some());
    assert!(apply_filter_active(&mut view, "gaussian-blur", &[6.0], false).is_some());
    assert_eq!(view.active_session_kind(), Some(SessionKind::FilterPreview));
    assert!(view.filter_preview.is_some(), "one filter preview is live");

    // A mode preview takes the slot only when no other session is live.
    assert!(
        view.mode_preview_base().is_none(),
        "the filter preview owns the slot"
    );
    cancel_filter_preview(&mut view);
    let base = view.mode_preview_base().expect("the mode preview starts");
    assert_eq!(view.active_session_kind(), Some(SessionKind::ModePreview));
    assert_eq!(
        view.mode_preview_base().expect("the base"),
        base,
        "the base is the first one, not the last preview"
    );
    view.cancel_mode_preview();

    // The magnetic lasso begins the gesture; the lasso attaches to it.
    assert!(view.begin_magnetic(50), "the magnetic gesture starts");
    assert_eq!(view.active_session_kind(), Some(SessionKind::Lasso));
    assert!(view.begin_lasso_session("new"), "the lasso attaches to it");
    assert!(view.edge_map.is_some(), "the magnetic field is still live");
    view.add_lasso_point(1, 1);
    view.add_lasso_point(6, 1);
    assert_eq!(view.pending_lasso.len(), 2, "the lasso points are attached");
    assert_eq!(view.active_session_kind(), Some(SessionKind::Lasso));
}

#[test]
fn a_transform_on_another_target_replaces_the_session() {
    let mut view = view_with_two_layers();
    let first = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the first session");
    view.transform_session = Some(first);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));

    let second = view
        .prepare_transform("1", TransformMode::Free)
        .expect("the second session");
    assert_eq!(second.path, "1", "the new target replaces the session");
    assert_eq!(view.history.count(), 0, "no history state was added");
    view.transform_session = Some(second);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));
}

#[test]
fn a_transform_mode_switch_replaces_the_session() {
    let mut view = view();
    let first = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the first session");
    view.transform_session = Some(first);

    let second = view
        .prepare_transform("0", TransformMode::Skew)
        .expect("the second session");
    assert_eq!(second.mode, TransformMode::Skew, "the new mode replaces it");
    assert_eq!(view.history.count(), 0, "no history state was added");
    view.transform_session = Some(second);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));
}

#[test]
fn a_failed_transform_begin_leaves_the_layer_unchanged() {
    let mut view = view_with_trimmable_layer();
    let before = view.doc.as_ref().expect("document").layers[0].rect;

    // A target that cannot be transformed is refused before anything changes.
    assert!(view
        .prepare_transform("missing", TransformMode::Free)
        .is_none());
    assert_eq!(
        view.doc.as_ref().expect("document").layers[0].rect,
        before,
        "the layer is unchanged"
    );
    assert!(view.transform_session.is_none());
    assert_eq!(view.history.count(), 0);

    // A begin that trims the layer and is then refused puts the layer back.
    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the session");
    let trimmed = view.doc.as_ref().expect("document").layers[0].rect;
    assert_ne!(trimmed, before, "the begin trimmed the layer");
    view.restore_transform_begin(session);
    assert_eq!(
        view.doc.as_ref().expect("document").layers[0].rect,
        before,
        "the refused begin restored the layer"
    );
}

#[test]
fn replacement_installs_incoming_history() {
    let mut view = view();
    let mut history = History::default();
    let mut doc = doc();
    history.capture_live(&mut doc, &None, &view.layer_target, "Opened");
    assert_eq!(history.count(), 1);

    view.replace_document(Some(doc), history, LayerTarget::only("0"), None);
    assert_eq!(view.history.count(), 1, "the incoming history is installed");
    assert_eq!(view.history.label(0), "Opened");
    assert_eq!(
        view.history.index(),
        0,
        "the incoming state is the current one"
    );
}

#[test]
fn undo_cancels_every_session() {
    let mut view = view();
    let history = view.history.count();
    view.mode_preview = Some(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session(), "the session is gone");
    assert_eq!(view.history.count(), history, "no history state was added");
    assert!(view.mode_preview.is_none());
}

#[test]
fn redo_cancels_every_session() {
    let mut view = view();
    view.filter_preview = None;
    view.mode_preview = Some(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session());
    assert!(view.mode_preview.is_none());
}

#[test]
fn jump_cancels_a_filter_preview() {
    let mut view = view();
    assert!(apply_filter_active(&mut view, "gaussian-blur", &[3.0], false).is_some());
    assert!(view.filter_preview.is_some(), "the preview is open");
    assert_eq!(view.active_session_kind(), Some(SessionKind::FilterPreview));

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(view.filter_preview.is_none(), "the preview is discarded");
    assert!(!view.has_active_session());
}

#[test]
fn snapshot_restore_cancels_the_session() {
    let mut view = view();
    view.mode_preview = Some(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session());
    assert!(view.mode_preview.is_none());
}

#[test]
fn a_shape_path_abort_requests_a_rebuild() {
    let mut view = view();
    // A gesture on a shape layer's outline edits the layer, whose path cannot
    // be restored: the abort asks for a rebuild instead of a notification.
    view.path_on_layer = true;
    view.begin_path_gesture();
    assert!(view.path_gesture.is_some(), "the gesture is live");

    let change = view.cancel_active_sessions();
    assert_eq!(
        change.effect,
        crate::cxxqt_object::edit::Effect::Rebuild,
        "a shape-layer path abort asks for a rebuild"
    );
    assert!(change.changed_display());
    assert!(view.path_gesture.is_none());
}

#[test]
fn replacement_clears_outgoing_state() {
    let mut view = view();
    view.deselected_selection = Some(Selection::none(8, 8));
    view.mask_edit_target = Some("0".into());
    view.active_path = crate::cxxqt_object::paths::ActivePath::Saved(3);
    view.pending_previews.opacity_changed = true;
    view.pending_previews.fill_changed = true;
    view.pending_previews.mask_refine = Some("Mask Feather");
    view.pdf_options = Some("options".into());
    view.pdf_password = Some("secret".into());
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.deselected_selection.is_none());
    assert!(view.mask_edit_target.is_none());
    assert!(matches!(
        view.active_path,
        crate::cxxqt_object::paths::ActivePath::Work
    ));
    assert!(!view.pending_previews.opacity_changed);
    assert!(!view.pending_previews.fill_changed);
    assert!(view.pending_previews.mask_refine.is_none());
    assert!(view.pdf_options.is_none());
    assert!(view.pdf_password.is_none());
    assert_eq!(view.document_generation, 1);
}

#[test]
fn replacing_a_document_cancels_every_session() {
    let mut view = view();
    let stroke = pictura_paint::Stroke::begin_at(
        view.doc.as_ref().expect("document"),
        "0",
        StrokeConfig {
            diameter: 2,
            ..Default::default()
        },
    )
    .expect("stroke");
    view.stroke = Some(stroke);
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(!view.has_active_session());
    assert!(view.stroke.is_none());
}

#[test]
fn a_full_change_requests_a_rebuild() {
    let mut view = view();
    view.mode_preview = Some(doc());
    let change = view.cancel_active_sessions();
    assert!(view.mode_preview.is_none());
    assert_eq!(change.effect, crate::cxxqt_object::edit::Effect::Rebuild);
    assert!(change.changed_display());
}

#[test]
fn cancelling_without_a_session_is_a_noop() {
    let mut view = view();
    let change = view.cancel_active_sessions();
    assert_eq!(change.effect, crate::cxxqt_object::edit::Effect::Nothing);
    assert!(!change.changed_display());
}

#[test]
fn cancelling_every_session_restores_state() {
    let mut view = view();
    view.selection = Some(Selection::none(8, 8));
    assert!(view.begin_selection_move());
    assert!(view.has_active_session());
    assert!(view.cancel_active_sessions().effect == crate::cxxqt_object::edit::Effect::Nothing);
    assert!(!view.has_active_session());
    assert!(view.selection_move_origin.is_none());
    assert!(view.selection_move_delta.is_none());
}

#[test]
fn a_notification_only_change_emits_no_blit() {
    let mut view = view();
    view.doc.as_mut().expect("document").work_path = Default::default();
    view.path_gesture = Some((
        crate::cxxqt_object::paths::ActivePath::Work,
        view.doc.as_ref().expect("document").work_path.clone(),
        false,
    ));
    view.doc
        .as_mut()
        .expect("document")
        .work_path
        .finish_editing();
    let change = view.cancel_active_sessions();
    assert_eq!(change.effect, crate::cxxqt_object::edit::Effect::Notify);
    assert!(view.path_gesture.is_none());
}

#[test]
fn reselect_is_unavailable_after_same_size_replacement() {
    let mut view = view();
    view.deselected_selection = Some(Selection::none(8, 8));
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.deselected_selection.is_none());
    assert!(view.selection.is_none());
}

#[test]
fn pdf_password_follows_the_incoming_document() {
    let mut view = view();
    view.pdf_password = Some("old".into());
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        Some("incoming".into()),
    );
    assert_eq!(view.pdf_password.as_deref(), Some("incoming"));
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.pdf_password.is_none());
}

#[test]
fn cancelling_records_no_history() {
    let mut view = view();
    let target = view.layer_target.clone();
    view.history.capture_live(
        view.doc.as_mut().expect("document"),
        &None,
        &target,
        "Start",
    );
    let count = view.history.count();
    view.mode_preview = Some(doc());
    view.cancel_active_sessions();
    assert_eq!(view.history.count(), count);
}

#[test]
fn a_region_change_does_not_notify() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    let change = SessionChange::blit(rect);
    assert_eq!(
        change.effect,
        crate::cxxqt_object::edit::Effect::Blit { notify: false }
    );
    assert_eq!(change.regions.len(), 1);
}

#[test]
fn a_region_change_can_also_notify() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    let change = SessionChange {
        effect: crate::cxxqt_object::edit::Effect::Blit { notify: true },
        regions: vec![super::session::RegionBlit { rect, srgb: None }],
    };
    assert_eq!(
        change.effect,
        crate::cxxqt_object::edit::Effect::Blit { notify: true }
    );
}
