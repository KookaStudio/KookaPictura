//! The Move tool's Auto-Select hit test. A free function over a
//! [`PictureView`] (its own bridge, so the `PictureView` declaration list does
//! not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::super::helpers_composite::{auto_select_path, layers_under_point};
use super::super::qobject::PictureView;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

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

fn auto_select_layer(view: &PictureView, x: i32, y: i32, group: bool) -> QString {
    let rust = view.rust();
    rust.doc
        .as_ref()
        .and_then(|doc| auto_select_path(doc, x, y, group, rust.isolation.as_deref()))
        .map(QString::from)
        .unwrap_or_default()
}

fn layers_under_layer(view: &PictureView, x: i32, y: i32) -> QStringList {
    let rust = view.rust();
    rust.doc
        .as_ref()
        .map(|doc| {
            layers_under_point(doc, x, y, rust.isolation.as_deref())
                .into_iter()
                .map(QString::from)
                .collect()
        })
        .unwrap_or_default()
}

fn layer_name_for_path(view: &PictureView, path: &QString) -> QString {
    let rust = view.rust();
    rust.doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
        .map(|layer| QString::from(layer.name.as_str()))
        .unwrap_or_default()
}
