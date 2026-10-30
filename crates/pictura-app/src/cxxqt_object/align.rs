//! Layer > Align / Align Layers To Selection / Distribute and the Move tool's
//! align and distribute buttons. Free functions over a [`PictureView`] (their
//! own bridge, so the `PictureView` declaration list does not grow). `edge` is
//! the menu order, 0 Top … 5 Right. Each edit recomposites and records one
//! state named for the edge.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether Align applies to `paths`: two layers with content, or one when the target box exists (`to_selection` with a selection, or `align_to` Canvas).
        fn align_can(
            view: &PictureView,
            paths: &QStringList,
            to_selection: bool,
            align_to: i32,
        ) -> bool;

        /// Whether Distribute applies to `paths` (three layers with content).
        fn distribute_can(view: &PictureView, paths: &QStringList) -> bool;

        /// Align `paths` along `edge`, against `align_to` 1 Canvas's bounds, the selection's bounds when `to_selection` and one exists, else against each other. Returns how many layers moved.
        fn align_apply(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            edge: i32,
            to_selection: bool,
            align_to: i32,
        ) -> i32;

        /// Distribute `paths` along `edge`. Returns how many layers moved.
        fn distribute_apply(view: Pin<&mut PictureView>, paths: &QStringList, edge: i32) -> i32;
    }
}
