use super::*;

use super::super::helpers::{join_path, list_of_strings};
use crate::cxxqt_object::edit::{structural, structural_paths, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

pub(super) fn can_move_layer_path(view: &PictureView, path: &QString, delta: i32) -> bool {
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_move_path(doc, &path.to_string(), delta),
        None => false,
    }
}

pub(super) fn can_background_from_layer(view: &PictureView, path: &QString) -> bool {
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_background_from_layer(doc, &path.to_string()),
        None => false,
    }
}

/// Map the bridge ordinal to the engine arrange op.
pub(super) fn arrange_kind(arrange: i32) -> Option<pictura_render::Arrange> {
    match arrange {
        0 => Some(pictura_render::Arrange::Front),
        1 => Some(pictura_render::Arrange::Forward),
        2 => Some(pictura_render::Arrange::Backward),
        3 => Some(pictura_render::Arrange::Back),
        _ => None,
    }
}

/// The path an arrange lands on, or `None` when the engine refuses it.
pub(super) fn arrange_landed(
    rust: &mut super::super::state::PictureViewRust,
    path: &str,
    op: pictura_render::Arrange,
) -> Option<String> {
    let doc = rust.session.doc.as_mut()?;
    pictura_render::arrange_path_index(doc, path, op).map(|index| {
        let parent = pictura_render::parent_path(path).unwrap_or("");
        join_path(parent, index)
    })
}

pub(super) fn arrange_layer_can(view: &PictureView, path: &QString, arrange: i32) -> bool {
    let Some(op) = arrange_kind(arrange) else {
        return false;
    };
    match view.rust().session.doc.as_ref() {
        Some(doc) => pictura_render::can_arrange_path(doc, &path.to_string(), op),
        None => false,
    }
}

pub(super) fn arrange_layer_run(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    arrange: i32,
) -> bool {
    let Some(op) = arrange_kind(arrange) else {
        return false;
    };
    let path = path.to_string();
    let edit = view.as_mut().edit("Arrange Layer", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(landed) = arrange_landed(rust, &path, op) else {
            return Edit::None;
        };
        rust.select_only_layer(&landed);
        let after = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &landed));
        structural(before, after)
    });
    !matches!(edit, Edit::None)
}

pub(super) fn reverse_layers_run(mut view: Pin<&mut PictureView>, paths: &QStringList) -> bool {
    let owned = list_of_strings(paths);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    let edit = view.as_mut().edit("Reverse Layers", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence_paths(doc, &refs));
        let Some(reversed) = (match rust.session.doc.as_mut() {
            Some(doc) => pictura_render::reverse_paths_result(doc, &refs),
            None => None,
        }) else {
            return Edit::None;
        };
        let mut target = rust.layer_target.clone();
        let anchor = target.anchor().map(str::to_string);
        target.select(reversed.clone(), anchor, rust.session.doc.as_ref());
        rust.set_layer_target(target);
        let reversed_refs: Vec<&str> = reversed.iter().map(String::as_str).collect();
        let after = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence_paths(doc, &reversed_refs));
        structural_paths(before, after)
    });
    !matches!(edit, Edit::None)
}

pub(super) fn stamp_visible_run(mut view: Pin<&mut PictureView>, path: &QString) -> QString {
    let active = path.to_string();
    let mut created = String::new();
    view.as_mut().edit("Stamp Visible", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(path) =
            pictura_render::stamp_scope(doc, pictura_render::StampScope::Visible(&active), &active)
        else {
            return Edit::None;
        };
        created = path;
        rust.select_only_layer(&created);
        Edit::Structural { damage: None }
    });
    QString::from(created.as_str())
}

pub(super) fn stamp_selected_run(
    mut view: Pin<&mut PictureView>,
    paths: &QStringList,
    active: &QString,
) -> QString {
    let owned = list_of_strings(paths);
    let anchor = active.to_string();
    let mut created = String::new();
    view.as_mut().edit("Stamp Selected", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let Some(path) =
            pictura_render::stamp_scope(doc, pictura_render::StampScope::Selected(&owned), &anchor)
        else {
            return Edit::None;
        };
        created = path;
        rust.select_only_layer(&created);
        Edit::Structural { damage: None }
    });
    QString::from(created.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::document_session::DocumentSession;
    use crate::cxxqt_object::edit::{effect_of, Effect};
    use crate::cxxqt_object::state::PictureViewRust;
    use pictura_core::{BitDepth, ColorMode, Document, LayerStack};

    fn doc_with_layers() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![
            crate::cxxqt_object::tests::pixel_layer("bottom", 4, 4, (255, 0, 0)),
            crate::cxxqt_object::tests::pixel_layer("top", 4, 4, (0, 255, 0)),
        ]);
        doc
    }

    #[test]
    fn a_refused_layer_command_commits_nothing() {
        let mut rust = PictureViewRust {
            session: DocumentSession::with_document(doc_with_layers()),
            ..Default::default()
        };
        let before: Vec<String> = rust
            .session
            .doc
            .as_ref()
            .expect("document")
            .layers
            .iter()
            .map(|layer| layer.name.clone())
            .collect();
        assert_eq!(
            arrange_landed(&mut rust, "missing", pictura_render::Arrange::Front),
            None
        );
        let applied = rust.apply_edit("Arrange Layer", Edit::None);
        assert_eq!(applied.edit, Edit::None);
        assert_eq!(effect_of(&applied.edit), Effect::Nothing);
        assert_eq!(rust.session.history.count(), 0);
        assert_eq!(rust.session.content_revision, 0);
        let after: Vec<String> = rust
            .session
            .doc
            .as_ref()
            .expect("document")
            .layers
            .iter()
            .map(|layer| layer.name.clone())
            .collect();
        assert_eq!(after, before);
    }
}
