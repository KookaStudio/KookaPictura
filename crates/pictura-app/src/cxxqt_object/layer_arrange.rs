//! Layer > Arrange / Reverse and the non-destructive Stamp Visible / Stamp
//! Selected. Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow). Each edit recomposites and
//! records one state.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::{join_path, list_of_strings};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
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
        /// Whether `arrange` (`0` Front, `1` Forward, `2` Backward, `3` Back)
        /// would reorder the layer at `path`.
        fn arrange_layer_can(view: &PictureView, path: &QString, arrange: i32) -> bool;

        /// Whether a delta move would change the order; no history.
        fn can_move_layer_path(view: &PictureView, path: &QString, delta: i32) -> bool;

        /// Whether `path` can become the Background; no history.
        fn can_background_from_layer(view: &PictureView, path: &QString) -> bool;

        /// Apply `arrange` to `path` within its container; one "Arrange Layer"
        /// state when the order changes.
        fn arrange_layer_run(view: Pin<&mut PictureView>, path: &QString, arrange: i32) -> bool;

        /// Reverse the selected contiguous run; one "Reverse Layers" state when
        /// the order changes.
        fn reverse_layers_run(view: Pin<&mut PictureView>, paths: &QStringList) -> bool;

        /// Composite the eye-visible layers into a NEW layer directly above
        /// `path`, leaving the originals. One "Stamp Visible" state. Returns
        /// the new path, or empty on refusal.
        fn stamp_visible_run(view: Pin<&mut PictureView>, path: &QString) -> QString;

        /// Composite the selected layers into a NEW layer directly above
        /// `active`, leaving the originals. One "Stamp Selected" state. Returns
        /// the new path, or empty on refusal.
        fn stamp_selected_run(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            active: &QString,
        ) -> QString;
    }
}

fn can_move_layer_path(view: &PictureView, path: &QString, delta: i32) -> bool {
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_move_path(doc, &path.to_string(), delta),
        None => false,
    }
}

fn can_background_from_layer(view: &PictureView, path: &QString) -> bool {
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_background_from_layer(doc, &path.to_string()),
        None => false,
    }
}

/// Map the bridge ordinal to the engine arrange op.
fn arrange_kind(arrange: i32) -> Option<pictura_render::Arrange> {
    match arrange {
        0 => Some(pictura_render::Arrange::Front),
        1 => Some(pictura_render::Arrange::Forward),
        2 => Some(pictura_render::Arrange::Backward),
        3 => Some(pictura_render::Arrange::Back),
        _ => None,
    }
}

/// Clear link sets, recomposite, and record one history state.
fn finish(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Structural);
}

fn arrange_layer_can(view: &PictureView, path: &QString, arrange: i32) -> bool {
    let Some(op) = arrange_kind(arrange) else {
        return false;
    };
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_arrange_path(doc, &path.to_string(), op),
        None => false,
    }
}

fn arrange_layer_run(mut view: Pin<&mut PictureView>, path: &QString, arrange: i32) -> bool {
    let Some(op) = arrange_kind(arrange) else {
        return false;
    };
    let path = path.to_string();
    let landed = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::arrange_path_index(doc, &path, op).map(|index| {
            let parent = pictura_render::parent_path(&path).unwrap_or("");
            join_path(parent, index)
        }),
        None => None,
    };
    let Some(landed) = landed else {
        return false;
    };
    view.as_mut().rust_mut().select_only_layer(&landed);
    finish(view, "Arrange Layer");
    true
}

fn reverse_layers_run(mut view: Pin<&mut PictureView>, paths: &QStringList) -> bool {
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    let reversed = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::reverse_paths_result(doc, &refs),
        None => None,
    };
    let Some(reversed) = reversed else {
        return false;
    };
    {
        let rust = &mut *view.as_mut().rust_mut();
        let mut target = rust.layer_target.clone();
        let anchor = target.anchor().map(str::to_string);
        target.select(reversed, anchor, rust.session.doc.as_ref());
        rust.set_layer_target(target);
    }
    finish(view, "Reverse Layers");
    true
}

fn stamp_visible_run(mut view: Pin<&mut PictureView>, path: &QString) -> QString {
    let active = path.to_string();
    let created = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => {
            pictura_render::stamp_scope(doc, pictura_render::StampScope::Visible(&active), &active)
        }
        None => None,
    };
    stamp_result(view, created, "Stamp Visible")
}

fn stamp_selected_run(
    mut view: Pin<&mut PictureView>,
    paths: &QStringList,
    active: &QString,
) -> QString {
    let owned = list_of_strings(paths);
    let anchor = active.to_string();
    let created = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => {
            pictura_render::stamp_scope(doc, pictura_render::StampScope::Selected(&owned), &anchor)
        }
        None => None,
    };
    stamp_result(view, created, "Stamp Selected")
}

fn stamp_result(mut view: Pin<&mut PictureView>, created: Option<String>, label: &str) -> QString {
    match created {
        Some(path) => {
            view.as_mut().rust_mut().select_only_layer(&path);
            finish(view, label);
            QString::from(path.as_str())
        }
        None => QString::default(),
    }
}
