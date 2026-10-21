//! Advanced Smart Object actions bridge: Reset Transform, Convert to Layers,
//! and New Smart Object via Copy over a [`PictureView`] row path. Free functions
//! (their own bridge, so the `PictureView` declaration list does not grow). Each
//! edit recomposites and records one state; a refusal records nothing.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
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
        /// Whether `path` is a smart object whose embedded source parses as a
        /// PSD/PSB document, so it can be reset to its native transform.
        fn layer_can_reset_smart_object_transform(view: &PictureView, path: &QString) -> bool;

        /// Reset the object at `path` to its embedded source's native frame and
        /// re-render it; one "Reset Transform" state. False on refusal.
        fn smart_object_reset_transform(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether `path` is a smart object whose embedded source parses as a
        /// PSD/PSB document, so it can be unpacked into layers.
        fn layer_can_convert_smart_object_to_layers(view: &PictureView, path: &QString) -> bool;

        /// Replace the object at `path` with the layers of its embedded source,
        /// mapped into the object's rect; one "Convert to Layers" state. False
        /// on refusal.
        fn smart_object_convert_to_layers(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether `path` is a copyable embedded smart object with a payload.
        fn layer_can_new_smart_object_via_copy(view: &PictureView, path: &QString) -> bool;

        /// Duplicate the object at `path` with an independent embedded source;
        /// one "New Smart Object via Copy" state. Returns the copy's path, or an
        /// empty string (no state) on refusal.
        fn smart_object_new_via_copy(view: Pin<&mut PictureView>, path: &QString) -> QString;
    }
}

fn layer_can_reset_smart_object_transform(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_reset_smart_object_transform(doc, &path.to_string()))
}

fn smart_object_reset_transform(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let changed = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::reset_smart_object_transform(doc, &path.to_string()),
        None => false,
    };
    if changed {
        view.as_mut().edit("Reset Transform", |_| Edit::Structural);
    }
    changed
}

fn layer_can_convert_smart_object_to_layers(view: &PictureView, path: &QString) -> bool {
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        pictura_render::can_convert_smart_object_to_layers(doc, &path.to_string())
    })
}

fn smart_object_convert_to_layers(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let edit = view.as_mut().edit("Convert to Layers", |rust| {
        let changed = {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            pictura_render::convert_smart_object_to_layers(doc, &path)
        };
        if !changed {
            return Edit::None;
        }
        if rust
            .session
            .doc
            .as_ref()
            .is_some_and(|doc| pictura_render::resolve_path(doc, &path).is_some())
        {
            rust.select_only_layer(&path);
        }
        Edit::Structural
    });
    !matches!(edit, Edit::None)
}

fn layer_can_new_smart_object_via_copy(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_new_smart_object_via_copy(doc, &path.to_string()))
}

fn smart_object_new_via_copy(mut view: Pin<&mut PictureView>, path: &QString) -> QString {
    let path = path.to_string();
    let mut created = String::new();
    view.as_mut().edit("New Smart Object via Copy", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(new_path) = pictura_render::new_smart_object_via_copy(doc, &path) else {
            return Edit::None;
        };
        created = new_path;
        rust.select_only_layer(&created);
        Edit::Structural
    });
    QString::from(created.as_str())
}
