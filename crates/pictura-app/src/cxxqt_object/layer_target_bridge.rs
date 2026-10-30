//! Bridge for the document layer target. Its own bridge keeps the root
//! `PictureView` declaration list and its file-size ceiling intact.
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
        /// Write the panel's selection as the document target. Silent: the
        /// panel already shows this selection.
        fn set_layer_target(view: Pin<&mut PictureView>, paths: &QStringList, anchor: &QString);

        /// The selected layer paths, in selection order.
        fn selected_layer_paths(view: &PictureView) -> QStringList;

        /// The panel's current row, or empty.
        fn layer_selection_anchor(view: &PictureView) -> QString;

        /// Replace the target with one layer and notify the panel. Used by
        /// tools, commands, and layer creation.
        fn select_layer_path(view: Pin<&mut PictureView>, path: &QString);

        /// Clear the target and notify the panel (Select > Deselect Layers).
        fn clear_layer_target(view: Pin<&mut PictureView>);

        /// Snapshot every layer's visibility and show only `path` and its
        /// ancestors (and a group's visible descendants) as one undo step.
        fn solo_set(view: Pin<&mut PictureView>, path: &QString);

        /// Restore the visibility snapshot and drop it as one undo step.
        fn solo_restore(view: Pin<&mut PictureView>);

        /// Drop the solo snapshot without touching visibility.
        fn solo_clear(view: Pin<&mut PictureView>);

        /// Whether this document has a solo snapshot.
        fn solo_active(view: &PictureView) -> bool;

        /// Replace this document's expanded group paths.
        fn expansion_set(view: Pin<&mut PictureView>, paths: &QStringList);

        /// This document's expanded group paths.
        fn expansion_paths(view: &PictureView) -> QStringList;
    }
}
