mod display;
mod purge;

use super::helpers::clamp_region;
use super::helpers_composite::{crop_planar, into_rgba_frame};
use super::{qobject, PictureViewRust};
use crate::history::{detached_copy, History, Restored, Snapshot};
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, PsdRect};
use pictura_select::Selection;

impl PictureViewRust {
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
    ) -> Option<Restored> {
        let cancelled = self.cancel_active_sessions();
        let had_session = cancelled.changed_display();
        let restored = self.session.restore(&mut self.layer_target, step, legacy)?;
        Some(if had_session {
            Restored::Everywhere
        } else {
            restored
        })
    }
}

impl qobject::PictureView {
    pub fn undo(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().session.history.can_undo() {
            return false;
        }
        let restored = self
            .as_mut()
            .rust_mut()
            .restore(History::undo_live, History::undo);
        self.show_restored(restored, false)
    }

    pub fn redo(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().session.history.can_redo() {
            return false;
        }
        let restored = self
            .as_mut()
            .rust_mut()
            .restore(History::redo_live, History::redo);
        self.show_restored(restored, false)
    }

    /// Present a restore: only the restored region when history bounded it,
    /// otherwise the whole canvas — rebuilt from the restored composite, or
    /// recomposited when `recomposite`. Either way `changed` follows for the
    /// panels and overlays; a bounded restore leaves `frame_revision` alone, so
    /// the shell skips the full image.
    fn show_restored(
        mut self: Pin<&mut Self>,
        restored: Option<Restored>,
        recomposite: bool,
    ) -> bool {
        match restored {
            None => return false,
            Some(Restored::Everywhere) if recomposite => self.as_mut().recomposite(),
            Some(Restored::Everywhere) => {
                self.as_mut().rust_mut().reset_pyramid();
                self.as_mut().changed();
            }
            Some(Restored::Region(rect)) => {
                if let Some(damaged) = self.as_mut().rust_mut().refresh_from_composite(rect) {
                    self.as_mut().region_damaged(
                        damaged.left,
                        damaged.top,
                        damaged.width(),
                        damaged.height(),
                    );
                }
                self.as_mut().changed();
            }
        }
        self.as_mut().session_cancelled();
        true
    }

    pub fn can_undo(&self) -> bool {
        self.rust().session.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.rust().session.history.can_redo()
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
        if i < 0 {
            return false;
        }
        let i = i as usize;
        if i >= self.rust().session.history.count() {
            return false;
        }
        let restored = self.as_mut().rust_mut().restore(
            |history, doc, selection, target| history.jump_live(i, doc, selection, target),
            |history| history.jump(i),
        );
        self.show_restored(restored, true)
    }

    pub fn history_add_snapshot(mut self: Pin<&mut Self>, label: &QString) -> bool {
        // The named state gets planes of its own: one shared with the live
        // document would make the next dab copy a whole plane.
        let Some(snapshot) = self.snapshot() else {
            return false;
        };
        let snapshot = Snapshot {
            doc: detached_copy(&snapshot.doc),
            ..snapshot
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

    pub fn history_snapshot_count(&self) -> i32 {
        self.rust().session.history.snapshot_count() as i32
    }

    pub fn history_snapshot_label(&self, i: i32) -> QString {
        if i < 0 {
            return QString::default();
        }
        QString::from(self.rust().session.history.snapshot_label(i as usize))
    }

    pub fn history_restore_snapshot(mut self: Pin<&mut Self>, i: i32) -> bool {
        if i < 0 {
            return false;
        }
        let Some(snapshot) = self.rust().session.history.snapshot(i as usize) else {
            return false;
        };
        let label = self
            .rust()
            .session
            .history
            .snapshot_label(i as usize)
            .to_string();
        let target = self.rust().session.history.snapshot_target(i as usize);
        let mut rust = self.as_mut().rust_mut();
        rust.cancel_active_sessions();
        rust.session.doc = Some(detached_copy(&snapshot.doc));
        rust.session.selection = snapshot.selection;
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
