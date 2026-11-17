use super::*;

use crate::cxxqt_object::edit::{structural, structural_after, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::StackMode;

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

fn stack_mode_from_code(code: i32) -> Option<Option<StackMode>> {
    match code {
        0 => Some(None),
        1..=11 => StackMode::from_code(code as u8).map(Some),
        _ => None,
    }
}

pub(super) fn layer_smart_object_stack_mode(view: &PictureView, path: &QString) -> i32 {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::smart_object_stack_mode(doc, &path.to_string()))
        .map_or(0, |mode| i32::from(mode.code()))
}

pub(super) fn layer_can_set_smart_object_stack_mode(
    view: &PictureView,
    path: &QString,
    mode: i32,
) -> bool {
    let Some(target) = stack_mode_from_code(mode) else {
        return false;
    };
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        pictura_render::can_set_smart_object_stack_mode(doc, &path.to_string(), target)
    })
}

pub(super) fn set_smart_object_stack_mode(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    mode: i32,
) -> bool {
    let Some(target) = stack_mode_from_code(mode) else {
        return false;
    };
    let path = path.to_string();
    let current = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::smart_object_stack_mode(doc, &path));
    if current == target {
        return false;
    }
    let label = match target {
        Some(mode) => format!("Stack Mode: {}", mode.label()),
        None => "Stack Mode: None".to_string(),
    };
    let mut changed = false;
    view.as_mut().edit(&label, |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::set_smart_object_stack_mode(doc, &path, target);
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
