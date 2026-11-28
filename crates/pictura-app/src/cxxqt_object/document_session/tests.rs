//! The document session's own checks: the live session, the cancel path, and
//! the replacement of the document.

use super::super::state::PictureViewRust;
use super::active::SessionKind;
use super::session::DocumentSession;
use super::transform::{RepeatTransform, TransformMode, TransformTarget};
use crate::cxxqt_object::impl_filters::{apply_filter_active, cancel_filter_preview};
use crate::cxxqt_object::presentation::{present_core, Presentation, Rebuild};
use crate::cxxqt_object::tests::pixel_layer;
use crate::history::{History, Snapshot};
use crate::layer_target::LayerTarget;
use pictura_core::{BitDepth, ColorMode, Document, LayerStack, LockFlags, PixelBuffer, PsdRect};
use pictura_paint::StrokeConfig;
use pictura_select::Selection;

/// A document with one opaque layer that covers the whole canvas.
fn doc() -> Document {
    let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![pixel_layer("px", 8, 8, (255, 0, 0))]);
    doc.composite
        .replace(PixelBuffer {
            width: 8,
            height: 8,
            channels: 4,
            data: vec![0u8; 8 * 8 * 4].into(),
        })
        .expect("valid composite");
    doc
}

/// A layer whose visible pixels are smaller than its rect, so a transform
/// begin trims it and a refused begin has something to restore.
fn trimmed_layer() -> pictura_core::Layer {
    let mut layer = pixel_layer("px", 8, 8, (255, 0, 0));
    let alpha = layer
        .channels_mut()
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
fn geometry_selection_clear_discards_quick_mask_and_clip() {
    let mut view = view();
    let remembered = Selection::rect(8, 8, 1, 1, 3, 3);
    view.session.deselected_selection = Some(remembered.clone());
    view.session
        .set_selection(Some(Selection::rect(8, 8, 2, 2, 4, 4)));
    assert!(view.set_quick_mask_mode(true));
    view.session
        .set_selection(Some(Selection::rect(8, 8, 3, 3, 2, 2)));
    view.session
        .doc
        .set(Some(Document::new(16, 16, ColorMode::Rgb, BitDepth::Eight)));
    let revision = view.session.selection_revision;
    let history = view.session.history.count();

    view.session.clear_geometry_selection();

    assert!(view.session.selection.is_none());
    assert!(view.session.quick_mask.coverage.is_none());
    assert!(view.session.quick_mask.active);
    assert_eq!(view.session.selection_revision, revision.wrapping_add(1));
    assert_eq!(view.session.history.count(), history);
    assert_eq!(view.session.quick_mask.deselected, Some(remembered.clone()));
    assert!(view.set_quick_mask_mode(false));
    assert!(view.session.selection.is_none());
    assert_eq!(view.session.deselected_selection, Some(remembered));
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
fn a_linked_transform_includes_a_hidden_partner() {
    let mut view = view_with_two_layers();
    view.session.link_sets.insert("0".to_string(), 1);
    view.session.link_sets.insert("1".to_string(), 1);
    if let Some(doc) = view.session.doc.as_mut() {
        if let Some(layer) = pictura_render::resolve_path_mut(doc, "1") {
            layer.visible = false;
        }
    }

    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the linked session");
    assert_eq!(session.path, "0", "the started-on layer stays the primary");
    let mut members: Vec<&str> = session
        .members
        .iter()
        .map(|(path, _)| path.as_str())
        .collect();
    members.sort_unstable();
    assert_eq!(
        members,
        vec!["0", "1"],
        "the hidden linked partner transforms with the set"
    );
    assert_eq!(
        session.orig_rect,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 8,
            right: 8,
        },
        "the box is the union of every member"
    );
}

#[test]
fn a_linked_transform_skips_a_position_locked_partner() {
    let mut view = view_with_two_layers();
    view.session.link_sets.insert("0".to_string(), 1);
    view.session.link_sets.insert("1".to_string(), 1);
    if let Some(doc) = view.session.doc.as_mut() {
        if let Some(layer) = pictura_render::resolve_path_mut(doc, "1") {
            layer.lock = layer.lock.with(LockFlags::POSITION, true);
        }
    }

    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the session");
    assert_eq!(
        session
            .members
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        vec!["0"],
        "a position-locked partner stays put"
    );
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
        change.has_display_change(),
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
    assert!(change.has_display_change());
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
    assert!(change.has_display_change());
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
    assert!(change.has_display_change());
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
    assert!(change.has_display_change());
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
        change.rebuild,
        Rebuild::Sync,
        "a shape-layer path abort asks for a rebuild"
    );
    assert!(change.has_display_change());
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
        protractor: None,
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
    view.session.pending_previews.opacity_changed = true;
    view.session.pending_previews.fill_changed = true;
    view.session.pending_previews.mask_refine = Some("Mask Feather");
    view.session.pdf_options = Some("options".into());
    view.session.pdf_password = Some("secret".into());
    view.save_options = Some(Default::default());
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
    assert!(!view.session.pending_previews.opacity_changed);
    assert!(!view.session.pending_previews.fill_changed);
    assert!(view.session.pending_previews.mask_refine.is_none());
    assert!(view.session.pdf_options.is_none());
    assert!(view.session.pdf_password.is_none());
    assert!(
        view.save_options.is_none(),
        "the Save As kind must not leak"
    );
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
    assert_eq!(change.rebuild, Rebuild::Sync);
    assert!(change.has_display_change());
}

#[test]
fn cancelling_without_a_session_is_a_noop() {
    let mut view = view();
    let change = view.cancel_active_sessions();
    assert!(change.is_empty());
    assert!(!change.has_display_change());
}

#[test]
fn cancelling_every_session_restores_state() {
    let mut view = view();
    view.session.selection = Some(Selection::none(8, 8));
    assert!(view.begin_selection_move());
    assert!(view.has_active_session());
    assert!(view.cancel_active_sessions().is_empty());
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
    assert!(change.notify);
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
    let change = Presentation::regions(vec![rect], false);
    assert_eq!(change.regions, vec![rect]);
    assert!(!change.notify);
    assert_eq!(change.rebuild, Rebuild::None);
}

#[test]
fn a_region_change_can_also_notify() {
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    let change = Presentation {
        regions: vec![rect],
        notify: true,
        ..Presentation::none()
    };
    assert_eq!(change.regions, vec![rect]);
    assert!(change.notify);
}

fn group_layer(name: &str, children: Vec<pictura_core::Layer>) -> pictura_core::Layer {
    let mut group = pixel_layer(name, 8, 8, (0, 0, 0));
    group.set_group_children(pictura_core::LayerStack::default());
    group.set_group_children(LayerStack::from_layers(children));
    group
}

fn view_with_group() -> PictureViewRust {
    let mut doc = doc();
    doc.layers = LayerStack::from_layers(vec![group_layer(
        "Group",
        vec![
            pixel_layer("child-a", 8, 8, (255, 0, 0)),
            pixel_layer("child-b", 8, 8, (0, 255, 0)),
        ],
    )]);
    let mut view = PictureViewRust {
        session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
        ..Default::default()
    };
    view.layer_target = LayerTarget::only("0");
    view
}

#[test]
fn duplicate_target_names_every_copy() {
    let mut view = view_with_two_layers();
    let created = {
        let doc = view.session.doc.as_mut().expect("document");
        pictura_render::duplicate_paths(doc, &["0", "1"])
    };
    let target = view.duplicate_target(&created);
    assert_eq!(target.selected(), ["3".to_string(), "1".to_string()]);
    assert_eq!(target.anchor(), Some("1"));
    let doc = view.session.doc.as_ref().expect("document");
    for path in target.selected() {
        let layer = pictura_render::resolve_path(doc, path).expect("copy resolves");
        assert!(layer.name.ends_with(" copy"), "{} is {}", path, layer.name);
    }
}

#[test]
fn merge_target_selects_the_result() {
    let mut view = view_with_two_layers();
    let outcome = {
        let doc = view.session.doc.as_mut().expect("document");
        pictura_render::merge_scope(doc, pictura_render::MergeScope::Down("1")).expect("merge")
    };
    let target = view.merge_target(&outcome.path);
    assert_eq!(target.selected(), std::slice::from_ref(&outcome.path));
    assert_eq!(target.anchor(), Some(outcome.path.as_str()));

    let mut view = view_with_two_layers();
    let outcome = {
        let doc = view.session.doc.as_mut().expect("document");
        pictura_render::merge_scope(doc, pictura_render::MergeScope::Visible("1"))
            .expect("merge visible")
    };
    let target = view.merge_target(&outcome.path);
    assert_eq!(target.selected(), std::slice::from_ref(&outcome.path));
    assert_eq!(target.anchor(), Some(outcome.path.as_str()));
}

#[test]
fn group_target_selects_the_group() {
    let mut view = view_with_two_layers();
    let selected = vec!["0".to_string(), "1".to_string()];
    let grouped = selected.clone();
    let path = {
        let doc = view.session.doc.as_mut().expect("document");
        pictura_render::group_paths(doc, &["0", "1"]).expect("group")
    };
    let target = view.group_target(&grouped, &selected, &path);
    assert_eq!(target.selected(), std::slice::from_ref(&path));
    assert_eq!(target.anchor(), Some(path.as_str()));
}

#[test]
fn ungroup_target_selects_the_children() {
    let mut view = view_with_group();
    let groups = vec![(String::new(), 0usize, 2usize)];
    let survivors: Vec<(String, usize)> = Vec::new();
    let changed = {
        let doc = view.session.doc.as_mut().expect("document");
        pictura_render::ungroup_paths(doc, &["0"])
    };
    assert_eq!(changed, 1);
    let target = view.ungroup_target(&groups, &survivors);
    assert_eq!(target.selected(), ["0".to_string(), "1".to_string()]);
    assert_eq!(target.anchor(), Some("0"));
}

#[test]
fn document_scoped_state_is_cleared_with_the_document() {
    let mut view = view();
    view.session.solo_snapshot = Some(std::collections::HashMap::from([("0".to_string(), true)]));
    view.expanded_groups = vec!["0".to_string()];
    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::only("0"),
        None,
    );
    assert!(view.session.solo_snapshot.is_none());
    assert!(view.expanded_groups.is_empty());
}

fn record_fade(view: &mut PictureViewRust) {
    let capture = crate::cxxqt_object::fade::capture(view).expect("a pixel layer");
    {
        let layer =
            pictura_render::resolve_path_mut(view.session.doc.as_mut().expect("document"), "0")
                .expect("the target layer");
        for (id, value) in [0i16, 1, 2].into_iter().zip([0u8, 255, 0]) {
            let channel = layer
                .channels_mut()
                .iter_mut()
                .find(|channel| channel.id == id)
                .expect("a colour channel");
            channel.data = vec![value; 8 * 8].into();
        }
    }
    crate::cxxqt_object::fade::record(view, capture, "Gaussian Blur");
    assert!(view.session.fade.is_some(), "the fade record is installed");
}

fn repeat_transform() -> RepeatTransform {
    RepeatTransform {
        target: TransformTarget::Layer,
        scale_x: 1.0,
        scale_y: 1.0,
        angle: 0.0,
        skew_x: 0.0,
        skew_y: 0.0,
        dx: 0.0,
        dy: 0.0,
        pivot_offset: (0.0, 0.0),
    }
}

fn fill_edit_state(view: &mut PictureViewRust) {
    record_fade(view);
    view.session.last_transform = Some(repeat_transform());
    view.session.link_sets.insert("0".into(), 1);
    view.session
        .link_remap
        .replace(std::collections::HashMap::from([("0".into(), 1)]));
    view.session.solo_snapshot = Some(std::collections::HashMap::from([("0".to_string(), true)]));
    view.session.pending_previews.opacity_changed = true;
    view.session.pending_previews.fill_changed = true;
    view.session.pending_previews.mask_refine = Some("Mask Feather");
}

fn assert_edit_state_clear(view: &PictureViewRust) {
    assert!(view.session.fade.is_none(), "Fade must not survive");
    assert!(view.session.last_transform.is_none());
    assert!(view.session.link_sets.is_empty());
    assert!(view.session.link_remap.is_none());
    assert!(view.session.solo_snapshot.is_none());
    assert!(!view.session.pending_previews.any());
    assert!(!crate::cxxqt_object::fade::available(view));
}

#[test]
fn snapshot_restore_clears_edit_state() {
    let mut view = view();
    fill_edit_state(&mut view);
    let snapshot = Snapshot {
        doc: doc(),
        selection: None,
        quick_mask: None,
    };

    let target = view
        .session
        .restore_named_snapshot(snapshot, LayerTarget::only("0"), "Snapshot");

    assert_eq!(target.anchor(), Some("0"));
    assert_edit_state_clear(&view);
    assert_eq!(view.session.history.count(), 1, "the restore is recorded");
    assert_eq!(view.session.history.label(0), "Snapshot");
}

#[test]
fn replacement_clears_edit_state() {
    let mut view = view();
    fill_edit_state(&mut view);
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    view.path_drag_damage = Some(Some(rect));
    view.move_above = Some(cxx_qt_lib::QImage::default());
    view.move_above_origin = Some((1, 2));
    view.move_above_scale = 2;
    view.move_above_presented = Some((1, rect));
    view.transform_preview_level = 2;
    view.smart_filter_edit = Some(("0".into(), 0));
    view.warp_text_original = Some(("0".into(), pixel_layer("px", 8, 8, (1, 2, 3))));

    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::only("0"),
        None,
    );

    assert_edit_state_clear(&view);
    assert!(view.path_drag_damage.is_none());
    assert!(view.move_above.is_none());
    assert!(view.move_above_origin.is_none());
    assert_eq!(view.move_above_scale, 1);
    assert!(view.move_above_presented.is_none());
    assert_eq!(view.transform_preview_level, 0);
    assert!(view.smart_filter_edit.is_none());
    assert!(view.warp_text_original.is_none());
}

/// A replaced document leaves no preview field behind, including the base
/// origin the document-scoped clear used to miss.
#[test]
fn a_replaced_document_clears_the_preview_fields() {
    let mut view = view();
    view.move_base_origin = Some((3, 4));
    view.move_prepared_layer = "0\n1".to_string();
    view.transform_preview_level = 2;

    view.replace_document(
        Some(doc()),
        History::default(),
        LayerTarget::only("0"),
        None,
    );

    assert!(
        view.move_base_origin.is_none(),
        "the base origin follows the outgoing document"
    );
    assert!(
        view.move_prepared_layer.is_empty(),
        "the prepared-group key names the outgoing layers"
    );
    assert_eq!(view.transform_preview_level, 0, "the preview level is back");
}

/// Cancelling a Move controls gesture through the document session must clear
/// the above-layer image and the drag flag, not just the fields the Move tool's
/// own cancel clears: a landing composite reads the flag.
#[test]
fn a_cancelled_controls_gesture_clears_the_above_layer() {
    let mut view = view();
    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the layer transforms");
    view.session.live.set_transform(session);
    view.transform_controls = true;
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    view.move_above = Some(cxx_qt_lib::QImage::default());
    view.move_above_origin = Some((1, 2));
    view.move_above_scale = 4;
    view.move_above_presented = Some((2, rect));
    view.move_drag_active = true;
    view.move_base_origin = Some((3, 4));
    let original = view.session.doc.as_ref().expect("document").layers[0].clone();
    view.mask_move_baseline = Some(("0".to_string(), original));

    let _ = view.cancel_active_sessions();

    assert!(!view.has_active_session(), "the gesture is gone");
    assert!(
        !view.transform_controls,
        "the session's controls flag goes with it"
    );
    assert!(view.move_above.is_none(), "the above-layer image is stale");
    assert!(view.move_above_origin.is_none(), "its origin is stale");
    assert_eq!(view.move_above_scale, 1, "the overlay scale is back");
    assert!(
        view.move_above_presented.is_none(),
        "the presented key named the outgoing view"
    );
    assert!(
        !view.move_drag_active,
        "a landing composite must not be told a drag owns the canvas"
    );
    assert!(
        view.move_base_origin.is_none(),
        "the base origin goes with the rest"
    );
    assert!(
        view.mask_move_baseline.is_none(),
        "the mask-move baseline named the outgoing layer"
    );
}

/// A session carries the pre-trim layer extent, so cancelling it restores the
/// extent a begin trimmed: a trimmed layer would clip later filters.
#[test]
fn a_cancelled_transform_restores_the_untrimmed_extent() {
    let mut view = view_with_trimmable_layer();
    let before = view.session.doc.as_ref().expect("document").layers[0].rect;
    let session = view
        .prepare_transform("0", TransformMode::Free)
        .expect("the layer transforms");
    let trimmed = view.session.doc.as_ref().expect("document").layers[0].rect;
    assert_ne!(trimmed, before, "the begin trimmed the layer");
    view.session.live.set_transform(session);

    let _ = view.cancel_active_sessions();

    assert!(!view.has_active_session(), "the session is gone");
    assert_eq!(
        view.session.doc.as_ref().expect("document").layers[0].rect,
        before,
        "the cancel restored the untrimmed extent"
    );
}

#[test]
fn returning_to_the_saved_state_clears_dirty() {
    let mut view = view();
    let mut target = view.layer_target.clone();
    assert!(view.session.record(&target, "First"));
    view.session.mark_saved();
    assert!(!view.session.dirty);

    assert!(view.session.record(&target, "Second"));
    assert!(view.session.dirty);
    let restored = view
        .session
        .restore(&mut target, History::undo_live, History::undo)
        .expect("a state to undo");
    let presentation = view.restore_presentation(restored, view.session.derived_dirty());
    present_core(&mut view, &presentation);
    assert!(!view.session.dirty, "undo lands on the saved state");
    let restored = view
        .session
        .restore(&mut target, History::redo_live, History::redo)
        .expect("a state to redo");
    let presentation = view.restore_presentation(restored, view.session.derived_dirty());
    present_core(&mut view, &presentation);
    assert!(view.session.dirty, "redo leaves the saved state");
}

#[test]
fn a_dropped_saved_state_stays_dirty() {
    let mut view = view();
    let mut target = view.layer_target.clone();
    assert!(view.session.record(&target, "Saved"));
    view.session.mark_saved();
    for i in 0..21 {
        assert!(view.session.record(&target, &format!("Edit {i}")));
    }
    assert!(view.session.dirty);
    assert_ne!(view.session.saved_id, view.session.history.current_id());

    while view.session.history.can_undo() {
        let restored = view
            .session
            .restore(&mut target, History::undo_live, History::undo)
            .expect("a state to undo");
        let presentation = view.restore_presentation(restored, view.session.derived_dirty());
        present_core(&mut view, &presentation);
        assert!(view.session.dirty, "the saved state is no longer retained");
    }
}

#[test]
fn purging_history_keeps_a_clean_document_clean() {
    let mut view = view();
    let target = view.layer_target.clone();
    assert!(view.session.record(&target, "Saved"));
    view.session.mark_saved();
    view.session.history.purge_states();
    assert_eq!(view.session.history.current_id(), view.session.saved_id);
    assert!(!view.session.dirty);
}

#[test]
fn one_constructor_installs_a_layer_stroke() {
    use crate::cxxqt_object::document_session::StrokePlan;
    use crate::cxxqt_object::impl_paint::tool_recording::ToolStrokeKind;

    let mut view = view();
    let cfg = StrokeConfig {
        diameter: 4,
        ..Default::default()
    };
    let began = view.begin_stroke(
        SessionKind::Paint,
        "Brush",
        &cfg,
        ToolStrokeKind::Paint,
        |rust| {
            let doc = rust.session.doc.as_ref()?;
            let stroke = pictura_paint::Stroke::begin_at(doc, "0", cfg).ok()?;
            Some(StrokePlan::layer("0", stroke))
        },
    );
    assert!(began);
    let session = view.session.live.stroke_session().expect("live stroke");
    assert_eq!(session.stroke_label, "Brush");
    assert_eq!(
        session.paint_pad,
        crate::cxxqt_object::helpers::paint_pad_for(
            view.session.doc.as_ref().expect("document"),
            "0"
        )
    );
    assert!(session.fade_capture.is_some());
    assert!(session.pending_present.is_none());
    assert!(!session.present_flush_due);
    assert!(session.stroke_tiles.bounds().is_none());
}

#[test]
fn one_constructor_installs_a_mask_stroke() {
    use crate::cxxqt_object::document_session::{MaskStroke, StrokePlan};
    use crate::cxxqt_object::impl_paint::tool_recording::ToolStrokeKind;

    let mut view = view();
    let cfg = StrokeConfig {
        diameter: 4,
        ..Default::default()
    };
    let mask_doc = Document::from_rgba("0", 8, 8, &[255u8; 8 * 8 * 4]);
    let began = view.begin_stroke(
        SessionKind::MaskPaint,
        "Eraser",
        &cfg,
        ToolStrokeKind::Mask,
        |_| {
            let stroke = pictura_paint::Stroke::begin_at(&mask_doc, "0", cfg).ok()?;
            Some(StrokePlan::mask(MaskStroke {
                path: "0".to_string(),
                baseline: None,
                rect: PsdRect {
                    top: 0,
                    left: 0,
                    bottom: 8,
                    right: 8,
                },
                document: mask_doc,
                stroke,
            }))
        },
    );
    assert!(began);
    let session = view.session.live.stroke_session().expect("live stroke");
    assert_eq!(session.stroke_label, "Eraser");
    assert!(session.mask_stroke.is_some());
    assert!(session.fade_capture.is_none());
    assert!(session.pending_present.is_none());
    assert!(!session.present_flush_due);
    assert!(session.stroke_tiles.bounds().is_none());
}

#[test]
fn a_refused_stroke_begin_leaves_the_live_state_untouched() {
    use crate::cxxqt_object::impl_paint::tool_recording::ToolStrokeKind;

    let mut view = view();
    assert!(view.begin_heal(20, 50), "the healing gesture starts");
    let cfg = StrokeConfig::default();
    let began = view.begin_stroke(
        SessionKind::Paint,
        "Brush",
        &cfg,
        ToolStrokeKind::Paint,
        |_| panic!("a refused begin must not build a stroke"),
    );
    assert!(!began);
    assert_eq!(view.active_session_kind(), Some(SessionKind::Healing));
}
