//! Selection move bridge: origin capture, preview, commit, cancel, and the
//! content move. Free functions over [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt_lib::QString;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Capture the current selection as the origin of a move drag; false without one.
        fn begin_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Set selection to the origin translated by `(dx, dy)`; no history, no `changed`.
        fn preview_selection_move(view: Pin<&mut PictureView>, dx: i32, dy: i32) -> bool;

        /// Record one "Move Selection" state when the selection moved; false on no-op.
        fn commit_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Restore the origin selection and drop the drag; no history.
        fn cancel_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Move selected content by `(dx, dy)`; `duplicate` copies. One undo state.
        fn move_selection_content(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            duplicate: bool,
            background: u32,
        ) -> bool;

        /// Flood-select around `(x, y)` within `tolerance` and combine per `mode`.
        fn quick_select(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            mode: &QString,
        ) -> bool;

        /// Snapshot the selection before a Quick Selection stroke.
        fn quick_select_begin(view: Pin<&mut PictureView>) -> bool;

        /// Apply one Quick Selection dab without recording history.
        fn quick_select_preview(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            mode: &QString,
        ) -> bool;

        /// Record the Quick Selection stroke's one history state.
        fn quick_select_commit(view: Pin<&mut PictureView>) -> bool;

        /// Restore the selection captured at the start of the Quick Selection stroke.
        fn quick_select_cancel(view: Pin<&mut PictureView>) -> bool;
    }
}

fn begin_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.begin_selection_move()
}

fn preview_selection_move(view: Pin<&mut PictureView>, dx: i32, dy: i32) -> bool {
    view.preview_selection_move(dx, dy)
}

fn commit_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.commit_selection_move()
}

fn cancel_selection_move(view: Pin<&mut PictureView>) -> bool {
    view.cancel_selection_move()
}

fn move_selection_content(
    view: Pin<&mut PictureView>,
    dx: i32,
    dy: i32,
    duplicate: bool,
    background: u32,
) -> bool {
    view.move_selection_content(dx, dy, duplicate, background)
}

fn quick_select(
    view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    mode: &QString,
) -> bool {
    view.quick_select(x, y, tolerance, mode)
}

fn quick_select_begin(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_begin()
}

fn quick_select_preview(
    view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    tolerance: i32,
    mode: &QString,
) -> bool {
    view.quick_select_preview(x, y, tolerance, mode)
}

fn quick_select_commit(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_commit()
}

fn quick_select_cancel(view: Pin<&mut PictureView>) -> bool {
    view.quick_select_cancel()
}
