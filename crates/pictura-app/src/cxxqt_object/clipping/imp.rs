use super::*;

use super::super::helpers::list_of_strings;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

pub(super) fn any(
    view: &PictureView,
    paths: &QStringList,
    test: fn(&pictura_core::Document, &str) -> bool,
) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    list_of_strings(paths).iter().any(|p| test(doc, p))
}

pub(super) fn clipping_can_create(view: &PictureView, paths: &QStringList) -> bool {
    any(view, paths, pictura_render::can_create_clipping_mask)
}

pub(super) fn clipping_can_release(view: &PictureView, paths: &QStringList) -> bool {
    any(view, paths, pictura_render::can_release_clipping_mask)
}

pub(super) fn clipping_create(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Create Clipping Mask",
        pictura_render::create_clipping_mask,
    )
}

pub(super) fn clipping_release(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Release Clipping Mask",
        pictura_render::release_clipping_mask,
    )
}

pub(super) fn clipping_toggle(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let clipped = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path))
        .is_some_and(|layer| layer.clipping);
    let mut paths = QStringList::default();
    paths.append(QString::from(path.as_str()));
    let changed = if clipped {
        clipping_release(view, &paths)
    } else {
        clipping_create(view, &paths)
    };
    changed > 0
}
