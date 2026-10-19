//! Selection move bridge: origin capture, preview, commit, cancel, and the
//! content move. Free functions over [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
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
