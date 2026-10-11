//! Path-addressed layer kind/lock queries. Free functions over a
//! [`PictureView`] with their own bridge, so the `PictureView` declaration list
//! does not grow (matching [`align`](super::align)).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::layer_kind_str;
use super::qobject::PictureView;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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
        /// Kind of the layer at `path` (`"0/1"` walks into node 0), or empty.
        fn layer_kind_path(view: &PictureView, path: &QString) -> QString;

        /// Lock flags of the layer at `path` as a bitmask, or 0 when unresolved.
        fn layer_lock_path(view: &PictureView, path: &QString) -> i32;
    }
}

fn layer_kind_path(view: &PictureView, path: &QString) -> QString {
    let Some(doc) = view.rust().doc.as_ref() else {
        return QString::default();
    };
    let path = path.to_string();
    match pictura_render::resolve_path(doc, &path) {
        Some(layer) => layer_kind_str(doc, &path, layer),
        None => QString::default(),
    }
}

fn layer_lock_path(view: &PictureView, path: &QString) -> i32 {
    let Some(doc) = view.rust().doc.as_ref() else {
        return 0;
    };
    pictura_render::resolve_path(doc, &path.to_string()).map_or(0, |layer| layer.lock.bits() as i32)
}
