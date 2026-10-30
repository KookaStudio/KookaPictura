use super::*;

use super::super::helpers::layer_kind_str;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

pub(super) fn layer_kind_path(view: &PictureView, path: &QString) -> QString {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    let path = path.to_string();
    match pictura_render::resolve_path(doc, &path) {
        Some(layer) => layer_kind_str(doc, &path, layer),
        None => QString::default(),
    }
}

pub(super) fn layer_lock_path(view: &PictureView, path: &QString) -> i32 {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return 0;
    };
    pictura_render::resolve_path(doc, &path.to_string()).map_or(0, |layer| layer.lock.bits() as i32)
}
