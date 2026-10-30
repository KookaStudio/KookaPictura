use super::*;

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_render::VectorMaskKind;

pub(super) fn vector_mask_present_path(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_vector_mask(doc, &path.to_string()))
}

pub(super) fn vector_mask_disabled_path(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::vector_mask_disabled(doc, &path.to_string()))
}

pub(super) fn kind_from(name: &str) -> Option<VectorMaskKind> {
    match name {
        "reveal-all" => Some(VectorMaskKind::RevealAll),
        "hide-all" => Some(VectorMaskKind::HideAll),
        "current-path" => Some(VectorMaskKind::CurrentPath),
        _ => None,
    }
}

pub(super) fn active_path(view: &PictureView) -> Option<String> {
    view.rust().layer_target.anchor().map(str::to_string)
}

pub(super) fn one(path: &str) -> QStringList {
    let mut paths = QStringList::default();
    paths.append(QString::from(path));
    paths
}

pub(super) fn vector_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    let Some(kind) = kind_from(&kind.to_string()) else {
        return false;
    };
    let Some(path) = active_path(&view) else {
        return false;
    };
    let changed = view.batch_changed(&one(&path), "Add Vector Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::add_vector_mask(doc, path, kind) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_add_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    kind: &QString,
) -> bool {
    let Some(kind) = kind_from(&kind.to_string()) else {
        return false;
    };
    let path = path.to_string();
    let changed = view.batch_changed(&one(&path), "Add Vector Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::add_vector_mask(doc, path, kind) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_delete(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_delete_path(view, &QString::from(path.as_str()))
}

pub(super) fn vector_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = view.batch_changed(&one(&path), "Delete Vector Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::delete_vector_mask(doc, path) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_set_enabled_path(view, &QString::from(path.as_str()), enabled)
}

pub(super) fn vector_mask_set_enabled_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    enabled: bool,
) -> bool {
    let path = path.to_string();
    let label = if enabled {
        "Enable Vector Mask"
    } else {
        "Disable Vector Mask"
    };
    let changed = view.batch_changed(&one(&path), label, move |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::set_vector_mask_enabled(doc, path, enabled) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_set_linked_path(view, &QString::from(path.as_str()), linked)
}

pub(super) fn vector_mask_set_linked_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    linked: bool,
) -> bool {
    let path = path.to_string();
    let label = if linked {
        "Link Vector Mask"
    } else {
        "Unlink Vector Mask"
    };
    let changed = view.batch_changed(&one(&path), label, move |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::set_vector_mask_linked(doc, path, linked) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_rasterize(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_rasterize_path(view, &QString::from(path.as_str()))
}

pub(super) fn vector_mask_rasterize_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = view.batch_changed(&one(&path), "Rasterize Vector Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::rasterize_vector_mask(doc, path) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn vector_mask_present(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::has_vector_mask(doc, &path))
    })
}

pub(super) fn vector_mask_linked(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::vector_mask_linked(doc, &path))
    })
}

pub(super) fn vector_mask_disabled(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::vector_mask_disabled(doc, &path))
    })
}
