//! The document session: one live session, one cancel path, one owner of the
//! document's state.
//!
//! The session is plain Rust. It holds the document, the selection, and the
//! history, and it reports what the Qt adapter must present; nothing here
//! knows about Qt.

use super::super::edit::Effect;
use super::super::paths::ActivePath;
use super::super::state::PictureViewRust;
use crate::history::{History, Restored, Snapshot};
use crate::layer_target::LayerTarget;
use pictura_core::path::{Subpath, VectorPath};
use pictura_core::{Document, PsdRect};
use pictura_select::Selection;

/// The kind of live session.
///
/// The slot holds one session at a time. A begin of another kind is refused; a
/// repeat begin of the same kind follows that kind's own rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::cxxqt_object) enum SessionKind {
    /// A brush or pencil stroke on a layer.
    Paint,
    /// A stroke on a layer mask's coverage.
    MaskPaint,
    /// A spot-healing or healing-brush gesture.
    Healing,
    /// A selection drag.
    SelectionMove,
    /// A Free Transform gesture.
    Transform,
    /// A Pen or Path Selection gesture on a path.
    Path,
    /// A lasso gesture, magnetic or not.
    Lasso,
    /// A filter dialog's live preview.
    FilterPreview,
    /// An image-mode dialog's live preview.
    ModePreview,
}

/// The document state the session owns: the document itself, the selection,
/// and the history that records its edits.
///
/// The session is the only owner, so a replacement swaps the whole group and
/// a headless test can drive it without a QObject.
#[derive(Default)]
pub(in crate::cxxqt_object) struct DocumentSession {
    /// The live document, `None` without one.
    pub(in crate::cxxqt_object) doc: Option<Document>,
    /// The live selection.
    pub(in crate::cxxqt_object) selection: Option<Selection>,
    /// CS6 "last deselected" memory: the selection replaced by Deselect or a
    /// New-mode commit. Restored by `reselect`.
    pub(in crate::cxxqt_object) deselected_selection: Option<Selection>,
    /// The recorded states.
    pub(in crate::cxxqt_object) history: History,
    /// Whether the document has unsaved changes.
    pub(in crate::cxxqt_object) dirty: bool,
    /// Bumped whenever the document's content changed.
    pub(in crate::cxxqt_object) content_revision: u64,
    /// Bumped whenever the view loads or replaces its document, so a panel can
    /// drop a gesture whose target no longer belongs to the current document.
    pub(in crate::cxxqt_object) document_generation: u64,
    /// The canonical `.joboptions` text this document was last saved to PDF
    /// with, so Save on the `.pdf` path reopens the dialog with it.
    pub(in crate::cxxqt_object) pdf_options: Option<String>,
    /// The password a protected PDF was opened with, kept in memory only so
    /// Revert can reopen it.
    pub(in crate::cxxqt_object) pdf_password: Option<String>,
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
        self.document_generation = self.document_generation.wrapping_add(1);
        self.dirty = false;
    }

    /// Capture the live document as a history state under `label`. False
    /// without a document.
    pub(in crate::cxxqt_object) fn record(&mut self, target: &LayerTarget, label: &str) -> bool {
        self.record_with(target, label, true)
    }

    /// As [`Self::record`] for a move of the topmost pixel layer, which leaves
    /// the cached move-preview base valid: no content-revision bump.
    pub(in crate::cxxqt_object) fn record_move(
        &mut self,
        target: &LayerTarget,
        label: &str,
    ) -> bool {
        self.record_with(target, label, false)
    }

    fn record_with(&mut self, target: &LayerTarget, label: &str, bump_revision: bool) -> bool {
        let Some(doc) = self.doc.as_mut() else {
            return false;
        };
        self.history
            .capture_live(doc, &self.selection, target, label);
        self.dirty = true;
        if bump_revision {
            self.content_revision = self.content_revision.wrapping_add(1);
        }
        true
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
        self.content_revision = self.content_revision.wrapping_add(1);
        Some(restored)
    }
}

/// What a path gesture replaced, so an abort can put it back.
pub(in crate::cxxqt_object) enum PathRestore {
    /// The Work or Saved path the gesture edited.
    WorkOrSaved {
        active: ActivePath,
        path: VectorPath,
    },
    /// A shape layer's outline, which lives in the layer.
    Layer {
        target: String,
        subpaths: Vec<Subpath>,
    },
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

/// A straight-sRGB region the adapter must blit. `None` means the adapter
/// composites the region from the current document.
pub(in crate::cxxqt_object) struct RegionBlit {
    pub(in crate::cxxqt_object) rect: PsdRect,
    pub(in crate::cxxqt_object) srgb: Option<pictura_core::PixelBuffer>,
}

/// What the session reports after a change: the effect to present and the
/// regions to blit.
pub(in crate::cxxqt_object) struct SessionChange {
    pub(in crate::cxxqt_object) effect: Effect,
    pub(in crate::cxxqt_object) regions: Vec<RegionBlit>,
}

impl SessionChange {
    pub(in crate::cxxqt_object) fn none() -> Self {
        Self {
            effect: Effect::Nothing,
            regions: Vec::new(),
        }
    }

    pub(in crate::cxxqt_object) fn notify() -> Self {
        Self {
            effect: Effect::Notify,
            regions: Vec::new(),
        }
    }

    pub(in crate::cxxqt_object) fn rebuild() -> Self {
        Self {
            effect: Effect::Rebuild,
            regions: Vec::new(),
        }
    }

    pub(in crate::cxxqt_object) fn blit(rect: PsdRect) -> Self {
        Self {
            effect: Effect::Blit { notify: false },
            regions: vec![RegionBlit { rect, srgb: None }],
        }
    }

    /// Keep the stronger of two reports: a rebuild covers a blit, and a blit
    /// keeps a notification that came with it.
    pub(in crate::cxxqt_object) fn merge(self, other: SessionChange) -> SessionChange {
        let effect = match (self.effect, other.effect) {
            (Effect::Rebuild, _) | (_, Effect::Rebuild) => Effect::Rebuild,
            (Effect::Blit { notify: a }, Effect::Blit { notify: b }) => {
                Effect::Blit { notify: a || b }
            }
            (Effect::Blit { .. }, Effect::Notify) | (Effect::Notify, Effect::Blit { .. }) => {
                Effect::Blit { notify: true }
            }
            (Effect::Blit { notify }, _) => Effect::Blit { notify },
            (_, Effect::Blit { notify }) => Effect::Blit { notify },
            (Effect::Notify, _) => Effect::Notify,
            (_, Effect::Notify) => Effect::Notify,
            (Effect::Nothing, Effect::Nothing) => Effect::Nothing,
        };
        let mut regions = self.regions;
        regions.extend(other.regions);
        SessionChange { effect, regions }
    }

    /// Whether the document's display changed and must be presented.
    pub(in crate::cxxqt_object) fn changed_display(&self) -> bool {
        !matches!(self.effect, Effect::Nothing)
    }
}

impl PictureViewRust {
    /// The kind of live session, inferred from the live gesture state.
    pub(in crate::cxxqt_object) fn active_session_kind(&self) -> Option<SessionKind> {
        if self.stroke.is_some() {
            Some(SessionKind::Paint)
        } else if self.mask_stroke.is_some() {
            Some(SessionKind::MaskPaint)
        } else if self.heal_stroke.is_some() {
            Some(SessionKind::Healing)
        } else if self.selection_move_origin.is_some() {
            Some(SessionKind::SelectionMove)
        } else if self.transform_session.is_some() {
            Some(SessionKind::Transform)
        } else if self.path_gesture.is_some() {
            Some(SessionKind::Path)
        } else if self.edge_map.is_some() || !self.pending_lasso.is_empty() {
            Some(SessionKind::Lasso)
        } else if self.filter_preview.is_some() || self.mask_filter_preview.is_some() {
            Some(SessionKind::FilterPreview)
        } else if self.mode_preview.is_some() {
            Some(SessionKind::ModePreview)
        } else {
            None
        }
    }

    /// Whether any live gesture owns the document.
    pub(in crate::cxxqt_object) fn has_active_session(&self) -> bool {
        self.active_session_kind().is_some()
    }

    /// Whether a begin of `kind` may take the session slot.
    ///
    /// The slot holds one session. A begin of another kind is refused. A repeat
    /// begin of the same kind follows that kind's rule: a transform replaces its
    /// own session, a filter or mode preview updates it, and a lasso attaches
    /// to the magnetic gesture it started. A healing, selection-move, path, or
    /// paint begin is refused.
    pub(in crate::cxxqt_object) fn slot_allows(&self, kind: SessionKind) -> bool {
        match self.active_session_kind() {
            None => true,
            Some(current) if current == kind => matches!(
                kind,
                SessionKind::Transform
                    | SessionKind::Lasso
                    | SessionKind::FilterPreview
                    | SessionKind::ModePreview
            ),
            Some(_) => false,
        }
    }

    /// Cancel every live gesture against the current document without recording
    /// history. The returned change is what the adapter must present.
    pub(in crate::cxxqt_object) fn cancel_active_sessions(&mut self) -> SessionChange {
        if !self.has_active_session() {
            self.clear_pending_previews();
            return SessionChange::none();
        }
        let mut change = SessionChange::none();

        if let Some(base) = self.mode_preview.take() {
            self.session.doc = Some(base);
            change = change.merge(SessionChange::rebuild());
        }

        if let Some(restored) = super::super::impl_filters::cancel_filter_preview(self) {
            change = change.merge(match restored {
                Some(rect) => SessionChange::blit(rect),
                None => SessionChange::rebuild(),
            });
        }

        if self.stroke.is_some() || self.mask_stroke.is_some() {
            change = change.merge(match self.cancel_stroke() {
                Some(rect) => SessionChange::blit(rect),
                None => SessionChange::rebuild(),
            });
        }

        if self.cancel_transform_state() {
            change = change.merge(SessionChange::rebuild());
        }

        change = change.merge(self.cancel_path_gesture());

        self.heal_stroke = None;
        self.pending_lasso.clear();
        self.pending_lasso_mode.clear();
        self.edge_map = None;
        self.cancel_selection_move();
        self.clear_pending_previews();
        change
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
        self.cancel_active_sessions();
        self.clear_document_scoped_state();
        self.session.install(doc, history, pdf_password);
        self.layer_target = target;
        self.reset_pyramid();
    }

    /// Clear every field that names or belongs to the outgoing document.
    pub(in crate::cxxqt_object) fn clear_document_scoped_state(&mut self) {
        self.session.selection = None;
        self.session.deselected_selection = None;
        self.selection_move_origin = None;
        self.selection_move_delta = None;
        self.path = None;
        self.layer_target.clear();
        self.path_on_layer = false;
        self.active_path = ActivePath::Work;
        self.path_gesture = None;
        self.session.pdf_options = None;
        self.session.pdf_password = None;
        self.pending_lasso.clear();
        self.pending_lasso_mode.clear();
        self.stroke = None;
        self.stroke_label.clear();
        self.stroke_base = None;
        self.mask_stroke = None;
        self.mask_edit_target = None;
        self.mask_solo = None;
        self.mask_overlay = None;
        self.isolation = None;
        self.pending_present = None;
        self.stroke_tiles = Default::default();
        self.present_flush_due = false;
        self.heal_stroke = None;
        self.move_base = None;
        self.move_layer = None;
        self.move_x = 0;
        self.move_y = 0;
        self.move_opacity = 0;
        self.move_prepared_revision = 0;
        self.move_prepared_layer.clear();
        self.move_preview_cache_hit = false;
        self.transform_session = None;
        self.clear_pending_previews();
        self.edge_map = None;
        self.ruler = None;
        self.preview = None;
        self.gpu_stroke = None;
        self.gpu_placer = None;
        self.filter_preview = None;
        self.mask_filter_preview = None;
        self.canvas_proxy = None;
        self.mode_preview = None;
        self.link_sets.clear();
        self.source_format = "psd".to_string();
        self.filter_error = None;
    }

    /// Drop the panel previews that were never committed.
    pub(in crate::cxxqt_object) fn clear_pending_previews(&mut self) {
        self.pending_previews.opacity_changed = false;
        self.pending_previews.fill_changed = false;
        self.pending_previews.mask_refine = None;
    }

    /// Restore the pre-lift document or the pre-trim layer extent. Returns
    /// whether the document changed and needs a rebuild.
    fn cancel_transform_state(&mut self) -> bool {
        let Some(session) = self.transform_session.take() else {
            return false;
        };
        if let Some(rect) = session.untrimmed {
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::untrim_to(doc, &session.path, rect);
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
    pub(in crate::cxxqt_object) fn cancel_path_gesture(&mut self) -> SessionChange {
        let Some(restore) = self.path_gesture.take() else {
            return SessionChange::none();
        };
        let Some(doc) = self.session.doc.as_mut() else {
            return SessionChange::none();
        };
        match restore {
            PathRestore::WorkOrSaved { active, path } => {
                match active {
                    ActivePath::None | ActivePath::Work => doc.work_path = path,
                    ActivePath::Saved(i) => match doc.saved_paths.get_mut(i) {
                        Some(saved) => saved.path = path,
                        None => return SessionChange::none(),
                    },
                }
                self.active_path = active;
                SessionChange::notify()
            }
            PathRestore::Layer { target, subpaths } => {
                let (width, height) = (doc.width, doc.height);
                let Some(layer) = pictura_render::resolve_path_mut(doc, &target) else {
                    return SessionChange::rebuild();
                };
                pictura_render::set_layer_shape_paths(layer, &subpaths, width, height);
                SessionChange::rebuild()
            }
        }
    }
}
