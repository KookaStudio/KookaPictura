//! Quick Selection: the wand region grows inside one stroke, previews without
//! history, and commits one Selection state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::edit::Edit;
use super::super::helpers::combine_mode_from;
use super::super::picture::picture;
use super::super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_select::{CombineMode, Selection};

pub(super) fn combine_selection(
    rust: &mut super::super::state::PictureViewRust,
    shape: Selection,
    mode: CombineMode,
    keep_reselect: bool,
) {
    let previous = rust.session.take_selection();
    let replaced = matches!(mode, CombineMode::New)
        && previous
            .as_ref()
            .is_some_and(|s| s.data.iter().any(|&v| v > 0));
    if replaced {
        rust.session.deselected_selection = previous;
        rust.session.set_selection(Some(shape));
    } else {
        if !keep_reselect {
            rust.session.deselected_selection = None;
        }
        rust.session.set_selection(Some(match previous {
            Some(mut base) => {
                base.combine_with(&shape, mode);
                base
            }
            // Combined into nothing, a new or added shape is itself.
            None if matches!(mode, CombineMode::New | CombineMode::Add) => shape,
            None => Selection::none(shape.width, shape.height),
        }));
    }
}

impl qobject::PictureView {
    fn quick_select_shape(&self, x: i32, y: i32, tolerance: i32) -> Option<Selection> {
        let rust = self.rust();
        let doc = rust.session.doc.as_ref()?;
        if x < 0 || y < 0 || x as u32 >= doc.width || y as u32 >= doc.height {
            return None;
        }
        let tolerance = tolerance.clamp(0, 255) as u8;
        pictura_select::magic_wand(&picture(doc, rust), x as u32, y as u32, tolerance, true).ok()
    }

    pub fn quick_select(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        mode: &QString,
    ) -> bool {
        let Some(shape) = self.quick_select_shape(x, y, tolerance) else {
            return false;
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    /// Apply one Quick Selection dab without recording history.
    pub fn quick_select_preview(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        mode: &QString,
        is_move: bool,
    ) -> bool {
        // A move inside the region the stroke has already selected adds
        // nothing. Skip before this dab composites, floods, combines, and
        // retraces the outline. New, Subtract, Intersect, the first press, and
        // partially covered pixels still run the full path.
        // ponytail: a seed whose local colour differs can no longer contribute
        // a few extra pixels inside an existing selection; the stroke does not
        // require that growth.
        if is_move && combine_mode_from(&mode.to_string()) == CombineMode::Add {
            let rust = self.rust();
            let inside = rust.session.selection.as_ref().is_some_and(|sel| {
                x >= 0
                    && y >= 0
                    && (x as u32) < sel.width
                    && (y as u32) < sel.height
                    && sel.data.get(y as usize * sel.width as usize + x as usize) == Some(&255)
            });
            if inside {
                return true;
            }
        }
        let Some(shape) = self.quick_select_shape(x, y, tolerance) else {
            return false;
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().edit("Selection", |rust| {
            // The first dab may set the reselect memory; later dabs of the same
            // stroke keep it instead of clearing it.
            let keep = rust
                .session
                .quick_selection_snapshot
                .as_ref()
                .is_some_and(|s| s.reselect.is_some());
            combine_selection(rust, shape, mode, keep);
            if !keep {
                if let Some(snapshot) = rust.session.quick_selection_snapshot.as_mut() {
                    snapshot.reselect = Some(rust.session.deselected_selection.clone());
                }
            }
            Edit::SelectionPreview
        });
        true
    }

    /// Snapshot the selection before a Quick Selection stroke.
    pub fn quick_select_begin(mut self: Pin<&mut Self>) -> bool {
        let mut rust = self.as_mut().rust_mut();
        if rust.session.doc.is_none() {
            return false;
        }
        let snapshot = super::super::document_session::QuickSelectionSnapshot {
            selection: rust.session.selection.clone(),
            deselected: rust.session.deselected_selection.clone(),
            reselect: None,
        };
        rust.session.quick_selection_snapshot = Some(snapshot);
        true
    }

    /// Record the stroke's one history state. False when there is no snapshot
    /// (no stroke, or already committed) or the selection did not change.
    pub fn quick_select_commit(mut self: Pin<&mut Self>) -> bool {
        let changed = {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return false;
            }
            let Some(snapshot) = rust.session.quick_selection_snapshot.take() else {
                return false;
            };
            let changed = rust.session.selection != snapshot.selection;
            if !changed {
                rust.session.deselected_selection = snapshot.deselected;
            }
            changed
        };
        if changed {
            self.as_mut().edit("Selection", |_| Edit::Meta);
        }
        changed
    }

    /// Restore the selection captured at the start of a Quick Selection stroke.
    pub fn quick_select_cancel(mut self: Pin<&mut Self>) -> bool {
        let snapshot = self
            .as_mut()
            .rust_mut()
            .session
            .quick_selection_snapshot
            .take();
        let Some(snapshot) = snapshot else {
            return false;
        };
        self.as_mut().edit("Selection", |rust| {
            rust.session.set_selection(snapshot.selection);
            rust.session.deselected_selection = snapshot.deselected;
            Edit::SelectionPreview
        });
        true
    }
}
