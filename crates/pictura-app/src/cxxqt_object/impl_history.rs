mod display;
mod purge;

use super::helpers::{clamp_region, memory_gate, paint_timing};
use super::helpers_composite::{crop_planar, current_buffer, into_rgba_frame};
use super::presentation::{Dirty, Presentation, Rebuild};
use super::{qobject, PictureViewRust};
use crate::history::{detached_copy, History, Restored, Snapshot};
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, PsdRect};
use pictura_render::DocumentComposite;
use pictura_select::Selection;
use std::time::Instant;

/// A restore to present, whether it ended a live session on the way, and the
/// dirty value the saved identity implies.
struct ShownRestore {
    restored: Restored,
    had_live_session: bool,
    dirty: bool,
}

impl PictureViewRust {
    /// Whether a history restore of the document fits the memory governor's
    /// room; a refusal is recorded for `edit_refused_for_memory`.
    pub(super) fn restore_fits(&self) -> bool {
        let estimate = self
            .session
            .doc
            .as_ref()
            .map_or(0, memory_gate::restore_bytes);
        memory_gate::admit_fresh(estimate)
    }

    /// Fold `rect` of the document's own composite into level 0 and the
    /// pyramid without compositing it again — history has just restored those
    /// bytes — and return the damaged document rectangle.
    pub(super) fn refresh_from_composite(&mut self, rect: PsdRect) -> Option<PsdRect> {
        let doc = self.session.doc.as_ref()?;
        if doc.composite.width != doc.width || doc.composite.height != doc.height {
            return self.refresh_regions(&[rect]);
        }
        let (x0, y0, w, h) = clamp_region(rect, doc.width, doc.height)?;
        let crop = crop_planar(&doc.composite, x0, y0, w, h);
        let srgb = into_rgba_frame(pictura_codec::buffer_to_srgb(doc, &crop).into_owned());
        let clipped = PsdRect {
            top: y0,
            left: x0,
            bottom: y0 + h as i32,
            right: x0 + w as i32,
        };
        self.apply_refreshed_region(x0, y0, clipped, srgb);
        Some(clipped)
    }

    /// The live document for a named snapshot: planes of its own, so the next
    /// dab does not copy a whole plane, and an exact composite. Mid-stroke the
    /// document's composite is still the pre-stroke base, so it is composited
    /// from the painted layers; anything that presents the snapshot can then
    /// trust it the way a jump trusts a state's composite.
    pub(super) fn named_snapshot(&self) -> Option<Snapshot> {
        let mut doc = detached_copy(self.session.doc.as_ref()?);
        if self.session.live.stroke().is_some() {
            let rendered = current_buffer(&doc, self.gpu_compute);
            DocumentComposite::new(&mut doc).store(&rendered);
        }
        Some(Snapshot {
            quick_mask: self
                .session
                .quick_mask
                .active
                .then(|| self.session.quick_mask.coverage.clone()),
            doc,
            selection: self.session.selection.clone(),
        })
    }

    /// Restore through the history in place into the live document, or by
    /// replacement when there is none. A live session is cancelled first; its
    /// pixels were never captured.
    fn restore(
        &mut self,
        step: impl FnOnce(
            &mut History,
            &mut Document,
            &mut Option<Selection>,
            &mut LayerTarget,
        ) -> Option<Restored>,
        legacy: impl FnOnce(&mut History) -> Option<Snapshot>,
    ) -> Option<ShownRestore> {
        let had_live_session = self.has_active_session();
        let cancelled = self.cancel_active_sessions_report_only();
        let t = Instant::now();
        let restored = self.session.restore(&mut self.layer_target, step, legacy)?;
        if self.session.quick_mask.active {
            self.invalidate_quick_mask();
        }
        paint_timing::record("history_restore", t.elapsed());
        Some(ShownRestore {
            restored: if cancelled.has_display_change() {
                Restored::Everywhere
            } else {
                restored
            },
            had_live_session,
            dirty: self.session.derived_dirty(),
        })
    }

    /// Convert a history report into the one presentation shape. The region
    /// paths apply the restored composite and the mask-solo refresh; the
    /// deferred rebuild is the presenter's.
    pub(super) fn restore_presentation(&mut self, restored: Restored, dirty: bool) -> Presentation {
        let (regions, rebuild) = match restored {
            Restored::Everywhere => (Vec::new(), Rebuild::Deferred),
            Restored::Region(rect) => (
                self.refresh_from_composite(rect).into_iter().collect(),
                Rebuild::None,
            ),
            Restored::Composite(rect) => {
                self.refresh_mask_solo();
                (
                    self.refresh_from_composite(rect).into_iter().collect(),
                    Rebuild::None,
                )
            }
        };
        Presentation {
            regions,
            rebuild,
            notify: true,
            dirty: Dirty::Set(dirty),
            revision: true,
            declaration: None,
        }
    }
}

impl qobject::PictureView {
    pub fn undo(mut self: Pin<&mut Self>) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if !self.rust().can_undo() || !self.rust().restore_fits() {
            return false;
        }
        let restored = self
            .as_mut()
            .rust_mut()
            .restore(History::undo_live, History::undo);
        self.show_restored(restored)
    }

    pub fn redo(mut self: Pin<&mut Self>) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if !self.rust().can_redo() || !self.rust().restore_fits() {
            return false;
        }
        let restored = self
            .as_mut()
            .rust_mut()
            .restore(History::redo_live, History::redo);
        self.show_restored(restored)
    }

    /// Present a restore: only the restored region when history bounded it,
    /// otherwise the whole canvas from the restored composite, its pyramid
    /// following from a worker on a large document. Either way `changed`
    /// follows for the panels and overlays; a bounded restore leaves
    /// `frame_revision` alone, so the shell skips the full image.
    ///
    /// Only a restore that ended a live session reports `session_cancelled`,
    /// whose handler refreshes the panels inline. Any other restore reports
    /// `history_restored` and leaves them to the debounce `changed` started, so
    /// undo stays off the input path the way a stroke commit does (#1067).
    fn show_restored(mut self: Pin<&mut Self>, shown: Option<ShownRestore>) -> bool {
        let Some(ShownRestore {
            restored,
            had_live_session,
            dirty,
        }) = shown
        else {
            return false;
        };
        let t = Instant::now();
        let presentation = self
            .as_mut()
            .rust_mut()
            .restore_presentation(restored, dirty);
        self.as_mut().present(presentation);
        paint_timing::record("history_show_restored", t.elapsed());
        if had_live_session {
            self.as_mut().session_cancelled();
        } else {
            self.as_mut().history_restored();
        }
        true
    }

    pub fn can_undo(&self) -> bool {
        self.rust().can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.rust().can_redo()
    }

    pub fn history_depth(&self) -> i32 {
        self.rust().session.history.depth() as i32
    }

    pub fn history_count(&self) -> i32 {
        self.rust().session.history.count() as i32
    }

    pub fn history_index(&self) -> i32 {
        self.rust().session.history.index() as i32
    }

    pub fn history_label(&self, i: i32) -> QString {
        if i < 0 {
            return QString::default();
        }
        QString::from(self.rust().session.history.label(i as usize))
    }

    pub fn history_jump(mut self: Pin<&mut Self>, i: i32) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if i < 0 {
            return false;
        }
        let i = i as usize;
        if i >= self.rust().session.history.count() || !self.rust().restore_fits() {
            return false;
        }
        let restored = self.as_mut().rust_mut().restore(
            |history, doc, selection, target| history.jump_live(i, doc, selection, target),
            |history| history.jump(i),
        );
        self.show_restored(restored)
    }

    pub fn history_add_snapshot(mut self: Pin<&mut Self>, label: &QString) -> bool {
        self.as_mut().settle_pending_composite();
        let estimate = self
            .rust()
            .session
            .doc
            .as_ref()
            .map_or(0, memory_gate::snapshot_bytes);
        if !memory_gate::admit_fresh(estimate) {
            return false;
        }
        let Some(snapshot) = self.rust().named_snapshot() else {
            return false;
        };
        let target = self.rust().layer_target.clone();
        self.as_mut()
            .rust_mut()
            .session
            .history
            .add_snapshot(&label.to_string(), snapshot, target);
        self.changed();
        true
    }

    pub fn history_restore_snapshot(mut self: Pin<&mut Self>, i: i32) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if i < 0 {
            return false;
        }
        let Some(snapshot) = self.rust().session.history.snapshot(i as usize) else {
            return false;
        };
        if !memory_gate::admit_fresh(memory_gate::snapshot_restore_bytes(&snapshot.doc)) {
            return false;
        }
        let label = self
            .rust()
            .session
            .history
            .snapshot_label(i as usize)
            .to_string();
        let target = self.rust().session.history.snapshot_target(i as usize);
        let mut rust = self.as_mut().rust_mut();
        rust.cancel_active_sessions_report_only();
        rust.session.doc = Some(detached_copy(&snapshot.doc));
        rust.session.set_selection(snapshot.selection);
        rust.session
            .restore_quick_mask_selection(snapshot.quick_mask);
        if rust.session.quick_mask.active {
            rust.invalidate_quick_mask();
        }
        if let Some(mut target) = target {
            if let Some(doc) = rust.session.doc.as_ref() {
                target.retain_resolving(doc);
            }
            rust.set_layer_target(target);
        }
        let target = rust.layer_target.clone();
        rust.session.record(&target, &label);
        rust.mask_edit_target = None;
        rust.active_path = super::paths::ActivePath::Work;
        rust.path_on_layer = false;
        rust.ruler = None;
        rust.canvas_proxy = None;
        rust.session.deselected_selection = None;
        rust.session.document_generation = rust.session.document_generation.wrapping_add(1);
        self.as_mut().recomposite();
        self.as_mut().session_cancelled();
        true
    }
}
