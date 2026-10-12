//! Layer > Vector Mask bridge: add (by kind), delete, enable/disable,
//! link/unlink, and rasterize, plus presence/link/disabled reads, over the
//! active layer. Free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). Each edit recomposites
//! and records one state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_render::VectorMaskKind;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Add a vector mask of `kind` (`"reveal-all"`, `"hide-all"`,
        /// `"current-path"`) to the active layer; one "Add Vector Mask" state.
        /// False when the kind is unknown, the layer already has a vector mask,
        /// or Current Path has no work path.
        fn vector_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool;

        /// Delete the active layer's vector mask; one "Delete Vector Mask"
        /// state. False without a vector mask.
        fn vector_mask_delete(view: Pin<&mut PictureView>) -> bool;

        /// Set the active layer's vector mask enabled; one
        /// "Enable/Disable Vector Mask" state. False without a mask or no change.
        fn vector_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool;

        /// Set the active layer's vector mask linked; one "Link/Unlink Vector
        /// Mask" state. False without a mask or no change.
        fn vector_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool;

        /// Set the vector mask enabled on the layer at `path`; one undoable
        /// state. The row-click contract edits the clicked row, not the active
        /// layer.
        fn vector_mask_set_enabled_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            enabled: bool,
        ) -> bool;

        /// Set the vector mask linked on the layer at `path`; one undoable state.
        fn vector_mask_set_linked_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            linked: bool,
        ) -> bool;

        /// Convert the active layer's vector mask to a layer mask; one
        /// "Rasterize Vector Mask" state. False without a vector mask.
        fn vector_mask_rasterize(view: Pin<&mut PictureView>) -> bool;

        /// Whether the active layer has a vector mask.
        fn vector_mask_present(view: &PictureView) -> bool;

        /// Whether the active layer's vector mask is linked to the layer.
        fn vector_mask_linked(view: &PictureView) -> bool;

        /// Whether the active layer's vector mask is disabled.
        fn vector_mask_disabled(view: &PictureView) -> bool;

    }
}

fn kind_from(name: &str) -> Option<VectorMaskKind> {
    match name {
        "reveal-all" => Some(VectorMaskKind::RevealAll),
        "hide-all" => Some(VectorMaskKind::HideAll),
        "current-path" => Some(VectorMaskKind::CurrentPath),
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

fn vector_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool {
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

fn vector_mask_delete(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
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

fn vector_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_set_enabled_path(view, &QString::from(path.as_str()), enabled)
}

fn vector_mask_set_enabled_path(
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

fn vector_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
    vector_mask_set_linked_path(view, &QString::from(path.as_str()), linked)
}

fn vector_mask_set_linked_path(view: Pin<&mut PictureView>, path: &QString, linked: bool) -> bool {
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

fn vector_mask_rasterize(view: Pin<&mut PictureView>) -> bool {
    let Some(path) = active_path(&view) else {
        return false;
    };
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

fn vector_mask_present(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::has_vector_mask(doc, &path))
    })
}

fn vector_mask_linked(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::vector_mask_linked(doc, &path))
    })
}

fn vector_mask_disabled(view: &PictureView) -> bool {
    active_path(view).is_some_and(|path| {
        view.rust()
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::vector_mask_disabled(doc, &path))
    })
}
