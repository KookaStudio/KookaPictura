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
    Regions {
        rects: Vec<PsdRect>,
        /// The document rectangles history may restrict its scan to. `None`
        /// means the full scan; an explicit hint is used by a paint commit
        /// whose refresh rectangles are only the still-pending region.
        hint: Option<Vec<PsdRect>>,
        notify: bool,
    },
    /// Pixels changed with no bound: one full recomposite.
    Full,
    /// The edit invalidates positional links. `damage` bounds the regional
    /// refresh; `None` means full.
    Structural { damage: Option<Vec<PsdRect>> },
    /// A content-neutral layer or group insertion (see
    /// `pictura_render::insertion_is_content_neutral`): clear the positional
    /// links like [`Edit::Structural`], but the stored composite and the
    /// display already match the new tree, so nothing is composited.
    Inserted,
}

impl Edit {
    /// A bounded region edit whose history hint is its refresh rectangles.
    pub(super) fn regions(rects: Vec<PsdRect>, notify: bool) -> Edit {
        let hint = (!rects.is_empty()).then(|| rects.clone());
        Edit::Regions {
            rects,
            hint,
            notify,
        }
    }

    /// A bounded region edit whose history hint is explicit. A paint commit
    /// uses this when its refresh rectangles are only the pending region but
    /// history must cover the whole stroke.
    pub(super) fn regions_hinted(rects: Vec<PsdRect>, hint: Vec<PsdRect>, notify: bool) -> Edit {
        Edit::Regions {
            rects,
            hint: (!hint.is_empty()).then_some(hint),
            notify,
        }
    }

    /// The rectangles history may restrict its scan to.
    pub(super) fn history_hint(&self) -> Option<&[PsdRect]> {
        match self {
            Edit::Regions { hint, .. } => hint.as_deref(),
            Edit::Structural { damage } => damage.as_deref(),
            _ => None,
        }
    }
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
        Edit::Regions {
            rects,
            hint,
            notify,
        } => {
            if rects.is_empty() && hint.is_none() {
                Effect::Rebuild
            } else {
                Effect::Blit { notify: *notify }
            }
        }
        Edit::Structural { damage } => {
            if damage.as_ref().is_some_and(|rects| !rects.is_empty()) {
                Effect::Blit { notify: true }
            } else {
                Effect::Rebuild
            }
        }
        Edit::Full => Effect::Rebuild,
    }
}

/// The outcome for a node just inserted at `path`: [`Edit::Inserted`] when the
/// insertion is content-neutral, a bounded structural edit otherwise.
pub(super) fn insertion_edit(doc: &pictura_core::Document, path: &str) -> Edit {
    if pictura_render::insertion_is_content_neutral(doc, path) {
        Edit::Inserted
    } else {
        Edit::Structural {
            damage: pictura_render::damage::influence(doc, path).map(|rect| vec![rect]),
        }
    }
}

fn merge_rects(rects: Vec<PsdRect>) -> Option<PsdRect> {
    let mut out: Option<PsdRect> = None;
    for rect in rects {
        out = Some(match out {
            None => rect,
            Some(current) => pictura_render::damage::union(Some(current), Some(rect))
                .expect("both rectangles are bounded"),
        });
    }
    out
}

fn merge_option(rects: Option<Vec<PsdRect>>) -> Option<PsdRect> {
    rects.and_then(merge_rects)
}

/// A two-sided structural edit: before and after reach of the same path (or
/// the source and the created copy).
pub(super) fn structural(before: Option<PsdRect>, after: Option<PsdRect>) -> Edit {
    Edit::Structural {
        damage: pictura_render::damage::union(before, after).map(|rect| vec![rect]),
    }
}

/// A structural edit whose affected paths exist only before the mutation.
pub(super) fn structural_before(damage: Option<Vec<PsdRect>>) -> Edit {
    Edit::Structural {
        damage: merge_option(damage).map(|rect| vec![rect]),
    }
}

/// A structural edit whose affected paths exist only after the mutation.
pub(super) fn structural_after(damage: Option<Vec<PsdRect>>) -> Edit {
    Edit::Structural {
        damage: merge_option(damage).map(|rect| vec![rect]),
    }
}

/// A two-sided structural edit over several paths.
pub(super) fn structural_paths(before: Option<Vec<PsdRect>>, after: Option<Vec<PsdRect>>) -> Edit {
    Edit::Structural {
        damage: pictura_render::damage::union(merge_option(before), merge_option(after))
            .map(|rect| vec![rect]),
    }
}

/// A bounded region edit whose reach is the union of the affected paths before
/// and after the mutation. Falls back to full when either side is unbounded.
pub(super) fn regions_influence(
    before: Option<Vec<PsdRect>>,
    after: Option<Vec<PsdRect>>,
    notify: bool,
) -> Edit {
    match pictura_render::damage::union(merge_option(before), merge_option(after)) {
        Some(rect) => Edit::regions(vec![rect], notify),
        None => Edit::Full,
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
        let pending = self.pending_previews.any();
        let force_full = self.has_active_session() || pending || self.mask_solo.is_some();
        let edit = match edit {
            // A live gesture leaves the stored composite stale and needs the
            // full rebuild to end it.
            Edit::Inserted if self.has_active_session() => Edit::Structural { damage: None },
            // A pending preview or a soloed mask owns display state the
            // regional path does not repair.
            Edit::Structural { .. } if force_full => Edit::Structural { damage: None },
            // An empty or canvas-sized bound is not worth the regional path;
            // it also means no history hint.
            Edit::Structural {
                damage: Some(rects),
            } if rects.is_empty() => Edit::Structural { damage: None },
            Edit::Structural {
                damage: Some(rects),
            } if self.structural_is_full(&rects) => Edit::Structural { damage: None },
            other => other,
        };
        let hint = if pending {
            None
        } else {
            edit.history_hint().map(|rects| rects.to_vec())
        };
        metrics::begin_edit(label);
        let blit = self.refresh_for(&edit);
        if matches!(edit, Edit::Structural { .. } | Edit::Inserted) {
            self.link_sets.clear();
            self.solo_snapshot = None;
        }
        let target = self.layer_target.clone();
        let t = Instant::now();
        if matches!(edit, Edit::SelectionPreview) {
            self.session.content_revision = self.session.content_revision.wrapping_add(1);
        } else {
            if bump_revision {
                self.session.record_hinted(&target, label, hint.as_deref());
            } else {
                self.session
                    .record_move_with(&target, label, hint.as_deref());
            }
            metrics::record_phase("history_capture", t.elapsed());
        }
        metrics::end_edit();
        Applied { edit, blit }
    }

    /// Whether a structural damage bound is large enough that the regional
    /// path is not worth its per-rectangle overhead.
    fn structural_is_full(&self, rects: &[PsdRect]) -> bool {
        let Some(doc) = self.session.doc.as_ref() else {
            return true;
        };
        let canvas = u64::from(doc.width) * u64::from(doc.height);
        if canvas == 0 {
            return true;
        }
        let area: u64 = rects
            .iter()
            .map(|rect| {
                u64::from(rect.width().max(0) as u32) * u64::from(rect.height().max(0) as u32)
            })
            .sum();
        area.saturating_mul(4) >= canvas.saturating_mul(3)
    }

    /// The compositing half of an outcome, shared by commits and previews.
    fn refresh_for(&mut self, edit: &Edit) -> Option<PsdRect> {
        match edit {
            Edit::Regions { rects, hint, .. } => {
                if rects.is_empty() {
                    if hint.is_none() {
                        metrics::mark_composited();
                        self.composite_full();
                    }
                    return None;
                }
                metrics::mark_composited();
                self.refresh_regions(rects)
            }
            Edit::Structural {
                damage: Some(rects),
            } if !rects.is_empty() => {
                metrics::mark_composited();
                if self.structural_is_full(rects) {
                    self.composite_full();
                    None
                } else {
                    self.refresh_regions(rects)
                }
            }
            Edit::Full | Edit::Structural { .. } => {
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
    use pictura_core::{BitDepth, ColorMode, Document, LayerStack};

    fn doc_with_layer() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![pixel_layer("base", 4, 4, (255, 0, 0))]);
        doc.composite = pictura_render::composite_rgba(&doc);
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
            effect_of(&Edit::regions(vec![rect], false)),
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
            effect_of(&Edit::regions(vec![a, b], false)),
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
    fn an_unbounded_structural_outcome_rebuilds() {
        let mut view = view(doc_with_layer());
        view.link_sets.insert("0".into(), 1);
        let applied = view.apply_edit("Edit", Edit::Structural { damage: None });
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert!(view.link_sets.is_empty());
        let doc = view.session.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
    }

    #[test]
    fn a_bounded_structural_outcome_clears_links_and_uses_a_region() {
        let mut view = view(doc_with_layer());
        view.link_sets.insert("0".into(), 1);
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let applied = view.apply_edit(
            "Edit",
            Edit::Structural {
                damage: Some(vec![rect]),
            },
        );
        assert_eq!(effect_of(&applied.edit), Effect::Blit { notify: true });
        assert_eq!(applied.blit, Some(rect));
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

    #[test]
    fn a_bounded_edit_hints_its_refresh_rects() {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let edit = Edit::regions(vec![rect], false);
        assert_eq!(edit.history_hint(), Some(&[rect][..]));
        let edit = Edit::Structural {
            damage: Some(vec![rect]),
        };
        assert_eq!(edit.history_hint(), Some(&[rect][..]));
    }

    #[test]
    fn empty_hint_is_no_hint() {
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        assert_eq!(Edit::regions(Vec::new(), false).history_hint(), None);
        assert_eq!(
            Edit::regions_hinted(Vec::new(), vec![rect], false).history_hint(),
            Some(&[rect][..])
        );
    }

    #[test]
    fn move_commit_hints_its_refresh_rects() {
        let mut view = view(doc_with_layer());
        let rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let applied = view.apply_edit_move("Move", Edit::regions(vec![rect], false));
        assert_eq!(applied.blit, Some(rect));
        assert_eq!(view.session.history.count(), 1);
    }
}
