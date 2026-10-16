//! Layer > Layer Mask bridge: add (by kind), delete, apply, enable/disable, and
//! link, plus presence/link/disabled reads, over the active layer, and per-row
//! disabled/link reads. Free functions over a [`PictureView`] (their own bridge,
//! so the `PictureView` declaration list does not grow). Each edit recomposites
//! and records one state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::helpers_composite::selection_to_mask;
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_render::LayerMaskKind;

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
        /// Add a mask of `kind` (`"reveal-all"`, `"hide-all"`, `"reveal-selection"`,
        /// `"hide-selection"`, `"from-transparency"`) to the active layer; one "Add Layer Mask" state. False when the kind is unknown or the layer already has a mask.
        fn layer_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool;

        /// Delete the active layer's mask; one "Delete Layer Mask" state. False without a mask.
        fn layer_mask_delete(view: Pin<&mut PictureView>) -> bool;

        /// Fold the active layer's mask into its alpha and clear it; one "Apply Layer Mask" state. False without a mask or on a smart object.
        fn layer_mask_apply(view: Pin<&mut PictureView>) -> bool;

        /// Fold the mask on the layer at `path` into its alpha and clear it;
        /// one "Apply Layer Mask" state. False without a mask or on a smart
        /// object. The mask-thumbnail menu acts on the clicked row, not the
        /// active layer.
        fn layer_mask_apply_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Set the active layer's mask enabled; one "Enable/Disable Layer Mask" state. False without a mask or no change.
        fn layer_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool;

        /// Set the active layer's mask linked to the layer; one "Link/Unlink Layer Mask" state. False without a mask or no change.
        fn layer_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool;

        /// Add a mask of `kind` to the layer at `path`; one "Add Layer Mask"
        /// state. The row-click contract edits the clicked row, not the active
        /// layer.
        fn layer_mask_add_path(view: Pin<&mut PictureView>, path: &QString, kind: &QString)
            -> bool;

        /// Delete the mask on the layer at `path`; one "Delete Layer Mask" state.
        fn layer_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Set the mask enabled on the layer at `path`; one undoable state.
        fn layer_mask_set_enabled_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            enabled: bool,
        ) -> bool;

        /// Set the mask linked on the layer at `path`; one undoable state.
        fn layer_mask_set_linked_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            linked: bool,
        ) -> bool;

        /// Whether the active layer has a raster layer mask.
        fn layer_mask_present(view: &PictureView) -> bool;

        /// Make the raster mask on the layer at `path` the view's mask edit
        /// target, so the brush, fill, and filters write its coverage; an empty
        /// path clears the target. False without a mask on that layer. Records
        /// no history.
        fn mask_edit_target_set(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// The path of the layer whose raster mask is the edit target, empty
        /// when none. Clears a target whose layer, mask, or active-layer status
        /// no longer holds.
        fn mask_edit_target_get(view: Pin<&mut PictureView>) -> QString;

        /// Show the raster mask on the layer at `path` alone on the canvas, or
        /// restore the composite when `path` is empty. View-only: records no
        /// history and leaves the document unchanged. False when `path` has no
        /// raster mask, or when clearing an already-clear solo.
        fn mask_solo_set(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// The path whose raster mask is shown alone, empty when none. Clears a
        /// solo whose mask is gone.
        fn mask_solo_get(view: Pin<&mut PictureView>) -> QString;

        /// Whether the active layer's mask is linked to the layer.
        fn layer_mask_linked(view: &PictureView) -> bool;

    }
}

fn kind_from(name: &str) -> Option<LayerMaskKind> {
    match name {
        "reveal-all" => Some(LayerMaskKind::RevealAll),
        "hide-all" => Some(LayerMaskKind::HideAll),
        "reveal-selection" => Some(LayerMaskKind::RevealSelection),
        "hide-selection" => Some(LayerMaskKind::HideSelection),
        "from-transparency" => Some(LayerMaskKind::FromTransparency),
        _ => None,
    }
}

fn active_path(view: &PictureView) -> Option<String> {
    view.rust().active_layer.clone()
}

fn one(path: &str) -> QStringList {
    let mut paths = QStringList::default();
    paths.append(QString::from(path));
    paths
}

fn layer_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_add_path(view, &QString::from(path.as_str()), kind)
}

fn layer_mask_add_path(mut view: Pin<&mut PictureView>, path: &QString, kind: &QString) -> bool {
    let Some(kind) = kind_from(&kind.to_string()) else {
        return false;
    };
    let path = path.to_string();
    let selection = {
        let rust = view.rust();
        rust.doc
            .as_ref()
            .and_then(|doc| rust.selection.as_ref().map(|s| selection_to_mask(s, doc)))
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

fn layer_mask_delete(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_delete_path(view, &QString::from(path.as_str()))
}

fn layer_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
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

fn layer_mask_apply(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_apply_path(view, &QString::from(path.as_str()))
}

fn layer_mask_apply_path(view: Pin<&mut PictureView>, path: &QString) -> bool {
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

fn layer_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_set_enabled_path(view, &QString::from(path.as_str()), enabled)
}

fn layer_mask_set_enabled_path(view: Pin<&mut PictureView>, path: &QString, enabled: bool) -> bool {
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

fn layer_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    layer_mask_set_linked_path(view, &QString::from(path.as_str()), linked)
}

fn layer_mask_set_linked_path(view: Pin<&mut PictureView>, path: &QString, linked: bool) -> bool {
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

fn layer_mask_present(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path))
    })
}

fn mask_edit_target_set(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    if path.is_empty() {
        view.as_mut().rust_mut().mask_edit_target = None;
        return true;
    }
    let resolves = view
        .rust()
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    if !resolves {
        return false;
    }
    view.as_mut().rust_mut().mask_edit_target = Some(path);
    true
}

fn mask_edit_target_get(mut view: Pin<&mut PictureView>) -> QString {
    let Some(path) = view.rust().mask_edit_target.clone() else {
        return QString::default();
    };
    let resolves = view
        .rust()
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::has_layer_mask(doc, &path));
    if !resolves {
        view.as_mut().rust_mut().mask_edit_target = None;
        return QString::default();
    }
    QString::from(path)
}

fn mask_solo_set(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    if path.is_empty() && view.rust().mask_solo.is_none() {
        return false;
    }
    if !path.is_empty() {
        let resolves = view
            .rust()
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

fn mask_solo_get(mut view: Pin<&mut PictureView>) -> QString {
    let Some(path) = view.rust().mask_solo.clone() else {
        return QString::default();
    };
    let resolves = view
        .rust()
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

fn layer_mask_linked(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::layer_mask_linked(doc, &path))
    })
}
