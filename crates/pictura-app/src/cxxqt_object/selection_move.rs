//! The Move tool's selection-drag state machine.
//!
//! A drag stores the origin plane plus the offset from it; the translated plane
//! is built once, on commit. Every consumer that needs the selection as it
//! looks during the drag goes through [`PictureViewRust::selection_bounds`] and
//! [`PictureViewRust::selection_coverage_at`], which apply the offset, so a
//! pointer move costs no plane allocation.

use pictura_core::PsdRect;

use super::state::PictureViewRust;

impl PictureViewRust {
    /// Capture the current selection as the drag origin. False without one.
    pub(super) fn begin_selection_move(&mut self) -> bool {
        if !self.slot_allows(super::document_session::SessionKind::SelectionMove) {
            return false;
        }
        let Some(origin) = self.session.selection.clone() else {
            return false;
        };
        self.selection_move_origin = Some(origin);
        self.selection_move_delta = Some((0, 0));
        true
    }

    /// Record the drag offset. False when no drag is active.
    pub(super) fn preview_selection_move(&mut self, dx: i32, dy: i32) -> bool {
        if self.selection_move_origin.is_none() {
            return false;
        }
        self.selection_move_delta = Some((dx, dy));
        true
    }

    /// Build the translated plane once and drop the drag. False when no drag is
    /// active or the offset is zero, in which case the origin stays in place.
    pub(super) fn commit_selection_move(&mut self) -> bool {
        let Some(origin) = self.selection_move_origin.take() else {
            return false;
        };
        let delta = self.selection_move_delta.take().unwrap_or((0, 0));
        if delta == (0, 0) {
            return false;
        }
        self.session.selection = Some(origin.translate(delta.0, delta.1));
        true
    }

    /// Drop the drag without touching the plane. False when no drag is active.
    pub(super) fn cancel_selection_move(&mut self) -> bool {
        self.selection_move_delta = None;
        self.selection_move_origin.take().is_some()
    }

    /// The selection's bounding box as it looks now: the origin's bounds
    /// translated by the drag offset and clipped to the canvas, or a row scan
    /// when no drag is active.
    pub(super) fn selection_bounds(&self) -> Option<PsdRect> {
        let sel = self.session.selection.as_ref()?;
        let bounds = sel.bounds()?;
        let Some((dx, dy)) = self.selection_move_delta else {
            return Some(bounds);
        };
        let (w, h) = (sel.width as i32, sel.height as i32);
        let moved = PsdRect {
            top: (bounds.top + dy).clamp(0, h),
            left: (bounds.left + dx).clamp(0, w),
            bottom: (bounds.bottom + dy).clamp(0, h),
            right: (bounds.right + dx).clamp(0, w),
        };
        (moved.right > moved.left && moved.bottom > moved.top).then_some(moved)
    }

    /// Selection coverage at a document pixel, read through the drag offset.
    pub(super) fn selection_coverage_at(&self, x: i32, y: i32) -> u8 {
        let Some(sel) = self.session.selection.as_ref() else {
            return 0;
        };
        let (dx, dy) = self.selection_move_delta.unwrap_or((0, 0));
        let (x, y) = (x - dx, y - dy);
        if x < 0 || y < 0 || x >= sel.width as i32 || y >= sel.height as i32 {
            return 0;
        }
        sel.data[y as usize * sel.width as usize + x as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_select::Selection;

    fn rect_selection(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Selection {
        let mut sel = Selection::none(w, h);
        for y in y0..y1 {
            let row = y as usize * w as usize;
            sel.data[row + x0 as usize..row + x1 as usize].fill(255);
        }
        sel
    }

    fn view(selection: Selection) -> PictureViewRust {
        let mut view = PictureViewRust::default();
        view.session.selection = Some(selection);
        view
    }

    #[test]
    fn preview_keeps_the_origin_plane() {
        let origin = rect_selection(32, 32, 4, 4, 12, 12);
        let mut rust = view(origin.clone());
        assert!(rust.begin_selection_move());
        assert!(rust.preview_selection_move(3, 4));
        assert_eq!(rust.session.selection.as_ref(), Some(&origin));
        assert_eq!(rust.selection_move_delta, Some((3, 4)));
    }

    #[test]
    fn coverage_follows_the_drag_offset() {
        let mut rust = view(rect_selection(32, 32, 4, 4, 12, 12));
        assert!(rust.begin_selection_move());
        assert_eq!(rust.selection_coverage_at(13, 8), 0);
        assert!(rust.preview_selection_move(3, 4));
        assert_eq!(rust.selection_coverage_at(13, 8), 255);
        assert_eq!(rust.selection_coverage_at(5, 5), 0);
        assert_eq!(rust.selection_coverage_at(7, 8), 255);
    }

    #[test]
    fn commit_materialises_the_translated_plane() {
        let origin = rect_selection(32, 32, 4, 4, 12, 12);
        let mut rust = view(origin.clone());
        assert!(rust.begin_selection_move());
        assert!(rust.preview_selection_move(3, 4));
        assert!(rust.commit_selection_move());
        assert_eq!(
            rust.session.selection.as_ref(),
            Some(&origin.translate(3, 4))
        );
        assert!(rust.selection_move_origin.is_none());
        assert!(rust.selection_move_delta.is_none());
    }

    #[test]
    fn a_zero_offset_commit_keeps_the_origin() {
        let origin = rect_selection(32, 32, 4, 4, 12, 12);
        let mut rust = view(origin.clone());
        assert!(rust.begin_selection_move());
        assert!(!rust.commit_selection_move());
        assert_eq!(rust.session.selection.as_ref(), Some(&origin));
        assert!(rust.selection_move_delta.is_none());
    }

    #[test]
    fn cancel_drops_the_drag_without_moving_the_plane() {
        let origin = rect_selection(32, 32, 4, 4, 12, 12);
        let mut rust = view(origin.clone());
        assert!(rust.begin_selection_move());
        assert!(rust.preview_selection_move(-6, 9));
        assert!(rust.cancel_selection_move());
        assert_eq!(rust.session.selection.as_ref(), Some(&origin));
        assert!(rust.selection_move_delta.is_none());
        assert_eq!(rust.selection_coverage_at(5, 5), 255);
    }

    #[test]
    fn selection_bounds_follow_the_drag_offset() {
        let mut rust = view(rect_selection(32, 32, 4, 4, 12, 12));
        let origin = rust.selection_bounds().expect("origin bounds");
        assert!(rust.begin_selection_move());
        assert!(rust.preview_selection_move(7, -2));
        let moved = rust.selection_bounds().expect("moved bounds");
        assert_eq!(
            moved,
            PsdRect {
                top: origin.top - 2,
                left: origin.left + 7,
                bottom: origin.bottom - 2,
                right: origin.right + 7,
            }
        );
        assert_eq!(
            moved,
            rust.session
                .selection
                .as_ref()
                .expect("plane")
                .translate(7, -2)
                .bounds()
                .expect("translated bounds")
        );
    }

    #[test]
    fn selection_bounds_clip_at_the_canvas_edge() {
        let mut rust = view(rect_selection(32, 32, 24, 24, 32, 32));
        assert!(rust.begin_selection_move());
        assert!(rust.preview_selection_move(10, 10));
        assert_eq!(rust.selection_bounds(), None, "fully off the canvas");
        assert!(rust.preview_selection_move(4, 4));
        assert_eq!(
            rust.selection_bounds(),
            Some(PsdRect {
                top: 28,
                left: 28,
                bottom: 32,
                right: 32
            })
        );
    }
}
