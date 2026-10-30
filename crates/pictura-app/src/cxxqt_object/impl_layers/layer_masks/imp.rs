use super::*;

use super::super::super::helpers_composite::selection_to_mask;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_render::LayerMaskKind;

pub(super) fn kind_from(name: &str) -> Option<LayerMaskKind> {
    match name {
        "reveal-all" => Some(LayerMaskKind::RevealAll),
        "hide-all" => Some(LayerMaskKind::HideAll),
        "reveal-selection" => Some(LayerMaskKind::RevealSelection),
        "hide-selection" => Some(LayerMaskKind::HideSelection),
        "from-transparency" => Some(LayerMaskKind::FromTransparency),
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

pub(super) fn layer_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_add_path(view, &QString::from(path.as_str()), kind)
}

pub(super) fn layer_mask_add_path(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    kind: &QString,
) -> bool {
    let Some(kind) = kind_from(&kind.to_string()) else {
        return false;
    };
    let path = path.to_string();
    let selection = {
        let rust = view.rust();
        rust.session.doc.as_ref().and_then(|doc| {
            rust.session
                .selection
                .as_ref()
                .map(|s| selection_to_mask(s, doc))
        })
    };
    let changed = view
        .as_mut()
        .batch_changed(&one(&path), "Add Layer Mask", |doc, paths| {
            let mut changed = 0;
            for &path in paths {
                // Add first, then convert: a refused add (an existing mask, or a
                // reveal-selection with no selection) must not convert the Background.
                if pictura_render::add_layer_mask_converting_background(
                    doc,
                    path,
                    kind,
                    selection.as_ref(),
                ) {
                    changed += 1;
                }
            }
            changed
        });
    if changed > 0 {
        // A freshly added mask becomes the edit target, as CS6 selects its
        // thumbnail so the brush, fill, and filters write coverage at once.
        view.as_mut().rust_mut().mask_edit_target = Some(path);
    }
    changed > 0
}

pub(super) fn layer_mask_delete(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_delete_path(view, &QString::from(path.as_str()))
}

pub(super) fn layer_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = view.batch_changed(&one(&path), "Delete Layer Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::delete_layer_mask(doc, path) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn layer_mask_apply(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_apply_path(view, &QString::from(path.as_str()))
}

pub(super) fn layer_mask_apply_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = view.batch_changed(&one(&path), "Apply Layer Mask", |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::apply_layer_mask(doc, path) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn layer_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_set_enabled_path(view, &QString::from(path.as_str()), enabled)
}

pub(super) fn layer_mask_set_enabled_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    enabled: bool,
) -> bool {
    let path = path.to_string();
    let label = if enabled {
        "Enable Layer Mask"
    } else {
        "Disable Layer Mask"
    };
    let changed = view.batch_changed(&one(&path), label, move |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::set_layer_mask_enabled(doc, path, enabled) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn layer_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_set_linked_path(view, &QString::from(path.as_str()), linked)
}

pub(super) fn layer_mask_set_linked_path(
    view: Pin<&mut PictureView>,
    path: &QString,
    linked: bool,
) -> bool {
    let path = path.to_string();
    let label = if linked {
        "Link Layer Mask"
    } else {
        "Unlink Layer Mask"
    };
    let changed = view.batch_changed(&one(&path), label, move |doc, paths| {
        let mut changed = 0;
        for &path in paths {
            if pictura_render::set_layer_mask_linked(doc, path, linked) {
                changed += 1;
            }
        }
        changed
    });
    changed > 0
}

pub(super) fn layer_mask_present(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path))
    })
}

pub(super) fn layer_mask_present_path(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path.to_string()))
}

pub(super) fn layer_mask_disabled_path(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::layer_mask_disabled(doc, &path.to_string()))
}

pub(super) fn mask_edit_target_set(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    if path.is_empty() {
        view.as_mut().rust_mut().mask_edit_target = None;
        return true;
    }
    let resolves = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    if !resolves {
        return false;
    }
    view.as_mut().rust_mut().mask_edit_target = Some(path);
    true
}

pub(super) fn mask_edit_target_get(mut view: Pin<&mut PictureView>) -> QString {
    let Some(path) = view.rust().mask_edit_target.clone() else {
        return QString::default();
    };
    let resolves = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    if !resolves {
        view.as_mut().rust_mut().mask_edit_target = None;
        return QString::default();
    }
    QString::from(path)
}

pub(super) fn mask_solo_set(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    if path.is_empty() && view.rust().mask_solo.is_none() {
        return false;
    }
    if !path.is_empty() {
        let resolves = view
            .rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
        if !resolves {
            return false;
        }
    }
    {
        let mut rust = view.as_mut().rust_mut();
        rust.mask_solo = (!path.is_empty()).then_some(path);
        rust.canvas_revision = rust.canvas_revision.wrapping_add(1);
        rust.frame_revision = rust.frame_revision.wrapping_add(1);
    }
    view.as_mut().rust_mut().rebuild_mask_solo_source();
    view.as_mut().changed();
    true
}

pub(super) fn mask_solo_get(mut view: Pin<&mut PictureView>) -> QString {
    let Some(path) = view.rust().mask_solo.clone() else {
        return QString::default();
    };
    let resolves = view
        .rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    if !resolves {
        let mut rust = view.as_mut().rust_mut();
        rust.mask_solo = None;
        rust.drop_mask_solo_source();
        return QString::default();
    }
    QString::from(path)
}

pub(super) fn layer_mask_linked(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::layer_mask_linked(doc, &path))
    })
}
