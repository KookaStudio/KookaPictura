use super::*;

use crate::cxxqt_object::paths::{active_in, ActivePath};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::path::Subpath;
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

/// The Paths panel's selected path, a saved path or the Work Path, as Current
/// Path authors it; `None` when no such row is selected or it is empty.
pub(super) fn current_path(view: &PictureView) -> Option<Vec<Subpath>> {
    let rust = view.rust();
    let doc = rust.session.doc.as_ref()?;
    let path = match active_in(doc, rust.active_path) {
        ActivePath::Work => &doc.work_path,
        ActivePath::Saved(i) => &doc.saved_paths[i].path,
        ActivePath::None | ActivePath::VectorMask => return None,
    };
    (!path.is_empty()).then(|| path.subpaths.clone())
}

pub(super) fn vector_mask_current_path_available(view: &PictureView) -> bool {
    current_path(view).is_some()
}

pub(super) fn active_path(view: &PictureView) -> Option<String> {
    view.rust().layer_target.anchor().map(str::to_string)
}

pub(super) fn one(path: &str) -> QStringList {
    let mut paths = QStringList::default();
    paths.append(QString::from(path));
    paths
}

/// Add a vector mask of `kind` (`"reveal-all"`, `"hide-all"`,
/// `"current-path"`) to the layer at `path`.
fn add(view: Pin<&mut PictureView>, path: &str, kind: &QString) -> bool {
    let outline;
    let kind = match kind.to_string().as_str() {
        "reveal-all" => VectorMaskKind::RevealAll,
        "hide-all" => VectorMaskKind::HideAll,
        "current-path" => {
            let Some(selected) = current_path(&view) else {
                return false;
            };
            outline = selected;
            VectorMaskKind::Path(&outline)
        }
        _ => return false,
    };
    let changed = view.batch_changed(&one(path), "Add Vector Mask", |doc, paths| {
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

pub(super) fn vector_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    add(view, &path, kind)
}

pub(super) fn vector_mask_add_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    kind: &QString,
) -> bool {
    add(view, &path.to_string(), kind)
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
