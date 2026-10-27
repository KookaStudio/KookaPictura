//! The edit transaction: one owner for committed document edits.
//!
//! A mutation declares what it changed; this module performs the compositing,
//! link invalidation, history capture, revision and dirty bookkeeping, and
//! reports what the QObject must present.

use super::metrics;
use super::qobject;
use super::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;
use std::time::Instant;

/// What a mutation changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    /// Nothing changed: no history, no compositing, no revision, no signal.
    None,
    /// Metadata or the selection changed: capture and notify, no compositing.
    Meta,
    /// The selection changed without a history state: notify, no compositing.
    SelectionPreview,
    /// A preview already presented the change: capture without a new signal.
    Record,
    /// Pixels changed inside these document rectangles.
    Regions { rects: Vec<PsdRect>, notify: bool },
    /// Pixels changed with no bound: one full recomposite.
    Full,
    /// The edit invalidates positional links: clear them and recomposite fully.
    Structural,
    /// A content-neutral layer or group insertion (see
    /// `pictura_render::insertion_is_content_neutral`): clear the positional
    /// links like [`Edit::Structural`], but the stored composite and the
    /// display already match the new tree, so nothing is composited.
    Inserted,
}

/// What the QObject must present after a commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Nothing,
    Notify,
    Blit { notify: bool },
    Rebuild,
}

/// How a declared edit is presented. Pure, so the policy is unit-tested.
pub(super) fn effect_of(edit: &Edit) -> Effect {
    match edit {
        Edit::None => Effect::Nothing,
        Edit::Meta | Edit::SelectionPreview | Edit::Inserted => Effect::Notify,
        Edit::Record => Effect::Nothing,
        Edit::Regions { notify, .. } => Effect::Blit { notify: *notify },
        Edit::Full | Edit::Structural => Effect::Rebuild,
    }
}

/// The outcome for a node just inserted at `path`: [`Edit::Inserted`] when the
/// insertion is content-neutral, [`Edit::Structural`] otherwise.
pub(super) fn insertion_edit(doc: &pictura_core::Document, path: &str) -> Edit {
    if pictura_render::insertion_is_content_neutral(doc, path) {
        Edit::Inserted
    } else {
        Edit::Structural
    }
}

/// A committed edit and the damaged rectangle (if any) the QObject must emit.
pub(super) struct Applied {
    pub(super) edit: Edit,
    pub(super) blit: Option<PsdRect>,
}

impl PictureViewRust {
    /// Execute the pixel half, then capture one state and bump the bookkeeping.
    pub(super) fn apply_edit(&mut self, label: &str, edit: Edit) -> Applied {
        self.apply_edit_with(label, edit, true)
    }

    /// As [`Self::apply_edit`] for a move of the topmost pixel layer, which
    /// leaves the cached move-preview base valid: no content-revision bump.
    pub(super) fn apply_edit_move(&mut self, label: &str, edit: Edit) -> Applied {
        self.apply_edit_with(label, edit, false)
    }

    fn apply_edit_with(&mut self, label: &str, edit: Edit, bump_revision: bool) -> Applied {
        if matches!(edit, Edit::None) {
            return Applied { edit, blit: None };
        }
        // A live gesture (stroke, preview, placer) leaves the stored composite
        // stale and needs the full rebuild to end it, so an insertion made
        // under one recomposites like any other tree edit.
        let edit = if matches!(edit, Edit::Inserted) && self.has_active_session() {
            Edit::Structural
        } else {
            edit
        };
        metrics::begin_edit(label);
        let blit = self.refresh_for(&edit);
        if matches!(edit, Edit::Structural | Edit::Inserted) {
            self.link_sets.clear();
            self.solo_snapshot = None;
        }
        let target = self.layer_target.clone();
        let t = Instant::now();
        if matches!(edit, Edit::SelectionPreview) {
            self.session.content_revision = self.session.content_revision.wrapping_add(1);
        } else {
            if bump_revision {
                self.session.record(&target, label);
            } else {
                self.session.record_move(&target, label);
            }
            metrics::record_phase("history_capture", t.elapsed());
        }
        metrics::end_edit();
        Applied { edit, blit }
    }

    /// The compositing half of an outcome, shared by commits and previews.
    fn refresh_for(&mut self, edit: &Edit) -> Option<PsdRect> {
        match edit {
            Edit::Regions { rects, .. } => {
                metrics::mark_composited();
                self.refresh_regions(rects)
            }
            Edit::Full | Edit::Structural => {
                metrics::mark_composited();
                self.composite_full();
                None
            }
            Edit::Inserted => {
                // The composite and the pyramid stay valid. A soloed mask's
                // source is found by path, which the insertion may have
                // shifted, so only that is re-resolved.
                if self.mask_solo.is_some() {
                    self.rebuild_mask_solo_source();
                    self.frame_revision = self.frame_revision.wrapping_add(1);
                }
                None
            }
            Edit::None | Edit::Meta | Edit::Record | Edit::SelectionPreview => None,
        }
    }

    /// Present a live edit without capturing history.
    pub(super) fn preview_edit(&mut self, edit: &Edit) -> Option<PsdRect> {
        if matches!(edit, Edit::None) {
            return None;
        }
        let blit = self.refresh_for(edit);
        self.session.content_revision = self.session.content_revision.wrapping_add(1);
        blit
    }
}

impl qobject::PictureView {
    /// Commit a document edit through the transaction and return its outcome.
    pub(super) fn edit(
        mut self: Pin<&mut Self>,
        label: &str,
        f: impl FnOnce(&mut PictureViewRust) -> Edit,
    ) -> Edit {
        let edit = {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return Edit::None;
            }
            // Open the record before the mutation, so its time is counted.
            metrics::begin_edit(label);
            f(&mut rust)
        };
        if matches!(edit, Edit::None) {
            metrics::cancel_edit();
        }
        let Applied { edit, blit } = self.as_mut().rust_mut().apply_edit(label, edit);
        if let Some(damaged) = blit {
            if damaged.width() > 0 && damaged.height() > 0 {
                self.as_mut().region_damaged(
                    damaged.left,
                    damaged.top,
                    damaged.width(),
                    damaged.height(),
                );
            }
        }
        if matches!(
            effect_of(&edit),
            Effect::Notify | Effect::Rebuild | Effect::Blit { notify: true }
        ) {
            self.changed();
        }
        edit
    }

    /// As [`Self::edit`] for a move of the topmost pixel layer.
    pub(super) fn commit_move_edit(
        mut self: Pin<&mut Self>,
        label: &str,
        f: impl FnOnce(&mut PictureViewRust) -> Edit,
    ) -> Edit {
        let edit = {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return Edit::None;
            }
            metrics::begin_edit(label);
            f(&mut rust)
        };
        if matches!(edit, Edit::None) {
            metrics::cancel_edit();
        }
        let Applied { edit, blit } = self.as_mut().rust_mut().apply_edit_move(label, edit);
        if let Some(damaged) = blit {
            if damaged.width() > 0 && damaged.height() > 0 {
                self.as_mut().region_damaged(
                    damaged.left,
                    damaged.top,
                    damaged.width(),
                    damaged.height(),
                );
            }
        }
        if matches!(
            effect_of(&edit),
            Effect::Notify | Effect::Rebuild | Effect::Blit { notify: true }
        ) {
            self.changed();
        }
        edit
    }

    /// Present a live edit without capturing history and return its outcome.
    pub(super) fn preview(
        mut self: Pin<&mut Self>,
        f: impl FnOnce(&mut PictureViewRust) -> Edit,
    ) -> Edit {
        let edit = {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return Edit::None;
            }
            f(&mut rust)
        };
        let blit = self.as_mut().rust_mut().preview_edit(&edit);
        if let Some(damaged) = blit {
            if damaged.width() > 0 && damaged.height() > 0 {
                self.as_mut().region_damaged(
                    damaged.left,
                    damaged.top,
                    damaged.width(),
                    damaged.height(),
                );
            }
        }
        if matches!(
            effect_of(&edit),
            Effect::Notify | Effect::Rebuild | Effect::Blit { notify: true }
        ) {
            self.changed();
        }
        edit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{BitDepth, ColorMode, Document, LayerStack, PixelBuffer};

    fn doc_with_layer() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 4, 4, (255, 0, 0))]);
        doc.composite = PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: vec![0u8; 4 * 4 * 4].into(),
        };
        doc
    }

    fn view(doc: Document) -> PictureViewRust {
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            ..Default::default()
        }
    }

    #[test]
    fn one_commit_refreshes_captures_and_versions() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Full);
        assert_eq!(applied.edit, Edit::Full);
        assert!(applied.blit.is_none());
        assert_eq!(view.session.history.count(), 1);
        assert!(view.session.dirty);
        assert_eq!(view.session.content_revision, 1);
        let doc = view.session.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
    }

    #[test]
    fn the_declared_outcome_is_returned() {
        let mut view = view(doc_with_layer());
        for edit in [Edit::None, Edit::Meta, Edit::Record, Edit::Full] {
            let applied = view.apply_edit("Edit", edit.clone());
            assert_eq!(applied.edit, edit);
        }
    }

    #[test]
    fn a_refused_mutation_records_nothing() {
        let mut view = view(doc_with_layer());
        let before = view
            .session
            .doc
            .as_ref()
            .expect("document")
            .composite
            .clone();
        let outcome = Edit::None;
        let applied = view.apply_edit("Refused", outcome);
        assert_eq!(applied.edit, Edit::None);
        assert_eq!(effect_of(&applied.edit), Effect::Nothing);
        assert_eq!(view.session.history.count(), 0);
        assert!(!view.session.dirty);
        assert_eq!(view.session.content_revision, 0);
        assert_eq!(
            view.session.doc.as_ref().expect("document").composite,
            before
        );
    }

    #[test]
    fn an_unchanged_edit_records_nothing() {
        let mut view = view(doc_with_layer());
        let before = view
            .session
            .doc
            .as_ref()
            .expect("document")
            .composite
            .clone();
        let applied = view.apply_edit("Edit", Edit::None);
        assert_eq!(applied.edit, Edit::None);
        assert_eq!(view.session.history.count(), 0);
        assert!(!view.session.dirty);
        assert_eq!(view.session.content_revision, 0);
        assert_eq!(
            view.session.doc.as_ref().expect("document").composite,
            before
        );
    }

    #[test]
    fn a_bounded_outcome_requests_one_region() {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        assert_eq!(
            effect_of(&Edit::Regions {
                rects: vec![rect],
                notify: false
            }),
            Effect::Blit { notify: false }
        );
    }

    #[test]
    fn a_multi_rect_outcome_requests_one_blit() {
        let a = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let b = PsdRect {
            top: 2,
            left: 2,
            bottom: 4,
            right: 4,
        };
        assert_eq!(
            effect_of(&Edit::Regions {
                rects: vec![a, b],
                notify: false
            }),
            Effect::Blit { notify: false }
        );
    }

    #[test]
    fn a_metadata_outcome_notifies_without_compositing() {
        let mut view = view(doc_with_layer());
        let before = view
            .session
            .doc
            .as_ref()
            .expect("document")
            .composite
            .clone();
        let applied = view.apply_edit("Edit", Edit::Meta);
        assert_eq!(effect_of(&applied.edit), Effect::Notify);
        assert_eq!(view.session.history.count(), 1);
        assert_eq!(
            view.session.doc.as_ref().expect("document").composite,
            before
        );
    }

    #[test]
    fn a_selection_preview_notifies_without_capturing_history() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Selection", Edit::SelectionPreview);
        assert_eq!(effect_of(&applied.edit), Effect::Notify);
        assert_eq!(view.session.history.count(), 0);
        assert!(!view.session.dirty);
        assert_eq!(view.session.content_revision, 1);
    }

    #[test]
    fn an_already_presented_edit_records_without_notifying() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Record);
        assert_eq!(effect_of(&applied.edit), Effect::Nothing);
        assert_eq!(view.session.history.count(), 1);
    }

    #[test]
    fn a_structural_outcome_clears_links_and_rebuilds() {
        let mut view = view(doc_with_layer());
        view.link_sets.insert("0".into(), 1);
        let applied = view.apply_edit("Edit", Edit::Structural);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert!(view.link_sets.is_empty());
        let doc = view.session.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
    }

    #[test]
    fn an_inserted_outcome_drops_links_without_compositing() {
        let mut view = view(doc_with_layer());
        view.link_sets.insert("0".into(), 1);
        let before = view
            .session
            .doc
            .as_ref()
            .expect("document")
            .composite
            .clone();
        let frame = view.frame_revision;
        let applied = view.apply_edit("New Layer", Edit::Inserted);
        assert_eq!(effect_of(&applied.edit), Effect::Notify);
        assert!(applied.blit.is_none());
        assert!(view.link_sets.is_empty());
        assert_eq!(view.session.history.count(), 1);
        assert_eq!(view.frame_revision, frame, "no display rebuild");
        assert_eq!(
            view.session.doc.as_ref().expect("document").composite,
            before,
            "no composite ran"
        );
    }

    #[test]
    fn a_full_outcome_rebuilds_once() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Full);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert!(applied.blit.is_none());
    }

    #[test]
    fn a_commit_captures_the_refreshed_composite() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Full);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        let doc = view.session.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
        let restored = view.session.history.jump(0).expect("captured state");
        assert_eq!(
            restored.doc.composite, doc.composite,
            "history must carry the refreshed composite"
        );
    }

    #[test]
    fn a_geometry_commit_composites_after_the_engine_op() {
        let mut view = view(doc_with_layer());
        {
            let doc = view.session.doc.as_mut().expect("document");
            pictura_render::resize_document(doc, 8, 8, pictura_render::Resample::Nearest)
                .expect("resize");
        }
        view.apply_edit("Image Size", Edit::Full);
        let doc = view.session.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
    }

    #[test]
    fn a_preview_records_nothing_and_its_commit_records_one() {
        let mut view = view(doc_with_layer());
        view.preview_edit(&Edit::Full);
        assert_eq!(view.session.history.count(), 0);
        assert_eq!(view.session.content_revision, 1);
        view.apply_edit("Edit", Edit::Meta);
        assert_eq!(view.session.history.count(), 1);
    }

    #[test]
    fn a_move_commit_keeps_the_content_revision() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit_move("Move", Edit::Full);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert_eq!(view.session.history.count(), 1);
        assert_eq!(view.session.content_revision, 0);
    }
}
