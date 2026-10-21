//! Layer > Create / Release Clipping Mask and the Layers panel's Alt-click
//! toggle. Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow). Each edit recomposites and
//! records one state.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::list_of_strings;
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether any of `paths` can clip to the layer below it.
        fn clipping_can_create(view: &PictureView, paths: &QStringList) -> bool;

        /// Whether Release Clipping Mask would free anything for `paths`.
        fn clipping_can_release(view: &PictureView, paths: &QStringList) -> bool;

        /// Clip `paths` to the layers below them; one "Create Clipping Mask" state. Returns how many changed.
        fn clipping_create(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// Release `paths` (a clipped layer with the clipped layers above it, or a base with every layer clipped to it); one "Release Clipping Mask" state. Returns how many changed.
        fn clipping_release(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// The Layers panel's Alt-click on the line under `path`: release it when clipped, else clip it. False when neither applies.
        fn clipping_toggle(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}

fn any(
    view: &PictureView,
    paths: &QStringList,
    test: fn(&pictura_core::Document, &str) -> bool,
) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    list_of_strings(paths).iter().any(|p| test(doc, p))
}

fn clipping_can_create(view: &PictureView, paths: &QStringList) -> bool {
    any(view, paths, pictura_render::can_create_clipping_mask)
}

fn clipping_can_release(view: &PictureView, paths: &QStringList) -> bool {
    any(view, paths, pictura_render::can_release_clipping_mask)
}

fn clipping_create(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Create Clipping Mask",
        pictura_render::create_clipping_mask,
    )
}

fn clipping_release(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Release Clipping Mask",
        pictura_render::release_clipping_mask,
    )
}

fn clipping_toggle(view: Pin<&mut PictureView>, path: &QString) -> bool {
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
