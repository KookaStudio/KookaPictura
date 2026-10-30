//! Image ▸ Duplicate / Trim. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). Ported from
//! photorust's `Image` menu handlers.
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
        /// Copy `source`'s document into `target` as a new tab. `merged` flattens
        /// the layer stack to one layer (Image ▸ Duplicate's "Duplicate Merged
        /// Layers Only"); otherwise the whole stack is copied. One "Duplicate"
        /// state on the target; false without a source document.
        fn duplicate_into(
            source: &PictureView,
            target: Pin<&mut PictureView>,
            name: &QString,
            merged: bool,
        ) -> bool;

        /// `Image ▸ Trim`: crop the canvas to the bounds of the visible content.
        /// One "Trim" state; false when the content already fills the canvas.
        fn trim_image(view: Pin<&mut PictureView>) -> bool;

        /// The name Image ▸ Duplicate suggests: the source file's base name plus
        /// " copy".
        fn duplicate_name(view: &PictureView) -> QString;

        /// `Image ▸ Image Rotation ▸ Arbitrary`: rotate the document about its
        /// centre by `angle_deg` degrees clockwise, recomposite, and record one
        /// "Rotate" state. False for a non-finite or zero angle, an out-of-range
        /// angle, or without a document.
        fn rotate_document_arbitrary(view: Pin<&mut PictureView>, angle_deg: f64) -> bool;
    }
}
