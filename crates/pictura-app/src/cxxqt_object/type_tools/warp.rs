//! `Type > Warp Text`: free functions over a [`PictureView`] for the Warp Text
//! dialog. A preview re-sets the type layer with the new warp and records
//! nothing; the first preview stashes the layer as it was, and Cancel puts
//! that copy back, so an opened layer's own pixels survive a cancelled
//! dialog. Commit records one "Warp Text" state over the pre-dialog state.
//!
//! [`PictureView`]: crate::cxxqt_object::qobject::PictureView

mod imp;

use crate::cxxqt_object::qobject::PictureView;
pub(super) use imp::*;

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
        /// Whether the layer at `path` is type carrying a `Type > Warp Text` warp (the Layers panel's warped row).
        fn type_layer_is_warped(view: &PictureView, path: &QString) -> bool;

        /// Whether Warp Text may warp the type layer at `path`: 0 yes, 1 type on a path, 2 Faux Bold, 3 a font without outlines, -1 not a type layer.
        fn type_warp_refusal(view: &PictureView, path: &QString) -> i32;

        /// Re-set the type layer at `path` with the warp (`style` a `WarpStyle` id, `none` or empty for none; percentages) for the dialog's live preview, recording no state. False (unchanged) for a missing or refused layer.
        fn type_warp_preview(
            view: Pin<&mut PictureView>,
            path: &QString,
            style: &QString,
            bend: f64,
            h: f64,
            v: f64,
            vertical: bool,
        ) -> bool;

        /// Put back the layer the dialog's first preview stashed, recording no state. False when nothing was previewed.
        fn type_warp_cancel(view: Pin<&mut PictureView>) -> bool;

        /// Re-set the type layer at `path` with the warp and record one "Warp Text" state. False (no state) for a missing or refused layer.
        fn type_warp_commit(
            view: Pin<&mut PictureView>,
            path: &QString,
            style: &QString,
            bend: f64,
            h: f64,
            v: f64,
            vertical: bool,
        ) -> bool;
    }
}
