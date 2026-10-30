use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

pub(super) fn set_layers_panel_options(
    mut view: Pin<&mut PictureView>,
    add_copy: bool,
    use_default_masks: bool,
    add_mask_by_default: bool,
) {
    let mut rust = view.as_mut().rust_mut();
    rust.add_copy = add_copy;
    rust.use_default_masks = use_default_masks;
    rust.add_mask_by_default = add_mask_by_default;
}

pub(super) fn lock_group_layers_run(mut view: Pin<&mut PictureView>, group_path: &QString) -> i32 {
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

pub(super) fn layers_can_delete(view: &PictureView, paths: &QStringList) -> bool {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return false;
    };
    let owned: Vec<String> = paths.iter().map(ToString::to_string).collect();
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    pictura_render::can_delete_paths(doc, &refs)
}
