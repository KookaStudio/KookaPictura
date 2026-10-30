//! The Magnetic Lasso bridge: build the edge field once per gesture and trace
//! the live wire against it. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). The closed
//! outline is committed through the ordinary lasso path (`begin_lasso` /
//! `lasso_add_point` / `end_lasso`), so feather, combine mode, and history are
//! shared with the other lassos; nothing here records history.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Build and cache the edge field from the visible composite at `contrast` (1–100 %). False without a document or on a 32-bit document, where CS6 disables the tool.
        fn magnetic_begin(view: Pin<&mut PictureView>, contrast: i32) -> bool;

        /// The live wire from `(x0, y0)` to `(x1, y1)` within `width` px of the straight segment, as flat x,y pairs including both ends; a straight line without a cached field.
        fn magnetic_trace(
            view: &PictureView,
            x0: i32,
            y0: i32,
            x1: i32,
            y1: i32,
            width: i32,
        ) -> Vec<i32>;

        /// Drop the cached edge field.
        fn magnetic_end(view: Pin<&mut PictureView>);
    }
}
