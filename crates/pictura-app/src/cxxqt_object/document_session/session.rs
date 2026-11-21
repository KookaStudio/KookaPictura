//! The document session: one live session, one cancel path, one owner of the
//! document's state.
//!
//! The session is plain Rust. It holds the document, the selection, and the
//! history, and it reports what the Qt adapter must present; nothing here
//! knows about Qt.

use super::super::fade::FadeState;
use super::super::paths::ActivePath;
use super::super::presentation::{Presentation, Rebuild};
use super::super::state::PictureViewRust;
use super::active::{ActiveSession, PathRestore, SessionKind};
use super::transform::{RepeatTransform, TransformSession};
use crate::history::{History, Restored, Snapshot};
use crate::layer_target::LayerTarget;
use pictura_core::{Document, PsdRect};
use pictura_select::Selection;
use std::collections::HashMap;
use std::sync::Arc;

/// The selection state captured at the start of a Quick Selection stroke.
pub(in crate::cxxqt_object) struct QuickSelectionSnapshot {
    pub(in crate::cxxqt_object) creates_selection: bool,
    pub(in crate::cxxqt_object) selection: Option<Selection>,
    pub(in crate::cxxqt_object) deselected: Option<Selection>,
    /// The reselect memory the stroke's first dab recorded, kept for the rest
    /// of the stroke so later dabs do not clear what the first one replaced.
    pub(in crate::cxxqt_object) reselect: Option<Option<Selection>>,
}

/// The document state the session owns: the document itself, the selection,
/// and the history that records its edits.
///
/// The session is the only owner, so a replacement swaps the whole group and
/// a headless test can drive it without a QObject.
#[derive(Default)]
pub(in crate::cxxqt_object) struct DocumentSession {
    pub(in crate::cxxqt_object) quick_mask: crate::cxxqt_object::quick_mask::QuickMaskState,
    /// The live document, `None` without one.
    pub(in crate::cxxqt_object) doc: SessionDocument,
    pub(in crate::cxxqt_object) compatibility:
        crate::cxxqt_object::open_jobs::compatibility::NativeCompatibility,
    pub(in crate::cxxqt_object) history_compatibility: Vec<(
        u64,
        crate::cxxqt_object::open_jobs::compatibility::NativeCompatibility,
    )>,
    pub(in crate::cxxqt_object) skipped_xmp_mirror: bool,
    pub(in crate::cxxqt_object) recheck_compatibility_on_restore: bool,
    /// The live selection.
    pub(in crate::cxxqt_object) selection: Option<Selection>,
    /// Bumped whenever the selection is written. The committed-outline cache
    /// keys on it; a history restore counts as a write.
    pub(in crate::cxxqt_object) selection_revision: u64,
    /// Bumped only when a new selection replaces the old one, so the View menu
    /// can re-show Selection Edges without reacting to in-place mask edits.
    pub(in crate::cxxqt_object) selection_generation: u64,
    /// CS6 "last deselected" memory: the selection replaced by Deselect or a
    /// New-mode commit. Restored by `reselect`.
    pub(in crate::cxxqt_object) deselected_selection: Option<Selection>,
    /// The selection state at the start of a Quick Selection stroke, restored
    /// when the stroke is cancelled.
    pub(in crate::cxxqt_object) quick_selection_snapshot: Option<QuickSelectionSnapshot>,
    /// The recorded states.
    pub(in crate::cxxqt_object) history: History,
    pub(in crate::cxxqt_object) history_log: crate::history_log::DocumentLog,
    pub(in crate::cxxqt_object) history_log_detail: Option<String>,
    pub(in crate::cxxqt_object) history_log_save_warning: Option<&'static str>,
    /// The last destructive effect's pre-effect and post-effect pixels, for
    /// `Edit > Fade`. Cleared by any later edit or history restore.
    pub(in crate::cxxqt_object) fade: Option<FadeState>,
    /// The last committed similarity transform, for `Edit > Transform > Again`.
    /// Projective and Warp commits clear it (they are not repeatable this way).
    pub(in crate::cxxqt_object) last_transform: Option<RepeatTransform>,
    pub(in crate::cxxqt_object) link_sets: HashMap<String, u32>,
    /// Link sets a structural command remapped instead of clearing. The edit
    /// transaction installs them; `None` clears the links as before.
    pub(in crate::cxxqt_object) link_remap: Option<HashMap<String, u32>>,
    /// Every layer path's own visibility when the current solo began. The
    /// snapshot belongs to this document and is dropped by a structural edit.
    pub(in crate::cxxqt_object) solo_snapshot: Option<HashMap<String, bool>>,
    /// The panel previews that are applied but not committed yet.
    pub(in crate::cxxqt_object) pending_previews: PendingPreviews,
    /// Whether the document has unsaved changes.
    pub(in crate::cxxqt_object) dirty: bool,
    /// The history identity that was current at the last successful save, open,
    /// install, or Revert. `None` means the document has no saved point.
    pub(in crate::cxxqt_object) saved_id: Option<u64>,
    pub(in crate::cxxqt_object) persisted_metadata_revision: u64,
    pub(in crate::cxxqt_object) saved_metadata_revision: u64,
    pub(in crate::cxxqt_object) save_publication_uncertain: bool,
    pub(in crate::cxxqt_object) recovery:
        Option<crate::cxxqt_object::impl_core::recovery::RecoveryAttachment>,
    /// Bumped whenever the document's content changed.
    pub(in crate::cxxqt_object) content_revision: u64,
    /// Bumped whenever the view loads or replaces its document, so a panel can
    /// drop a gesture whose target no longer belongs to the current document.
    pub(in crate::cxxqt_object) document_generation: u64,
    /// The document's one live edit session: the slot that owns every
    /// in-progress gesture, so two sessions cannot exist at once.
    pub(in crate::cxxqt_object) live: ActiveSession,
    /// The canonical `.joboptions` text this document was last saved to PDF
    /// with, so Save on the `.pdf` path reopens the dialog with it.
    pub(in crate::cxxqt_object) pdf_options: Option<String>,
    /// The password a protected PDF was opened with, kept in memory only so
    /// Revert can reopen it.
    pub(in crate::cxxqt_object) pdf_password: Option<String>,
    pub(in crate::cxxqt_object) print_resources:
        Arc<pictura_codec::print_resources::PrintResourceEdits>,
    pub(in crate::cxxqt_object) print_revision: u64,
    pub(in crate::cxxqt_object) saved_print_revision: u64,
    /// The Vanishing Point planes this document's last session kept, so the
    /// dialog reopens on them: read from the file when the document is
    /// installed and written into it by a save. Kept out of the document so
    /// undo leaves them alone, as no history state records them.
    pub(in crate::cxxqt_object) vanishing_planes: Option<pictura_filters::VpPlanes>,
    /// Encoded PSD channel streams for this document session. Shared with the
    /// save and recovery workers; replaced when a document is installed.
    pub(in crate::cxxqt_object) encode_cache: std::sync::Arc<pictura_codec::EncodeCache>,
}

/// The installed document remains readable during compatibility viewing.
/// Edit paths share this boundary for in-place access and replacement.
#[derive(Debug, Default)]
pub(in crate::cxxqt_object) struct SessionDocument {
    value: Option<Document>,
    read_only: bool,
}

impl SessionDocument {
    pub(in crate::cxxqt_object) fn as_ref(&self) -> Option<&Document> {
        self.value.as_ref()
    }

    pub(in crate::cxxqt_object) fn as_mut(&mut self) -> Option<&mut Document> {
        if self.read_only {
            return None;
        }
        self.value.as_mut()
    }

    pub(in crate::cxxqt_object) fn is_some(&self) -> bool {
        self.value.is_some()
    }

    pub(in crate::cxxqt_object) fn is_none(&self) -> bool {
        self.value.is_none()
    }

    pub(in crate::cxxqt_object) fn snapshot(&self) -> Option<Document> {
        if self.read_only {
            return None;
        }
        self.value.clone()
    }

    pub(in crate::cxxqt_object) fn set(&mut self, value: Option<Document>) {
        if !self.read_only {
            self.value = value;
        }
    }

    pub(in crate::cxxqt_object) fn replace(&mut self, doc: Document) -> Option<Document> {
        if self.read_only {
            return None;
        }
        self.value.replace(doc)
    }

    #[cfg(test)]
    pub(in crate::cxxqt_object) fn take(&mut self) -> Option<Document> {
        if self.read_only {
            return None;
        }
        self.value.take()
    }
}

#[cfg(test)]
impl PartialEq<Option<Document>> for SessionDocument {
    fn eq(&self, other: &Option<Document>) -> bool {
        &self.value == other
    }
}

impl DocumentSession {
    pub(in crate::cxxqt_object) fn read_only(&self) -> bool {
        self.compatibility.access.is_read_only()
    }

    pub(in crate::cxxqt_object) fn set_compatibility(
        &mut self,
        state: crate::cxxqt_object::open_jobs::compatibility::NativeCompatibility,
    ) {
        self.compatibility.deactivate();
        state.activate();
        self.doc.read_only = state.access.is_read_only();
        self.compatibility = state;
    }

    /// A session with `doc` installed and an empty history.
    #[cfg(test)]
    pub(in crate::cxxqt_object) fn with_document(doc: Document) -> Self {
        Self {
            doc: SessionDocument {
                value: Some(doc),
                read_only: false,
            },
            ..Default::default()
        }
    }

    /// A no-op live session for the transaction tests: it exercises the
    /// full-rebuild fallback without starting a real gesture.
    #[cfg(test)]
    pub(in crate::cxxqt_object) fn force_test_session(&mut self) {
        self.live = ActiveSession::ModePreview(Box::new(Document::new(
            1,
            1,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        )));
    }

    /// Replace the live selection and bump its revision.
    pub(in crate::cxxqt_object) fn set_selection(&mut self, selection: Option<Selection>) {
        if selection.is_some() {
            self.mark_new_selection();
        }
        self.set_selection_in_place(selection);
    }

    pub(in crate::cxxqt_object) fn mark_new_selection(&mut self) {
        if !self.quick_mask.active {
            self.selection_generation = self.selection_generation.wrapping_add(1);
        }
    }

    /// Geometry changes invalidate both the editing clip and temporary mask coverage.
    pub(in crate::cxxqt_object) fn clear_geometry_selection(&mut self) {
        self.set_selection_in_place(None);
        self.quick_mask.coverage = None;
    }

    /// Replace the live selection without calling it a new selection. Derived
    /// masks, moves, history restores and Quick Mask transitions use this so the
    /// View menu keeps hidden Selection Edges hidden.
    pub(in crate::cxxqt_object) fn set_selection_in_place(&mut self, selection: Option<Selection>) {
        self.selection = selection;
        self.touch_selection();
    }

    /// Take the live selection, bumping its revision when there was one.
    pub(in crate::cxxqt_object) fn take_selection(&mut self) -> Option<Selection> {
        let selection = self.selection.take();
        if selection.is_some() {
            self.touch_selection();
        }
        selection
    }

    /// Mark the selection as written without replacing it. Used for an
    /// in-place mask edit and after a history step rewrites the selection.
    pub(in crate::cxxqt_object) fn touch_selection(&mut self) {
        self.selection_revision = self.selection_revision.wrapping_add(1);
    }

    /// Install a replacement document, its history, and its PDF password.
    ///
    /// The document generation is bumped, so a panel can drop a gesture whose
    /// target no longer belongs to the current document.
    pub(in crate::cxxqt_object) fn install(
        &mut self,
        doc: Option<Document>,
        history: History,
        pdf_password: Option<String>,
    ) {
        self.set_compatibility(Default::default());
        self.doc.set(doc);
        self.history = history;
        self.history_log = Default::default();
        self.history_log_detail = None;
        self.history_log_save_warning = None;
        self.pdf_password = pdf_password;
        self.print_resources = Default::default();
        self.print_revision = 0;
        self.saved_print_revision = 0;
        self.encode_cache = std::sync::Arc::new(pictura_codec::EncodeCache::new());
        self.document_generation = self.document_generation.wrapping_add(1);
        self.saved_id = self.history.current_id();
        self.refresh_dirty();
    }

    /// Clear the edit state that names or belongs to the outgoing document.
    /// Both a document replacement and a named-snapshot restore call it, so a
    /// stale Fade or link set can never survive either path.
    pub(in crate::cxxqt_object) fn clear_edit_state(&mut self) {
        self.fade = None;
        self.last_transform = None;
        self.link_sets.clear();
        self.link_remap = None;
        self.solo_snapshot = None;
        self.pending_previews = PendingPreviews::default();
    }

    /// Restore a named snapshot: replace the document, selection, and Quick
    /// Mask, clear the edit state, and record the restore under `label`. The
    /// returned target is the incoming one retained against the restored
    /// document.
    pub(in crate::cxxqt_object) fn restore_named_snapshot(
        &mut self,
        snapshot: Snapshot,
        mut target: LayerTarget,
        label: &str,
    ) -> LayerTarget {
        self.doc.set(Some(snapshot.doc));
        if let Some(doc) = self.doc.as_mut() {
            pictura_render::prune_layer_comp_warnings(doc);
        }
        if let Some(doc) = self.doc.as_ref() {
            target.retain_resolving(doc);
        }
        self.set_selection_in_place(snapshot.selection);
        self.restore_quick_mask_selection(snapshot.quick_mask);
        self.deselected_selection = None;
        self.clear_edit_state();
        self.record(&target, label);
        self.document_generation = self.document_generation.wrapping_add(1);
        target
    }

    /// Capture the live document as a history state under `label`. False
    /// without a document.
    pub(in crate::cxxqt_object) fn record(&mut self, target: &LayerTarget, label: &str) -> bool {
        self.record_with(target, label, true, None)
    }

    fn record_with(
        &mut self,
        target: &LayerTarget,
        label: &str,
        bump_revision: bool,
        hint: Option<&[PsdRect]>,
    ) -> bool {
        self.capture_with(target, label, bump_revision, hint, true)
    }

    fn capture_with(
        &mut self,
        target: &LayerTarget,
        label: &str,
        bump_revision: bool,
        hint: Option<&[PsdRect]>,
        bookkeeping: bool,
    ) -> bool {
        let Some(doc) = self.doc.as_mut() else {
            return false;
        };
        self.history
            .capture_live_with(doc, &self.selection, target, label, hint);
        if let Some((marker, value)) = self.history_log.take_marker() {
            self.history
                .set_history_marker(marker, value.map(std::sync::Arc::<str>::from));
        }
        self.history_log
            .edit(label, self.history_log_detail.take().as_deref());
        self.history.set_current_quick_mask(
            self.quick_mask
                .active
                .then(|| self.quick_mask.coverage.clone()),
        );
        if bookkeeping {
            self.dirty = true;
            if bump_revision {
                self.content_revision = self.content_revision.wrapping_add(1);
            }
        }
        self.capture_recovery(false);
        true
    }

    /// Capture the live document without writing dirty or the content revision:
    /// the presentation owns both.
    pub(in crate::cxxqt_object) fn capture_live_without_bookkeeping(
        &mut self,
        target: &LayerTarget,
        label: &str,
        hint: Option<&[PsdRect]>,
    ) -> bool {
        self.capture_with(target, label, true, hint, false)
    }

    /// As [`Self::capture_live_without_bookkeeping`] for a move commit, which
    /// does not advance the content revision.
    pub(in crate::cxxqt_object) fn capture_move_without_bookkeeping(
        &mut self,
        target: &LayerTarget,
        label: &str,
        hint: Option<&[PsdRect]>,
    ) -> bool {
        self.capture_with(target, label, false, hint, false)
    }

    /// Record the current history state as the saved one and clear `dirty`.
    pub(in crate::cxxqt_object) fn mark_saved(&mut self) {
        self.saved_id = self.history.current_id();
        self.saved_metadata_revision = self.persisted_metadata_revision;
        self.dirty =
            self.save_publication_uncertain || self.print_revision != self.saved_print_revision;
    }

    pub(in crate::cxxqt_object) fn stage_print_resources(
        &mut self,
        resources: pictura_codec::print_resources::PrintResourceEdits,
    ) {
        self.print_resources = Arc::new(resources);
        self.print_revision = self.print_revision.wrapping_add(1);
        self.persisted_metadata_changed();
    }

    pub(in crate::cxxqt_object) fn persisted_metadata_changed(&mut self) {
        self.persisted_metadata_revision = self.persisted_metadata_revision.wrapping_add(1);
        self.dirty = true;
        self.capture_recovery(false);
    }

    pub(in crate::cxxqt_object) fn capture_recovery(&self, checkpoint: bool) {
        let Some(recovery) = &self.recovery else {
            return;
        };
        let document = self
            .history
            .current_snapshot()
            .map(|snapshot| &snapshot.doc)
            .or(self.doc.as_ref());
        if let Some(document) = document {
            recovery.capture(
                document,
                self.vanishing_planes.as_ref(),
                &self.print_resources,
                &self.encode_cache,
                crate::cxxqt_object::impl_core::recovery::RecoveryIdentity {
                    history: self.history.current_id(),
                    metadata: self.persisted_metadata_revision,
                    generation: self.document_generation,
                },
                checkpoint,
            );
        }
    }

    /// Forget the saved point and mark the document modified, so a recovered
    /// untitled document cannot become clean by undoing back to its start.
    pub(in crate::cxxqt_object) fn mark_unsaved(&mut self) {
        self.saved_id = None;
        self.dirty = true;
    }

    /// Derive `dirty` from the saved identity after history navigation.
    fn refresh_dirty(&mut self) {
        self.dirty = self.derived_dirty();
    }

    /// The dirty value the saved identity implies, without writing it.
    pub(in crate::cxxqt_object) fn derived_dirty(&self) -> bool {
        match (self.doc.as_ref(), self.saved_id) {
            (None, _) => false,
            (Some(_), Some(saved)) => {
                self.history.current_id() != Some(saved)
                    || self.print_revision != self.saved_print_revision
                    || self.persisted_metadata_revision != self.saved_metadata_revision
                    || self.save_publication_uncertain
            }
            (Some(_), None) => true,
        }
    }

    pub(in crate::cxxqt_object) fn restore_quick_mask_selection(
        &mut self,
        coverage: Option<Option<Selection>>,
    ) {
        if self.quick_mask.active {
            self.quick_mask.coverage = match coverage {
                Some(coverage) => coverage,
                None => self.selection.take(),
            };
        } else if let Some(coverage) = coverage {
            self.selection = coverage;
        }
    }

    /// Step the history into the live document and selection, and report where
    /// the picture changed.
    pub(in crate::cxxqt_object) fn restore(
        &mut self,
        target: &mut LayerTarget,
        step: impl FnOnce(
            &mut History,
            &mut Document,
            &mut Option<Selection>,
            &mut LayerTarget,
        ) -> Option<Restored>,
        legacy: impl FnOnce(&mut History) -> Option<Snapshot>,
    ) -> Option<Restored> {
        let was_read_only = self.doc.read_only;
        if was_read_only && !self.recheck_compatibility_on_restore {
            return None;
        }
        let previous_marker = self.history.history_marker().0;
        self.doc.read_only = false;
        let restored = match self.doc.as_mut() {
            Some(doc) => step(&mut self.history, doc, &mut self.selection, target),
            None => legacy(&mut self.history).map(|snapshot| {
                self.doc.set(Some(snapshot.doc));
                self.selection = snapshot.selection;
                Restored::Everywhere
            }),
        };
        self.doc.read_only = was_read_only;
        let restored = restored?;
        let (marker, value) = self.history.history_marker();
        if marker != previous_marker {
            if let Some(value) = value {
                self.history_log
                    .restore_history(value.map(|value| value.to_string()));
            }
        }
        let quick_mask = self
            .history
            .current_snapshot()
            .and_then(|s| s.quick_mask.clone());
        self.restore_quick_mask_selection(quick_mask);
        self.touch_selection();
        self.capture_recovery(false);
        Some(restored)
    }
}

/// The panel previews that are applied but not committed yet.
#[derive(Default)]
pub(in crate::cxxqt_object) struct PendingPreviews {
    /// A layer-opacity preview changed the document.
    pub(in crate::cxxqt_object) opacity_changed: bool,
    /// A layer-fill preview changed the document.
    pub(in crate::cxxqt_object) fill_changed: bool,
    /// The history label of the mask Density/Feather control whose preview
    /// changed the document.
    pub(in crate::cxxqt_object) mask_refine: Option<&'static str>,
}

impl PendingPreviews {
    /// Whether any uncommitted panel preview has changed the document.
    pub(in crate::cxxqt_object) fn any(&self) -> bool {
        self.opacity_changed || self.fill_changed || self.mask_refine.is_some()
    }
}

impl PictureViewRust {
    /// The kind of live session, read from the slot.
    pub(in crate::cxxqt_object) fn active_session_kind(&self) -> Option<SessionKind> {
        self.session.live.kind()
    }

    /// Whether any live gesture owns the document.
    pub(in crate::cxxqt_object) fn has_active_session(&self) -> bool {
        self.active_session_kind().is_some()
    }

    /// Whether a paint or mask stroke is writing the document in place.
    ///
    /// A layer stroke patches the composite only where it has presented, and a
    /// mask stroke never patches it, so the composite lags the pixels.
    pub(in crate::cxxqt_object) fn is_painting(&self) -> bool {
        self.session.live.stroke().is_some()
            || self.session.live.mask_stroke().is_some()
            || self.quick_mask_stroke_active()
    }

    /// Whether a lifted transform owns the document.
    ///
    /// The lift rewrote the layer stack and the session does not recomposite,
    /// so the cached composite predates the lift until the session ends.
    pub(in crate::cxxqt_object) fn has_lifted_transform(&self) -> bool {
        self.lifted_transform_is_live()
    }

    /// Whether the live transform carries a lifted selection, ignoring whether
    /// the Move tool's controls box owns it.
    fn lifted_transform_is_live(&self) -> bool {
        self.session
            .live
            .transform()
            .is_some_and(|session| session.lifted.is_some())
    }

    /// Whether a **lifted** transform is pending. A non-lifted transform can
    /// accept a document edit: the edit lands in the layer and the transform
    /// commit keeps it. A lifted selection restores its pre-lift snapshot on
    /// cancel/commit, so an edit made under it would be discarded. A Move
    /// controls gesture does not lock while it is in progress (`transform_controls`
    /// is true); once it is promoted to pending it locks like a modal transform.
    pub(in crate::cxxqt_object) fn modal_transform_blocks_edit(&self) -> bool {
        self.lifted_transform_is_live() && !self.transform_controls
    }

    /// Fill and Stroke refuse any pending transform, lifted or not, matching
    /// the Edit menu lock. A Move controls gesture in progress is not a pending
    /// transform; a released changed gesture is.
    pub(in crate::cxxqt_object) fn modal_transform_blocks_fill(&self) -> bool {
        self.session.live.transform().is_some() && !self.transform_controls
    }

    /// Whether a begin of `kind` may take the session slot.
    ///
    /// The slot holds one session. A begin of another kind is refused. A repeat
    /// begin of the same kind follows that kind's rule: a transform replaces its
    /// own session, a filter or mode preview updates it, and a lasso attaches
    /// to the magnetic gesture it started. A healing, selection-move, path, or
    /// paint begin is refused.
    pub(in crate::cxxqt_object) fn slot_allows(&self, kind: SessionKind) -> bool {
        !self.session.read_only() && self.session.live.allows(kind)
    }

    /// Cancel the live session against the current document without recording
    /// history. The returned change is what the adapter must present.
    ///
    /// The slot holds one session, so the cancel matches it: the mode preview's
    /// whole-document restore no longer has to run before the in-place
    /// cancels, because only one of them can be live.
    pub(in crate::cxxqt_object) fn cancel_active_sessions(&mut self) -> Presentation {
        self.cancel_active_sessions_with(true)
    }

    /// As [`Self::cancel_active_sessions`], but skip the display work. Callers
    /// that replace the document and discard the presentation use this so the
    /// cancelled session does not composite or refresh pixels nobody reads.
    pub(in crate::cxxqt_object) fn cancel_active_sessions_report_only(&mut self) -> Presentation {
        self.cancel_active_sessions_with(false)
    }

    fn cancel_active_sessions_with(&mut self, refresh_display: bool) -> Presentation {
        if !self.has_active_session() {
            self.clear_pending_previews();
            return Presentation::none();
        }
        let presentation = match std::mem::replace(&mut self.session.live, ActiveSession::Idle) {
            ActiveSession::Idle => Presentation::none(),
            ActiveSession::ModePreview(base) => {
                self.session.doc.set(Some(*base));
                if refresh_display {
                    self.composite_into_doc();
                }
                Presentation::rebuild(Rebuild::Sync)
            }
            ActiveSession::FilterPreview(preview) => {
                match super::super::impl_filters::cancel_filter_preview_session(self, *preview) {
                    Some(Some(rect)) => {
                        let regions = if refresh_display {
                            self.refresh_regions(&[rect]).into_iter().collect()
                        } else {
                            vec![rect]
                        };
                        Presentation::regions(regions, false)
                    }
                    Some(None) => {
                        if refresh_display {
                            self.composite_into_doc();
                        }
                        Presentation::rebuild(Rebuild::Sync)
                    }
                    None => Presentation::none(),
                }
            }
            ActiveSession::Stroke(session) => match self.cancel_stroke_session(*session) {
                Some(rect) => {
                    let regions = if refresh_display {
                        self.refresh_regions(&[rect]).into_iter().collect()
                    } else {
                        vec![rect]
                    };
                    Presentation::regions(regions, false)
                }
                None => {
                    if refresh_display {
                        self.composite_into_doc();
                    }
                    Presentation::rebuild(Rebuild::Sync)
                }
            },
            ActiveSession::Transform(session) => {
                if self.cancel_transform_session(*session) {
                    if refresh_display {
                        self.composite_into_doc();
                    }
                    Presentation::rebuild(Rebuild::Sync)
                } else {
                    Presentation::none()
                }
            }
            ActiveSession::Path(restore) => {
                self.cancel_path_gesture_restore(*restore, refresh_display)
            }
            ActiveSession::Heal(_) => Presentation::none(),
            ActiveSession::Selection(_) => Presentation::none(),
            ActiveSession::SelectionMove(_) => Presentation::none(),
        };

        self.clear_pending_previews();
        presentation
    }

    /// Install a replacement document. Cancellation runs first, against the
    /// outgoing document; the incoming document, its history, and its password
    /// are installed afterwards.
    pub(in crate::cxxqt_object) fn replace_document(
        &mut self,
        doc: Option<Document>,
        history: History,
        target: LayerTarget,
        pdf_password: Option<String>,
    ) {
        self.cancel_active_sessions_report_only();
        self.clear_document_scoped_state();
        self.session.install(doc, history, pdf_password);
        self.session.vanishing_planes = self
            .session
            .doc
            .as_ref()
            .and_then(crate::cxxqt_object::filter_tools::saved_planes);
        self.layer_target = target;
        self.reset_pyramid();
    }

    /// Clear every field that names or belongs to the outgoing document.
    pub(in crate::cxxqt_object) fn clear_document_scoped_state(&mut self) {
        self.cancel_move_composite();
        self.move_drag_active = false;
        self.mask_move_baseline = None;
        self.session.quick_mask.active = false;
        self.session.quick_mask.coverage = None;
        self.session.quick_mask.deselected = None;
        self.session.quick_mask.gray = false;
        self.session.quick_mask.pending = None;
        self.session.quick_mask.move_origin = None;
        self.session.set_selection_in_place(None);
        self.session.deselected_selection = None;
        self.session.quick_selection_snapshot = None;
        self.session.live = ActiveSession::Idle;
        self.path = None;
        self.session.set_compatibility(Default::default());
        self.session.history_compatibility.clear();
        self.session.skipped_xmp_mirror = false;
        self.session.recheck_compatibility_on_restore = false;
        self.mask_edit_target = None;
        self.mask_solo = None;
        self.mask_overlay = None;
        self.layer_target.clear();
        self.path_on_layer = false;
        self.active_path = super::super::paths::ActivePath::Work;
        self.layer_path_editing = None;
        self.session.pdf_options = None;
        self.session.pdf_password = None;
        if let Some(recovery) = &self.session.recovery {
            recovery.retire();
        }
        self.session.vanishing_planes = None;
        self.session.persisted_metadata_revision = 0;
        self.session.saved_metadata_revision = 0;
        self.session.save_publication_uncertain = false;
        self.isolation = None;
        self.session.clear_edit_state();
        self.expanded_groups.clear();
        self.move_base = None;
        self.move_view = None;
        self.move_view_pixels = Default::default();
        self.move_layer = None;
        self.move_x = 0;
        self.move_y = 0;
        self.move_layer_offset = (0, 0);
        self.move_opacity = 0;
        self.move_prepared_revision = 0;
        self.move_prepared_layer.clear();
        self.move_prepared_union = None;
        self.move_preview_cache_hit = false;
        self.path_drag_damage = None;
        self.move_above = None;
        self.move_above_origin = None;
        self.move_above_scale = 1;
        self.move_above_presented = None;
        self.transform_preview_level = 0;
        self.transform_controls = false;
        self.ruler = None;
        self.canvas_proxy = None;
        self.source_format = "psd".to_string();
        self.filter_error = None;
        self.smart_filter_edit = None;
        self.warp_text_original = None;
        self.save_embed_profile = None;
        self.save_raster_options = None;
        self.save_options = None;
        self.filter_jobs.cancel_all();
        self.auto_blend.cancel();
        self.auto_align.cancel();
        self.adjust_jobs.cancel();
    }

    pub(crate) fn set_history_log_name(&mut self, name: &str, opened: bool) {
        if self.session.doc.is_some() {
            self.session.history_log.set_name(name, opened);
        }
    }

    /// Drop the panel previews that were never committed.
    pub(in crate::cxxqt_object) fn clear_pending_previews(&mut self) {
        self.session.pending_previews.opacity_changed = false;
        self.session.pending_previews.fill_changed = false;
        self.session.pending_previews.mask_refine = None;
    }

    /// Restore the pre-lift document or the pre-trim layer extent. Returns
    /// whether the document changed and needs a rebuild.
    fn cancel_transform_session(&mut self, session: TransformSession) -> bool {
        self.move_base = None;
        self.move_view = None;
        self.move_view_pixels = Default::default();
        self.move_layer = None;
        self.move_x = 0;
        self.move_y = 0;
        self.move_layer_offset = (0, 0);
        self.move_opacity = 0;
        self.move_prepared_revision = 0;
        self.move_prepared_layer.clear();
        self.move_prepared_union = None;
        self.move_preview_cache_hit = false;
        for (member, rect) in &session.untrimmed {
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::untrim_to(doc, member, *rect);
            }
        }
        if let Some((_, before)) = session.lifted {
            self.session.doc.set(Some(*before));
            return true;
        }
        false
    }

    /// Restore the path a Pen or Path Selection gesture replaced.
    ///
    /// A Work or Saved path changes no pixel, so the abort notifies. A shape
    /// layer's outline lives in the layer: restoring it changes the composite,
    /// so the abort asks for a rebuild.
    pub(in crate::cxxqt_object) fn cancel_path_gesture_restore(
        &mut self,
        restore: PathRestore,
        refresh_display: bool,
    ) -> Presentation {
        let presentation = {
            let Some(doc) = self.session.doc.as_mut() else {
                return Presentation::none();
            };
            match restore {
                PathRestore::WorkOrSaved { active, path } => {
                    match active {
                        ActivePath::None | ActivePath::Work | ActivePath::VectorMask => {
                            doc.work_path = path
                        }
                        ActivePath::Saved(i) => match doc.saved_paths.get_mut(i) {
                            Some(saved) => saved.path = path,
                            None => return Presentation::none(),
                        },
                    }
                    self.active_path = active;
                    Presentation::notify()
                }
                PathRestore::Layer { target, subpaths } => {
                    let (width, height) = (doc.width, doc.height);
                    if let Some(layer) = pictura_render::resolve_path_mut(doc, &target) {
                        pictura_render::set_layer_shape_paths(layer, &subpaths, width, height);
                    }
                    Presentation::rebuild(Rebuild::Sync)
                }
            }
        };
        if presentation.rebuild == Rebuild::Sync && refresh_display {
            self.composite_into_doc();
        }
        presentation
    }

    /// Restore the path a Pen or Path Selection gesture replaced, taking the
    /// gesture out of the slot first.
    pub(in crate::cxxqt_object) fn cancel_path_gesture(&mut self) -> Presentation {
        let Some(restore) = self.session.live.take_path() else {
            return Presentation::none();
        };
        self.cancel_path_gesture_restore(restore, true)
    }
}
