//! The edit transaction: one owner for committed document edits.
//!
//! A mutation declares what it changed; this module performs the compositing,
//! link invalidation, history capture, revision and dirty bookkeeping, and
//! reports what the QObject must present.

use super::qobject;
use super::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::PsdRect;

/// What a mutation changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    /// Nothing changed: no history, no compositing, no revision, no signal.
    None,
    /// Metadata or the selection changed: capture and notify, no compositing.
    Meta,
    /// A preview already presented the change: capture without a new signal.
    Record,
    /// Pixels changed inside these document rectangles.
    Regions { rects: Vec<PsdRect>, notify: bool },
    /// Pixels changed with no bound: one full recomposite.
    Full,
    /// The edit invalidates positional links: clear them and recomposite fully.
    Structural,
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
        Edit::Meta => Effect::Notify,
        Edit::Record => Effect::Nothing,
        Edit::Regions { notify, .. } => Effect::Blit { notify: *notify },
        Edit::Full | Edit::Structural => Effect::Rebuild,
    }
}

/// A committed edit and the region blit (if any) the QObject must emit.
pub(super) struct Applied {
    pub(super) edit: Edit,
    pub(super) blit: Option<(QImage, i32, i32)>,
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
        let blit = match &edit {
            Edit::Regions { rects, .. } => self.refresh_regions(rects),
            Edit::Full | Edit::Structural => {
                self.composite_full();
                None
            }
            Edit::None | Edit::Meta | Edit::Record => None,
        };
        if matches!(edit, Edit::Structural) {
            self.link_sets.clear();
        }
        if let Some(doc) = self.doc.as_mut() {
            self.history
                .capture_live(doc, &self.selection, &self.layer_target, label);
            self.dirty = true;
            if bump_revision {
                self.content_revision = self.content_revision.wrapping_add(1);
            }
        }
        Applied { edit, blit }
    }

    /// Present a live edit without capturing history.
    pub(super) fn preview_edit(&mut self, edit: &Edit) -> Option<(QImage, i32, i32)> {
        if matches!(edit, Edit::None) {
            return None;
        }
        let blit = match edit {
            Edit::Regions { rects, .. } => self.refresh_regions(rects),
            Edit::Full | Edit::Structural => {
                self.composite_full();
                None
            }
            Edit::None | Edit::Meta | Edit::Record => None,
        };
        self.content_revision = self.content_revision.wrapping_add(1);
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
            if rust.doc.is_none() {
                return Edit::None;
            }
            f(&mut rust)
        };
        let Applied { edit, blit } = self.as_mut().rust_mut().apply_edit(label, edit);
        if let Some((image, x, y)) = blit {
            if image.width() > 0 && image.height() > 0 {
                self.as_mut().region_blitted(image, x, y);
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
            if rust.doc.is_none() {
                return Edit::None;
            }
            f(&mut rust)
        };
        let Applied { edit, blit } = self.as_mut().rust_mut().apply_edit_move(label, edit);
        if let Some((image, x, y)) = blit {
            if image.width() > 0 && image.height() > 0 {
                self.as_mut().region_blitted(image, x, y);
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
            if rust.doc.is_none() {
                return Edit::None;
            }
            f(&mut rust)
        };
        let blit = self.as_mut().rust_mut().preview_edit(&edit);
        if let Some((image, x, y)) = blit {
            if image.width() > 0 && image.height() > 0 {
                self.as_mut().region_blitted(image, x, y);
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
            doc: Some(doc),
            ..Default::default()
        }
    }

    #[test]
    fn one_commit_refreshes_captures_and_versions() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Full);
        assert_eq!(applied.edit, Edit::Full);
        assert!(applied.blit.is_none());
        assert_eq!(view.history.count(), 1);
        assert!(view.dirty);
        assert_eq!(view.content_revision, 1);
        let doc = view.doc.as_ref().expect("document");
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
    fn an_unchanged_edit_records_nothing() {
        let mut view = view(doc_with_layer());
        let before = view.doc.as_ref().expect("document").composite.clone();
        let applied = view.apply_edit("Edit", Edit::None);
        assert_eq!(applied.edit, Edit::None);
        assert_eq!(view.history.count(), 0);
        assert!(!view.dirty);
        assert_eq!(view.content_revision, 0);
        assert_eq!(view.doc.as_ref().expect("document").composite, before);
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
        let before = view.doc.as_ref().expect("document").composite.clone();
        let applied = view.apply_edit("Edit", Edit::Meta);
        assert_eq!(effect_of(&applied.edit), Effect::Notify);
        assert_eq!(view.history.count(), 1);
        assert_eq!(view.doc.as_ref().expect("document").composite, before);
    }

    #[test]
    fn an_already_presented_edit_records_without_notifying() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit("Edit", Edit::Record);
        assert_eq!(effect_of(&applied.edit), Effect::Nothing);
        assert_eq!(view.history.count(), 1);
    }

    #[test]
    fn a_structural_outcome_clears_links_and_rebuilds() {
        let mut view = view(doc_with_layer());
        view.link_sets.insert("0".into(), 1);
        let applied = view.apply_edit("Edit", Edit::Structural);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert!(view.link_sets.is_empty());
        let doc = view.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
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
        let doc = view.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
        let restored = view.history.jump(0).expect("captured state");
        assert_eq!(
            restored.doc.composite, doc.composite,
            "history must carry the refreshed composite"
        );
    }

    #[test]
    fn a_geometry_commit_composites_after_the_engine_op() {
        let mut view = view(doc_with_layer());
        {
            let doc = view.doc.as_mut().expect("document");
            pictura_render::resize_document(doc, 8, 8, pictura_render::Resample::Nearest)
                .expect("resize");
        }
        view.apply_edit("Image Size", Edit::Full);
        let doc = view.doc.as_ref().expect("document");
        assert_eq!(doc.composite, pictura_render::composite_rgba(doc));
    }

    #[test]
    fn a_preview_records_nothing_and_its_commit_records_one() {
        let mut view = view(doc_with_layer());
        view.preview_edit(&Edit::Full);
        assert_eq!(view.history.count(), 0);
        assert_eq!(view.content_revision, 1);
        view.apply_edit("Edit", Edit::Meta);
        assert_eq!(view.history.count(), 1);
    }

    #[test]
    fn a_move_commit_keeps_the_content_revision() {
        let mut view = view(doc_with_layer());
        let applied = view.apply_edit_move("Move", Edit::Full);
        assert_eq!(effect_of(&applied.edit), Effect::Rebuild);
        assert_eq!(view.history.count(), 1);
        assert_eq!(view.content_revision, 0);
    }
}
