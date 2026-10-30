//! Layers-panel surface bridge: the CS6 Panel Options that gate duplicate
//! naming and default fill masks, the `Layer > Lock All Layers In Group…`
//! batch, and whether a selection can be deleted.
//!
//! Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list in `cxxqt_object.rs` stays at its size
//! ceiling). Each lock edit recomposites and records one state.
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
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {

        /// Apply the CS6 Layers Panel Options that gate bridge behaviour:
        /// `add_copy` (name a duplicate `"<name> copy"`), `use_default_masks`
        /// (a fill layer created with a selection takes it as a mask), and
        /// `add_mask_by_default` (a new adjustment layer gets a mask).
        fn set_layers_panel_options(
            view: Pin<&mut PictureView>,
            add_copy: bool,
            use_default_masks: bool,
            add_mask_by_default: bool,
        );

        /// Apply the full lock set to every descendant of the group at
        /// `group_path`; one "Lock Group Layers" state. Returns the number of
        /// layers changed, `0` when `group_path` is not a group.
        fn lock_group_layers_run(view: Pin<&mut PictureView>, group_path: &QString) -> i32;

        /// Whether deleting `paths` would remove anything: false for an empty
        /// selection, the Background, fully-locked layers, and the document's
        /// last layer, all of which Delete Layer refuses.
        fn layers_can_delete(view: &PictureView, paths: &QStringList) -> bool;
    }
}
