//! The Move tool's Auto-Select hit test. A free function over a
//! [`PictureView`] (its own bridge, so the `PictureView` declaration list does
//! not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

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
        /// The panel path of the topmost visible pixel layer whose content
        /// covers document `(x, y)`, or empty when none does. `group` resolves
        /// a nested hit to its top-level ancestor.
        fn auto_select_layer(view: &PictureView, x: i32, y: i32, group: bool) -> QString;

        /// Every layer path with a pixel under document `(x, y)`, topmost
        /// first; empty when none. Scoped to the active isolation set.
        fn layers_under_layer(view: &PictureView, x: i32, y: i32) -> QStringList;

        /// The display name of the layer at `path`, or empty when no layer is
        /// there.
        fn layer_name_for_path(view: &PictureView, path: &QString) -> QString;
    }
}
