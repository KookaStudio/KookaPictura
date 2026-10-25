//! New Layer / New Group dialog and Layer via Copy/Cut bridge commands. An
//! accepted dialog creates one node with the chosen attributes directly above
//! the selection path, recomposites unless the insertion is
//! content-neutral, and records exactly one undo state ("New Layer"/"New
//! Group"); via copy/cut needs an active selection, recomposites,
//! and records one "Layer via Copy"/"Layer via Cut" state. An invalid blend key,
//! a missing document, or a missing selection returns empty and records nothing.

use super::helpers::{as_str_slice, list_of_strings};
use super::helpers_composite::selection_to_mask;
use super::qobject;
use crate::cxxqt_object::edit::{insertion_edit, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::{BlendMode, ColorLabel};

/// Decode a 4-byte PSD blend `key`, exactly as [`PictureView::set_layers_blend`]
/// does. `None` for an unknown key.
///
/// [`PictureView::set_layers_blend`]: qobject::PictureView::set_layers_blend
pub(super) fn decode_blend_key(key: &str) -> Option<BlendMode> {
    let bytes = key.as_bytes();
    if bytes.len() != 4 {
        return None;
    }
    BlendMode::from_psd_key([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(super) fn decode_color(value: i32) -> ColorLabel {
    if (0..=7).contains(&value) {
        ColorLabel::from_byte(value as u8)
    } else {
        ColorLabel::None
    }
}

impl qobject::PictureView {
    /// Create a raster layer from the New Layer dialog, directly above
    /// `selection_path` (top of the stack when empty), applying every attribute
    /// in `spec`. Returns the new path, or empty on an invalid blend key or
    /// without a document.
    #[allow(clippy::too_many_arguments)]
    pub fn new_layer_dialog(
        mut self: Pin<&mut Self>,
        selection_path: &QString,
        name: &QString,
        color: i32,
        blend_key: &QString,
        opacity: i32,
        fill: i32,
        neutral_fill: bool,
        clipping: bool,
    ) -> QString {
        let Some(blend) = decode_blend_key(&blend_key.to_string()) else {
            return QString::default();
        };
        let spec = pictura_render::NewLayerSpec {
            name: name.to_string(),
            color: decode_color(color),
            blend,
            opacity: opacity.clamp(0, 255) as u8,
            fill: fill.clamp(0, 255) as u8,
            clipping,
            neutral_fill,
        };
        let mut created = String::new();
        self.as_mut().edit("New Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_layer_full(doc, &selection_path.to_string(), &spec);
            if created.is_empty() {
                return Edit::None;
            }
            let edit = insertion_edit(doc, &created);
            rust.select_only_layer(&created);
            edit
        });
        QString::from(created.as_str())
    }

    /// Create an empty group from the New Group dialog by the [`new_layer_dialog`]
    /// placement rule. Groups ignore fill, clipping, and the neutral fill, so
    /// only name/color/blend/opacity are carried. Returns the new path, or empty
    /// on an invalid blend key or without a document.
    ///
    /// [`new_layer_dialog`]: qobject::PictureView::new_layer_dialog
    pub fn new_group_dialog(
        mut self: Pin<&mut Self>,
        selection_path: &QString,
        name: &QString,
        color: i32,
        blend_key: &QString,
        opacity: i32,
    ) -> QString {
        let Some(blend) = decode_blend_key(&blend_key.to_string()) else {
            return QString::default();
        };
        let spec = pictura_render::NewLayerSpec {
            name: name.to_string(),
            color: decode_color(color),
            blend,
            opacity: opacity.clamp(0, 255) as u8,
            ..pictura_render::NewLayerSpec::default()
        };
        let mut created = String::new();
        self.as_mut().edit("New Group", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_group_full(doc, &selection_path.to_string(), &spec);
            if created.is_empty() {
                return Edit::None;
            }
            let edit = insertion_edit(doc, &created);
            rust.select_only_layer(&created);
            edit
        });
        QString::from(created.as_str())
    }

    /// `Layer > New > Group from Layers…`: wrap the dialog's `paths` in one new
    /// group, then apply the chosen name/color/blend/opacity to it. Recomposites
    /// and records exactly one "Group from Layers" state. Returns the new path,
    /// or empty on an empty/invalid selection or an invalid blend key.
    pub fn group_from_layers_dialog(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        name: &QString,
        color: i32,
        blend_key: &QString,
        opacity: i32,
    ) -> QString {
        let Some(blend) = decode_blend_key(&blend_key.to_string()) else {
            return QString::default();
        };
        let owned = list_of_strings(paths);
        let mut created = String::new();
        self.as_mut().edit("Group from Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let refs = as_str_slice(&owned);
            let Some(path) = pictura_render::group_paths(doc, &refs) else {
                return Edit::None;
            };
            if let Some(group) = pictura_render::resolve_path_mut(doc, &path) {
                if !name.is_empty() {
                    group.name = name.to_string();
                }
                group.color = decode_color(color);
                group.blend = blend;
                group.opacity = opacity.clamp(0, 255) as u8;
            }
            created = path;
            rust.select_only_layer(&created);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    /// `Layer via Copy` (`Ctrl+J`): copy the active selection's pixels from the
    /// layer at `path` into a new layer directly above it. Records one
    /// "Layer via Copy" state. Returns the new path, or empty without a
    /// document/selection or when the engine refuses.
    pub fn layer_via_copy(mut self: Pin<&mut Self>, path: &QString) -> QString {
        self.as_mut().layer_via(&path.to_string(), false)
    }

    /// `Layer via Cut` (`Shift+Ctrl+J`): as [`layer_via_copy`], and clear the
    /// selected pixels from the source layer. Records one "Layer via Cut" state.
    ///
    /// [`layer_via_copy`]: qobject::PictureView::layer_via_copy
    pub fn layer_via_cut(mut self: Pin<&mut Self>, path: &QString) -> QString {
        self.as_mut().layer_via(&path.to_string(), true)
    }

    fn layer_via(mut self: Pin<&mut Self>, source_path: &str, cut: bool) -> QString {
        if source_path.is_empty() {
            return QString::default();
        }
        let mask = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return QString::default();
            };
            let Some(selection) = rust.session.selection.as_ref() else {
                return QString::default();
            };
            selection_to_mask(selection, doc)
        };
        let mut created = String::new();
        self.as_mut().edit(
            if cut {
                "Layer via Cut"
            } else {
                "Layer via Copy"
            },
            |rust| {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                created = if cut {
                    pictura_render::layer_via_cut(doc, source_path, &mask)
                } else {
                    pictura_render::layer_via_copy(doc, source_path, &mask)
                };
                if created.is_empty() {
                    return Edit::None;
                }
                rust.select_only_layer(&created);
                Edit::Structural
            },
        );
        QString::from(created.as_str())
    }
}
