//! `File ▸ Export As…` with the SVG format: build the document's SVG text. Free
//! functions over a [`PictureView`] (their own bridge). SVG is text, so it does
//! not go through the Qt `QImageWriter` edge; the dialog writes the returned
//! string.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub use imp::*;

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
        /// The active document serialized to SVG text, or empty without one.
        /// Never changes the document, its path, dirty flag, or history.
        fn svg_from_view(view: &PictureView) -> QString;
    }
}
