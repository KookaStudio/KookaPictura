//! Layer > Arrange / Reverse and the non-destructive Stamp Visible / Stamp
//! Selected. Free functions over a [`PictureView`] (their own bridge, so the
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
        /// Whether `arrange` (`0` Front, `1` Forward, `2` Backward, `3` Back)
        /// would reorder the layer at `path`.
        fn arrange_layer_can(view: &PictureView, path: &QString, arrange: i32) -> bool;

        /// Whether a delta move would change the order; no history.
        fn can_move_layer_path(view: &PictureView, path: &QString, delta: i32) -> bool;

        /// Whether `path` can become the Background; no history.
        fn can_background_from_layer(view: &PictureView, path: &QString) -> bool;

        /// Apply `arrange` to `path` within its container; one "Arrange Layer"
        /// state when the order changes.
        fn arrange_layer_run(view: Pin<&mut PictureView>, path: &QString, arrange: i32) -> bool;

        /// Reverse the selected contiguous run; one "Reverse Layers" state when
        /// the order changes.
        fn reverse_layers_run(view: Pin<&mut PictureView>, paths: &QStringList) -> bool;

        /// Composite the eye-visible layers into a NEW layer directly above
        /// `path`, leaving the originals. One "Stamp Visible" state. Returns
        /// the new path, or empty on refusal.
        fn stamp_visible_run(view: Pin<&mut PictureView>, path: &QString) -> QString;

        /// Composite the selected layers into a NEW layer directly above
        /// `active`, leaving the originals. One "Stamp Selected" state. Returns
        /// the new path, or empty on refusal.
        fn stamp_selected_run(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            active: &QString,
        ) -> QString;
    }
}
