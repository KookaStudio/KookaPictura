//! Layers-panel surface bridge: the CS6 Panel Options that gate duplicate
//! naming and default fill masks, the `Layer > Lock All Layers In Group…`
//! batch, and whether a selection can be deleted.
//!
//! Free functions over a [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list in `cxxqt_object.rs` stays at its size
//! ceiling). Each lock edit recomposites and records one state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
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

        /// Apply the CS6 Layers Panel Options that gate bridge behaviour:
        /// `add_copy` (name a duplicate `"<name> copy"`) and
        /// `use_default_masks` (a fill/adjustment layer created with a
        /// selection takes it as a mask).
        fn set_layers_panel_options(
            view: Pin<&mut PictureView>,
            add_copy: bool,
            use_default_masks: bool,
        );

        /// Apply the full lock set to every descendant of the group at
        /// `group_path`; one "Lock Group Layers" state. Returns the number of
        /// layers changed, `0` when `group_path` is not a group.
        fn lock_group_layers_run(view: Pin<&mut PictureView>, group_path: &QString) -> i32;

        /// Whether deleting `paths` would remove anything: false for an empty
        /// selection, the Background, fully-locked layers, and the document's
        /// last layer, all of which Delete Layer refuses.
        fn layers_can_delete(view: &PictureView, paths: &QStringList) -> bool;
    }
}

fn set_layers_panel_options(
    mut view: Pin<&mut PictureView>,
    add_copy: bool,
    use_default_masks: bool,
) {
    let mut rust = view.as_mut().rust_mut();
    rust.add_copy = add_copy;
    rust.use_default_masks = use_default_masks;
}

fn lock_group_layers_run(mut view: Pin<&mut PictureView>, group_path: &QString) -> i32 {
    let path = group_path.to_string();
    let mut changed = 0;
    view.as_mut().edit("Lock Group Layers", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::lock_group_layers(doc, &path);
        if changed == 0 {
            Edit::None
        } else {
            Edit::Full
        }
    });
    changed as i32
}

fn layers_can_delete(view: &PictureView, paths: &QStringList) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    let owned: Vec<String> = paths.iter().map(ToString::to_string).collect();
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_delete_paths(doc, &refs)
}
