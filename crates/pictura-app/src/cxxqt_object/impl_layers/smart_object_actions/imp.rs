use super::*;

use crate::cxxqt_object::edit::{structural, structural_after, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

pub(super) fn layer_can_reset_smart_object_transform(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_reset_smart_object_transform(doc, &path.to_string()))
}

pub(super) fn smart_object_reset_transform(
    mut view: Pin<&mut PictureView>,
    path: &QString,
) -> bool {
    let path = path.to_string();
    let mut changed = false;
    view.as_mut().edit("Reset Transform", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::reset_smart_object_transform(doc, &path);
        if changed {
            let after = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::damage::influence(doc, &path));
            structural(before, after)
        } else {
            Edit::None
        }
    });
    changed
}

pub(super) fn layer_can_convert_smart_object_to_layers(view: &PictureView, path: &QString) -> bool {
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        pictura_render::can_convert_smart_object_to_layers(doc, &path.to_string())
    })
}

pub(super) fn smart_object_convert_to_layers(
    mut view: Pin<&mut PictureView>,
    path: &QString,
) -> bool {
    let path = path.to_string();
    let edit = view.as_mut().edit("Convert to Layers", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
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
        let after = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        structural(before, after)
    });
    !matches!(edit, Edit::None)
}

pub(super) fn layer_can_new_smart_object_via_copy(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_new_smart_object_via_copy(doc, &path.to_string()))
}

pub(super) fn smart_object_new_via_copy(
    mut view: Pin<&mut PictureView>,
    path: &QString,
) -> QString {
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
        let after = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &created));
        structural_after(after.map(|rect| vec![rect]))
    });
    QString::from(created.as_str())
}
