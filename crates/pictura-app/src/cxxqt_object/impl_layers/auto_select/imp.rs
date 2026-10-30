use super::*;

use super::super::super::helpers_composite::{auto_select_path, layers_under_point};
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

pub(super) fn auto_select_layer(view: &PictureView, x: i32, y: i32, group: bool) -> QString {
    let rust = view.rust();
    rust.session
        .doc
        .as_ref()
        .and_then(|doc| auto_select_path(doc, x, y, group, rust.isolation.as_deref()))
        .map(QString::from)
        .unwrap_or_default()
}

pub(super) fn layers_under_layer(view: &PictureView, x: i32, y: i32) -> QStringList {
    let rust = view.rust();
    rust.session
        .doc
        .as_ref()
        .map(|doc| {
            layers_under_point(doc, x, y, rust.isolation.as_deref())
                .into_iter()
                .map(QString::from)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn layer_name_for_path(view: &PictureView, path: &QString) -> QString {
    let rust = view.rust();
    rust.session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
        .map(|layer| QString::from(layer.name.as_str()))
        .unwrap_or_default()
}
