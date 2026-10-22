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

        /// Snapshot every layer's visibility and show only `path` and its
        /// ancestors (and a group's visible descendants) as one undo step.
        fn solo_set(view: Pin<&mut PictureView>, path: &QString);

        /// Restore the visibility snapshot and drop it as one undo step.
        fn solo_restore(view: Pin<&mut PictureView>);

        /// Drop the solo snapshot without touching visibility.
        fn solo_clear(view: Pin<&mut PictureView>);

        /// Whether this document has a solo snapshot.
        fn solo_active(view: &PictureView) -> bool;

        /// Replace this document's expanded group paths.
        fn expansion_set(view: Pin<&mut PictureView>, paths: &QStringList);

        /// This document's expanded group paths.
        fn expansion_paths(view: &PictureView) -> QStringList;
    }
}

fn set_layer_target(mut view: Pin<&mut PictureView>, paths: &QStringList, anchor: &QString) {
    let owned: Vec<String> = paths.iter().map(ToString::to_string).collect();
    let anchor = (!anchor.is_empty()).then(|| anchor.to_string());
    let rust = &mut *view.as_mut().rust_mut();
    let mut target = rust.layer_target.clone();
    target.select(owned, anchor, rust.session.doc.as_ref());
    if target == rust.layer_target {
        return;
    }
    rust.set_layer_target(target.clone());
    rust.session.history.set_current_target(&target);
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
        let doc = rust.session.doc.as_ref();
        rust.layer_target
            .select(std::iter::once(path.clone()), Some(path), doc);
        let anchor = rust.layer_target.anchor().map(str::to_string);
        if rust.mask_edit_target.as_deref() != anchor.as_deref() {
            rust.mask_edit_target = None;
        }
        let target = rust.layer_target.clone();
        rust.session.history.set_current_target(&target);
    }
    view.as_mut().changed();
}

fn clear_layer_target(mut view: Pin<&mut PictureView>) {
    {
        let rust = &mut *view.as_mut().rust_mut();
        rust.layer_target.clear();
        rust.mask_edit_target = None;
        let target = rust.layer_target.clone();
        rust.session.history.set_current_target(&target);
    }
    view.as_mut().changed();
}

fn solo_set(mut view: Pin<&mut PictureView>, path: &QString) {
    let path = path.to_string();
    let (snapshot, paths) = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return;
        };
        if pictura_render::resolve_path(doc, &path).is_none() {
            return;
        }
        let snapshot: std::collections::HashMap<String, bool> = pictura_render::flatten_rows(doc)
            .into_iter()
            .map(|(candidate, _)| {
                let visible = pictura_render::resolve_path(doc, &candidate)
                    .is_some_and(|layer| layer.visible);
                (candidate, visible)
            })
            .collect();
        if snapshot.is_empty() {
            return;
        }
        let paths = solo_paths(doc, &path, &snapshot);
        (snapshot, paths)
    };
    {
        let rust = &mut *view.as_mut().rust_mut();
        rust.solo_snapshot = Some(snapshot);
    }
    let paths: QStringList = paths.into_iter().map(QString::from).collect();
    view.as_mut()
        .apply_visibility(&paths, &QString::from("Solo Visibility"));
}

fn solo_restore(mut view: Pin<&mut PictureView>) {
    let visible: Option<Vec<String>> = {
        let rust = view.rust();
        rust.solo_snapshot.as_ref().map(|snapshot| {
            snapshot
                .iter()
                .filter(|(_, visible)| **visible)
                .map(|(path, _)| path.clone())
                .collect()
        })
    };
    let Some(visible) = visible else {
        return;
    };
    {
        let rust = &mut *view.as_mut().rust_mut();
        rust.solo_snapshot = None;
    }
    let visible: QStringList = visible.into_iter().map(QString::from).collect();
    view.as_mut()
        .apply_visibility(&visible, &QString::from("Restore Visibility"));
}

fn solo_clear(mut view: Pin<&mut PictureView>) {
    let rust = &mut *view.as_mut().rust_mut();
    rust.solo_snapshot = None;
}

fn solo_active(view: &PictureView) -> bool {
    view.rust().solo_snapshot.is_some()
}

fn expansion_set(mut view: Pin<&mut PictureView>, paths: &QStringList) {
    let mut seen = std::collections::HashSet::new();
    let paths: Vec<String> = paths
        .iter()
        .map(ToString::to_string)
        .filter(|path| seen.insert(path.clone()))
        .collect();
    let rust = &mut *view.as_mut().rust_mut();
    rust.expanded_groups = paths;
}

fn expansion_paths(view: &PictureView) -> QStringList {
    view.rust()
        .expanded_groups
        .iter()
        .map(|path| QString::from(path.as_str()))
        .collect()
}

/// The layer, its ancestors, and a group's descendants that were visible in
/// the snapshot.
fn solo_paths(
    doc: &pictura_core::Document,
    path: &str,
    snapshot: &std::collections::HashMap<String, bool>,
) -> Vec<String> {
    let mut result = vec![path.to_string()];
    let mut current = path;
    while let Some(parent) = pictura_render::parent_path(current) {
        result.push(parent.to_string());
        current = parent;
    }
    if pictura_render::resolve_path(doc, path).is_some_and(|layer| layer.is_group) {
        let prefix = format!("{path}/");
        for (candidate, _) in pictura_render::flatten_rows(doc) {
            if candidate.starts_with(&prefix) && snapshot.get(&candidate).copied().unwrap_or(false)
            {
                result.push(candidate);
            }
        }
    }
    result.dedup();
    result
}
