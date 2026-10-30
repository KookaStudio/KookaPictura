//! Layer > Create / Release Clipping Mask and the Layers panel's Alt-click
//! toggle. Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow). Each edit recomposites and
//! records one state.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether any of `paths` can clip to the layer below it.
        fn clipping_can_create(view: &PictureView, paths: &QStringList) -> bool;

        /// Whether Release Clipping Mask would free anything for `paths`.
        fn clipping_can_release(view: &PictureView, paths: &QStringList) -> bool;

        /// Clip `paths` to the layers below them; one "Create Clipping Mask" state. Returns how many changed.
        fn clipping_create(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// Release `paths` (a clipped layer with the clipped layers above it, or a base with every layer clipped to it); one "Release Clipping Mask" state. Returns how many changed.
        fn clipping_release(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// The Layers panel's Alt-click on the line under `path`: release it when clipped, else clip it. False when neither applies.
        fn clipping_toggle(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}
