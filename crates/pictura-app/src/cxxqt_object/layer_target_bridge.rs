//! Bridge for the document layer target. Its own bridge keeps the root
//! `PictureView` declaration list and its file-size ceiling intact.
//!
//! [`PictureView`]: super::qobject::PictureView

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
        /// Write the panel's selection as the document target. Silent: the
        /// panel already shows this selection.
        fn set_layer_target(view: Pin<&mut PictureView>, paths: &QStringList, anchor: &QString);

        /// The selected layer paths, in selection order.
        fn selected_layer_paths(view: &PictureView) -> QStringList;

        /// The panel's current row, or empty.
        fn layer_selection_anchor(view: &PictureView) -> QString;

        /// Replace the target with one layer and notify the panel. Used by
        /// tools, commands, and layer creation.
        fn select_layer_path(view: Pin<&mut PictureView>, path: &QString);

        /// Clear the target and notify the panel (Select > Deselect Layers).
        fn clear_layer_target(view: Pin<&mut PictureView>);
    }
}

fn set_layer_target(mut view: Pin<&mut PictureView>, paths: &QStringList, anchor: &QString) {
    let owned: Vec<String> = paths.iter().map(ToString::to_string).collect();
    let anchor = (!anchor.is_empty()).then(|| anchor.to_string());
    let rust = &mut *view.as_mut().rust_mut();
    let mut target = rust.layer_target.clone();
    target.select(owned, anchor, rust.doc.as_ref());
    if target == rust.layer_target {
        return;
    }
    rust.set_layer_target(target.clone());
    rust.history.set_current_target(&target);
}

fn selected_layer_paths(view: &PictureView) -> QStringList {
    view.rust()
        .layer_target
        .selected()
        .iter()
        .map(|path| QString::from(path.as_str()))
        .collect()
}

fn layer_selection_anchor(view: &PictureView) -> QString {
    view.rust()
        .layer_target
        .anchor()
        .map(QString::from)
        .unwrap_or_default()
}

fn select_layer_path(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    {
        let rust = &mut *view.as_mut().rust_mut();
        let doc = rust.doc.as_ref();
        rust.layer_target
            .select(std::iter::once(path.clone()), Some(path), doc);
        let anchor = rust.layer_target.anchor().map(str::to_string);
        if rust.mask_edit_target.as_deref() != anchor.as_deref() {
            rust.mask_edit_target = None;
        }
        let target = rust.layer_target.clone();
        rust.history.set_current_target(&target);
    }
    view.as_mut().changed();
}

fn clear_layer_target(mut view: Pin<&mut PictureView>) {
    {
        let rust = &mut *view.as_mut().rust_mut();
        rust.layer_target.clear();
        rust.mask_edit_target = None;
        let target = rust.layer_target.clone();
        rust.history.set_current_target(&target);
    }
    view.as_mut().changed();
}
