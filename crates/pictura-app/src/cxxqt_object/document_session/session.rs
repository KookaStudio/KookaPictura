//! The document session: one live session, one cancel path, one owner of the
//! document's state.
//!
//! The session is plain Rust. It holds the document, the selection, and the
//! history, and it reports what the Qt adapter must present; nothing here
//! knows about Qt.

use super::super::paths::ActivePath;
use super::super::presentation::{Presentation, Rebuild};
use super::super::state::PictureViewRust;
use super::active::{ActiveSession, PathRestore, SessionKind};
use super::transform::TransformSession;
use crate::history::{History, Restored, Snapshot};
use crate::layer_target::LayerTarget;
use pictura_core::{Document, PsdRect};
use pictura_select::Selection;

/// The selection state captured at the start of a Quick Selection stroke.
pub(in crate::cxxqt_object) struct QuickSelectionSnapshot {
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
    pub(in crate::cxxqt_object) doc: Option<Document>,
    /// The live selection.
    pub(in crate::cxxqt_object) selection: Option<Selection>,
    /// Bumped whenever the selection is written. The committed-outline cache
    /// keys on it; a history restore counts as a write.
    pub(in crate::cxxqt_object) selection_revision: u64,
    /// CS6 "last deselected" memory: the selection replaced by Deselect or a
    /// New-mode commit. Restored by `reselect`.
    pub(in crate::cxxqt_object) deselected_selection: Option<Selection>,
    /// The selection state at the start of a Quick Selection stroke, restored
    /// when the stroke is cancelled.
    pub(in crate::cxxqt_object) quick_selection_snapshot: Option<QuickSelectionSnapshot>,
    /// The recorded states.
    pub(in crate::cxxqt_object) history: History,
    /// Whether the document has unsaved changes.
    pub(in crate::cxxqt_object) dirty: bool,
    /// The history identity that was current at the last successful save, open,
    /// install, or Revert. `None` means the document has no saved point.
    pub(in crate::cxxqt_object) saved_id: Option<u64>,
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
    /// The Vanishing Point planes this document's last session kept, so the
    /// dialog reopens on them: read from the file when the document is
    /// installed and written into it by a save. Kept out of the document so
    /// undo leaves them alone, as no history state records them.
    pub(in crate::cxxqt_object) vanishing_planes: Option<pictura_filters::VpPlanes>,
    /// Encoded PSD channel streams for this document session. Shared with the
    /// save and recovery workers; replaced when a document is installed.
    pub(in crate::cxxqt_object) encode_cache: std::sync::Arc<pictura_codec::EncodeCache>,
}

impl DocumentSession {
    /// A session with `doc` installed and an empty history.
    #[cfg(test)]
    pub(in crate::cxxqt_object) fn with_document(doc: Document) -> Self {
        Self {
            doc: Some(doc),
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
        self.selection = selection;
        self.touch_selection();
    }

    /// Geometry changes invalidate both the editing clip and temporary mask coverage.
    pub(in crate::cxxqt_object) fn clear_geometry_selection(&mut self) {
        self.set_selection(None);
        self.quick_mask.coverage = None;
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
        self.doc = doc;
        self.history = history;
        self.pdf_password = pdf_password;
        self.encode_cache = std::sync::Arc::new(pictura_codec::EncodeCache::new());
        self.document_generation = self.document_generation.wrapping_add(1);
        self.saved_id = self.history.current_id();
        self.refresh_dirty();
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
        self.dirty = false;
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
            (Some(_), Some(saved)) => self.history.current_id() != Some(saved),
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
        let restored = match self.doc.as_mut() {
            Some(doc) => step(&mut self.history, doc, &mut self.selection, target),
            None => legacy(&mut self.history).map(|snapshot| {
                self.doc = Some(snapshot.doc);
                self.selection = snapshot.selection;
                Restored::Everywhere
            }),
        }?;
        let quick_mask = self
            .history
            .current_snapshot()
            .and_then(|s| s.quick_mask.clone());
        self.restore_quick_mask_selection(quick_mask);
        self.touch_selection();
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
    /// cancel/commit, so an edit made under it would be discarded. The Move
    /// tool's controls box commits per gesture and never locks.
    pub(in crate::cxxqt_object) fn modal_transform_blocks_edit(&self) -> bool {
        self.lifted_transform_is_live() && !self.transform_controls
    }

    /// Fill and Stroke refuse any pending transform, lifted or not, matching
    /// the Edit menu lock. The Move tool's controls box is not a transform.
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
        self.session.live.allows(kind)
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
                self.session.doc = Some(*base);
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
        self.session.set_selection(None);
        self.session.deselected_selection = None;
        self.session.quick_selection_snapshot = None;
        self.session.live = ActiveSession::Idle;
        self.path = None;
        self.mask_edit_target = None;
        self.mask_solo = None;
        self.mask_overlay = None;
        self.layer_target.clear();
        self.path_on_layer = false;
        self.active_path = super::super::paths::ActivePath::Work;
        self.session.pdf_options = None;
        self.session.pdf_password = None;
        self.session.vanishing_planes = None;
        self.isolation = None;
        self.solo_snapshot = None;
        self.expanded_groups.clear();
        self.move_base = None;
        self.move_view = None;
        self.move_layer = None;
        self.move_x = 0;
        self.move_y = 0;
        self.move_layer_offset = (0, 0);
        self.move_opacity = 0;
        self.move_prepared_revision = 0;
        self.move_prepared_layer.clear();
        self.move_prepared_union = None;
        self.move_preview_cache_hit = false;
        self.fade = None;
        self.last_transform = None;
        self.transform_controls = false;
        self.clear_pending_previews();
        self.ruler = None;
        self.canvas_proxy = None;
        self.link_sets.clear();
        self.link_remap = None;
        self.source_format = "psd".to_string();
        self.filter_error = None;
        self.save_embed_profile = None;
        self.save_raster_options = None;
        self.filter_jobs.cancel_all();
        self.auto_blend.cancel();
        self.auto_align.cancel();
        self.adjust_jobs.cancel();
        self.save_job = None;
    }

    /// Drop the panel previews that were never committed.
    pub(in crate::cxxqt_object) fn clear_pending_previews(&mut self) {
        self.pending_previews.opacity_changed = false;
        self.pending_previews.fill_changed = false;
        self.pending_previews.mask_refine = None;
    }

    /// Restore the pre-lift document or the pre-trim layer extent. Returns
    /// whether the document changed and needs a rebuild.
    fn cancel_transform_session(&mut self, session: TransformSession) -> bool {
        self.move_base = None;
        self.move_view = None;
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
            self.session.doc = Some(*before);
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
                        ActivePath::None | ActivePath::Work => doc.work_path = path,
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
