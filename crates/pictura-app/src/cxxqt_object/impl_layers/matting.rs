//! `Layer > Matting` bridge: Remove Black Matte, Remove White Matte, and
//! Defringe over the active pixel layer, honouring the active selection. Free
//! functions over a [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow). Each edit recomposites and records one
//! state; a locked or empty layer is refused without change.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use imp::*;

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
        /// Recover the active layer's un-composited colour assuming a black
        /// background was composited into its alpha; honours the selection and
        /// records one "Remove Black Matte" state. False on a locked/empty
        /// layer or no single pixel layer active.
        fn layer_remove_black_matte(view: Pin<&mut PictureView>) -> bool;

        /// As [`layer_remove_black_matte`] for a white background; one
        /// "Remove White Matte" state.
        fn layer_remove_white_matte(view: Pin<&mut PictureView>) -> bool;

        /// Replace the active layer's edge-band colour with the nearest
        /// interior colour within `width` pixels; honours the selection and
        /// records one "Defringe" state.
        fn layer_defringe(view: Pin<&mut PictureView>, width: i32) -> bool;

        /// Whether the active layer can take a Matting command: an unlocked,
        /// visible pixel layer with editable colour content and a document open.
        fn matting_target_ready(view: &PictureView) -> bool;
    }
}
