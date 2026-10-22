//! The document session's own checks: the live session, the cancel path, and
//! the replacement of the document.

use super::super::state::PictureViewRust;
use super::active::SessionKind;
use super::session::{DocumentSession, SessionChange};
use super::transform::TransformMode;
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
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc()),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

fn view_with_trimmable_layer() -> PictureViewRust {
    let mut doc = doc();
    doc.layers = LayerStack::from_layers(vec![trimmed_layer()]);
    let mut view = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
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
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
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
    let history = view.session.history.count();

    assert!(
        !view.begin_paint(StrokeConfig::default(), "Brush"),
        "a paint begin is refused while the healing gesture is live"
    );
    assert!(
        view.session.live.stroke().is_none(),
        "no paint stroke was begun"
    );
    assert_eq!(
        view.session.history.count(),
        history,
        "no history state was added"
    );
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
    assert!(
        view.session.live.filter_preview().is_some(),
        "one filter preview is live"
    );

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
    assert!(
        view.session.live.edge_map().is_some(),
        "the magnetic field is still live"
    );
    view.add_lasso_point(1, 1);
    view.add_lasso_point(6, 1);
    assert_eq!(
        view.session.live.lasso_points().map(|points| points.len()),
        Some(2),
        "the lasso points are attached"
    );
    assert_eq!(view.active_session_kind(), Some(SessionKind::Lasso));
}

#[test]
fn a_transform_on_another_target_replaces_the_session() {
    let mut view = view_with_two_layers();
    let first = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the first session");
    view.session.live.set_transform(first);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));

    let second = view
        .prepare_transform("1", TransformMode::Free)
        .expect("the second session");
    assert_eq!(second.path, "1", "the new target replaces the session");
    assert_eq!(
        view.session.history.count(),
        0,
        "no history state was added"
    );
    view.session.live.set_transform(second);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));
}

#[test]
fn a_transform_mode_switch_replaces_the_session() {
    let mut view = view();
    let first = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the first session");
    view.session.live.set_transform(first);

    let second = view
        .prepare_transform("0", TransformMode::Skew)
        .expect("the second session");
    assert_eq!(second.mode, TransformMode::Skew, "the new mode replaces it");
    assert_eq!(
        view.session.history.count(),
        0,
        "no history state was added"
    );
    view.session.live.set_transform(second);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Transform));
}

#[test]
fn a_failed_transform_begin_leaves_the_layer_unchanged() {
    let mut view = view_with_trimmable_layer();
    let before = view.session.doc.as_ref().expect("document").layers[0].rect;

    // A target that cannot be transformed is refused before anything changes.
    assert!(view
        .prepare_transform("missing", TransformMode::Free)
        .is_none());
    assert_eq!(
        view.session.doc.as_ref().expect("document").layers[0].rect,
        before,
        "the layer is unchanged"
    );
    assert!(view.session.live.transform().is_none());
    assert_eq!(view.session.history.count(), 0);

    // A begin that trims the layer and is then refused puts the layer back.
    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the session");
    let trimmed = view.session.doc.as_ref().expect("document").layers[0].rect;
    assert_ne!(trimmed, before, "the begin trimmed the layer");
    view.restore_transform_begin(session);
    assert_eq!(
        view.session.doc.as_ref().expect("document").layers[0].rect,
        before,
        "the refused begin restored the layer"
    );
}

#[test]
fn the_slot_names_the_live_session_kind() {
    let mut view = view_with_two_layers();
    let first = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the first session");
    view.session.live.set_transform(first);

    assert_eq!(
        view.active_session_kind(),
        Some(SessionKind::Transform),
        "the slot names the live session"
    );
    // Every other kind's state is absent: the slot holds one session.
    assert!(view.session.live.stroke().is_none(), "no paint stroke");
    assert!(view.session.live.mask_stroke().is_none(), "no mask stroke");
    assert!(
        view.session.live.heal_stroke().is_none(),
        "no healing gesture"
    );
    assert!(
        view.session.live.selection_gesture().is_none(),
        "no lasso gesture"
    );
    assert!(
        view.session.live.selection_move_origin().is_none(),
        "no selection move"
    );
    assert!(view.session.live.path().is_none(), "no path gesture");
    assert!(
        view.session.live.filter_preview().is_none(),
        "no filter preview"
    );
    assert!(
        view.session.live.mode_preview().is_none(),
        "no mode preview"
    );
}

#[test]
fn only_the_live_sessions_state_exists() {
    let mut view = view();
    assert!(apply_filter_active(&mut view, "gaussian-blur", &[2.0], false).is_some());
    assert_eq!(
        view.active_session_kind(),
        Some(SessionKind::FilterPreview),
        "the slot names the filter preview"
    );
    assert!(
        view.session.live.filter_preview().is_some(),
        "the layer preview is live"
    );
    assert!(
        view.session.live.mask_filter_preview().is_none(),
        "no mask preview is live beside it"
    );

    let change = view.cancel_active_sessions();
    assert!(
        change.changed_display(),
        "the cancel restores the pre-preview pixels"
    );
    assert!(
        view.session.live.filter_preview().is_none(),
        "the preview is discarded"
    );
    assert!(!view.has_active_session(), "the session is gone");
}

#[test]
fn a_filter_commit_leaves_a_live_transform() {
    let mut view = view();
    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the transform");
    view.session.live.set_transform(session);

    assert!(apply_filter_active(&mut view, "gaussian-blur", &[2.0], true).is_some());
    assert_eq!(
        view.active_session_kind(),
        Some(SessionKind::Transform),
        "the filter commit does not take the slot"
    );
    assert!(view.session.live.transform().is_some());
}

#[test]
fn cancelling_a_stroke_leaves_another_session_live() {
    let mut view = view();
    view.session.live.set_mode_preview(doc());

    assert!(
        view.cancel_stroke().is_none(),
        "there is no paint session to cancel"
    );
    assert_eq!(
        view.active_session_kind(),
        Some(SessionKind::ModePreview),
        "the other session still owns the slot"
    );
    assert!(view.session.live.mode_preview().is_some());
}

#[test]
fn replacement_installs_incoming_history() {
    let mut view = view();
    let mut history = History::default();
    let mut doc = doc();
    history.capture_live(&mut doc, &None, &view.layer_target, "Opened");
    assert_eq!(history.count(), 1);

    view.replace_document(Some(doc), history, LayerTarget::only("0"), None);
    assert_eq!(
        view.session.history.count(),
        1,
        "the incoming history is installed"
    );
    assert_eq!(view.session.history.label(0), "Opened");
    assert_eq!(
        view.session.history.index(),
        0,
        "the incoming state is the current one"
    );
}

#[test]
fn undo_cancels_every_session() {
    let mut view = view();
    let history = view.session.history.count();
    view.session.live.set_mode_preview(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session(), "the session is gone");
    assert_eq!(
        view.session.history.count(),
        history,
        "no history state was added"
    );
    assert!(view.session.live.mode_preview().is_none());
}

#[test]
fn redo_cancels_every_session() {
    let mut view = view();
    view.session.live.clear_filter_preview();
    view.session.live.set_mode_preview(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session());
    assert!(view.session.live.mode_preview().is_none());
}

#[test]
fn jump_cancels_a_filter_preview() {
    let mut view = view();
    assert!(apply_filter_active(&mut view, "gaussian-blur", &[3.0], false).is_some());
    assert!(
        view.session.live.filter_preview().is_some(),
        "the preview is open"
    );
    assert_eq!(view.active_session_kind(), Some(SessionKind::FilterPreview));

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(
        view.session.live.filter_preview().is_none(),
        "the preview is discarded"
    );
    assert!(!view.has_active_session());
}

#[test]
fn snapshot_restore_cancels_the_session() {
    let mut view = view();
    view.session.live.set_mode_preview(doc());
    assert!(view.has_active_session());

    let change = view.cancel_active_sessions();
    assert!(change.changed_display());
    assert!(!view.has_active_session());
    assert!(view.session.live.mode_preview().is_none());
}

#[test]
fn a_shape_path_abort_requests_a_rebuild() {
    let mut view = view();
    // A gesture on a shape layer's outline edits the layer, whose pixels
    // change: the abort restores the outline and asks for a rebuild.
    view.session
        .live
        .set_path(crate::cxxqt_object::document_session::PathRestore::Layer {
            target: "0".into(),
            subpaths: Vec::new(),
        });
    assert!(view.has_active_session(), "the gesture is live");

    let change = view.cancel_active_sessions();
    assert_eq!(
        change.effect,
        crate::cxxqt_object::edit::Effect::Rebuild,
        "a shape-layer path abort asks for a rebuild"
    );
    assert!(change.changed_display());
    assert!(!view.has_active_session(), "the gesture is gone");
    let layer = &view.session.doc.as_ref().expect("document").layers[0];
    assert!(layer.vector_mask.is_some(), "the layer's outline is back");
}

#[test]
fn a_session_runs_without_a_qobject() {
    // The session owns the document, the selection, and the history, so it
    // runs without a QObject: install, record, undo.
    let mut session = DocumentSession::default();
    let mut doc = doc();
    let mut history = History::default();
    history.capture_live(&mut doc, &None, &LayerTarget::only("0"), "Start");
    session.install(Some(doc), history, None);
    assert_eq!(session.history.count(), 1);
    assert_eq!(session.history.index(), 0);

    let mut target = LayerTarget::only("0");
    session.record(&target, "Edit");
    assert_eq!(session.history.count(), 2, "the edit is recorded");
    assert!(session.dirty);

    let restored = session
        .restore(&mut target, History::undo_live, History::undo)
        .expect("the undo restores a state");
    assert_eq!(session.history.index(), 0, "the undo steps back");
    assert_eq!(session.history.count(), 2, "the history keeps both states");
    let _ = restored;
    assert!(session.doc.is_some(), "the document is still installed");
    assert!(!session.doc.as_ref().expect("document").layers.is_empty());
}

#[test]
fn replacement_clears_outgoing_state() {
    let mut view = view();
    view.session.deselected_selection = Some(Selection::none(8, 8));
    view.mask_edit_target = Some("0".into());
    view.mask_solo = Some("0".into());
    view.mask_overlay = Some("0".into());
    view.isolation = Some(vec!["0".into()]);
    view.ruler = Some(pictura_core::Ruler {
        a: (0.0, 0.0),
        b: (1.0, 1.0),
    });
    view.session
        .doc
        .as_mut()
        .expect("document")
        .saved_paths
        .push(pictura_core::path::NamedPath {
            name: "Saved".into(),
            path: Default::default(),
        });
    view.active_path = crate::cxxqt_object::paths::ActivePath::Saved(0);
    view.pending_previews.opacity_changed = true;
    view.pending_previews.fill_changed = true;
    view.pending_previews.mask_refine = Some("Mask Feather");
    view.session.pdf_options = Some("options".into());
    view.session.pdf_password = Some("secret".into());
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.session.deselected_selection.is_none());
    assert!(view.mask_edit_target.is_none());
    assert!(view.mask_solo.is_none());
    assert!(view.mask_overlay.is_none());
    assert!(view.isolation.is_none());
    assert!(view.ruler.is_none());
    assert!(matches!(
        view.active_path,
        crate::cxxqt_object::paths::ActivePath::Work
    ));
    assert!(!view.pending_previews.opacity_changed);
    assert!(!view.pending_previews.fill_changed);
    assert!(view.pending_previews.mask_refine.is_none());
    assert!(view.session.pdf_options.is_none());
    assert!(view.session.pdf_password.is_none());
    assert_eq!(view.session.document_generation, 1);
}

#[test]
fn replacing_a_document_cancels_every_session() {
    let mut view = view();
    let stroke = pictura_paint::Stroke::begin_at(
        view.session.doc.as_ref().expect("document"),
        "0",
        StrokeConfig {
            diameter: 2,
            ..Default::default()
        },
    )
    .expect("stroke");
    let session = crate::cxxqt_object::document_session::StrokeSession {
        stroke: Some(stroke),
        ..Default::default()
    };
    view.session.live.set_stroke_session(session);
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(!view.has_active_session());
    assert!(view.session.live.stroke().is_none());
}

#[test]
fn a_full_change_requests_a_rebuild() {
    let mut view = view();
    view.session.live.set_mode_preview(doc());
    let change = view.cancel_active_sessions();
    assert!(view.session.live.mode_preview().is_none());
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
    view.session.selection = Some(Selection::none(8, 8));
    assert!(view.begin_selection_move());
    assert!(view.has_active_session());
    assert!(view.cancel_active_sessions().effect == crate::cxxqt_object::edit::Effect::Nothing);
    assert!(!view.has_active_session());
    assert!(view.session.live.selection_move_origin().is_none());
    assert!(view.session.live.selection_move_delta().is_none());
}

#[test]
fn a_notification_only_change_emits_no_blit() {
    let mut view = view();
    view.session.doc.as_mut().expect("document").work_path = Default::default();
    view.session.live.set_path(
        crate::cxxqt_object::document_session::PathRestore::WorkOrSaved {
            active: crate::cxxqt_object::paths::ActivePath::Work,
            path: view
                .session
                .doc
                .as_ref()
                .expect("document")
                .work_path
                .clone(),
        },
    );
    view.session
        .doc
        .as_mut()
        .expect("document")
        .work_path
        .finish_editing();
    let change = view.cancel_active_sessions();
    assert_eq!(change.effect, crate::cxxqt_object::edit::Effect::Notify);
    assert!(!view.has_active_session(), "the gesture is gone");
}

#[test]
fn reselect_is_unavailable_after_same_size_replacement() {
    let mut view = view();
    view.session.deselected_selection = Some(Selection::none(8, 8));
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.session.deselected_selection.is_none());
    assert!(view.session.selection.is_none());
}

#[test]
fn pdf_password_follows_the_incoming_document() {
    let mut view = view();
    view.session.pdf_password = Some("old".into());
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        Some("incoming".into()),
    );
    assert_eq!(view.session.pdf_password.as_deref(), Some("incoming"));
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::default(),
        None,
    );
    assert!(view.session.pdf_password.is_none());
}

#[test]
fn cancelling_records_no_history() {
    let mut view = view();
    let target = view.layer_target.clone();
    view.session.history.capture_live(
        view.session.doc.as_mut().expect("document"),
        &None,
        &target,
        "Start",
    );
    let count = view.session.history.count();
    view.session.live.set_mode_preview(doc());
    view.cancel_active_sessions();
    assert_eq!(view.session.history.count(), count);
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
