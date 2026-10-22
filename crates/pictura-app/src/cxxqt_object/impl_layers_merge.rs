//! Merge and flatten bridge commands. Engine: `pictura_render::merge_scope` /
//! `flatten`; each successful command recomposites then records one undo state,
//! and every refusal records nothing. Merge commands return the merged path
//! (empty on refusal); flatten returns the replaced count.

use super::helpers::*;
use super::qobject;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::ColorLabel;

impl qobject::PictureView {
    /// Merge Down when `paths` names one layer, Merge Layers when it names two
    /// or more (CS6 shares one command, design D4). Returns the merged node's
    /// path, empty on refusal.
    pub fn merge_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> QString {
        let owned = list_of_strings(paths);
        let (scope, label) = if owned.len() == 1 {
            (pictura_render::MergeScope::Down(&owned[0]), "Merge Down")
        } else {
            (pictura_render::MergeScope::Selected(&owned), "Merge Layers")
        };
        self.as_mut()
            .merge_scope_changed(scope, label)
            .map_or_else(QString::default, |outcome| {
                QString::from(outcome.path.as_str())
            })
    }

    /// Whether [`merge_layers`] would apply to `paths`, without mutating the
    /// document or recording history. The UI gates the menu item on it.
    ///
    /// [`merge_layers`]: qobject::PictureView::merge_layers
    pub fn can_merge_layers(&self, paths: &QStringList) -> bool {
        let owned = list_of_strings(paths);
        if owned.is_empty() {
            return false;
        }
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return false;
        };
        let scope = if owned.len() == 1 {
            pictura_render::MergeScope::Down(&owned[0])
        } else {
            pictura_render::MergeScope::Selected(&owned)
        };
        pictura_render::can_merge_scope(doc, &scope)
    }

    /// Whether [`merge_clipping_mask`] would apply to `path`, without mutating
    /// the document or recording history. The UI gates the menu item on it.
    ///
    /// [`merge_clipping_mask`]: qobject::PictureView::merge_clipping_mask
    pub fn can_merge_clipping_mask(&self, path: &QString) -> bool {
        let path = path.to_string();
        if path.is_empty() {
            return false;
        }
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return false;
        };
        pictura_render::can_merge_scope(doc, &pictura_render::MergeScope::ClippingMask(&path))
    }

    /// Merge every eye-visible layer, anchored on `path`. Returns the merged
    /// node's path, empty on refusal (including a hidden active layer).
    pub fn merge_visible(mut self: Pin<&mut Self>, path: &QString) -> QString {
        let path = path.to_string();
        self.as_mut()
            .merge_scope_changed(pictura_render::MergeScope::Visible(&path), "Merge Visible")
            .map_or_else(QString::default, |outcome| {
                QString::from(outcome.path.as_str())
            })
    }

    /// Collapse the clipping group above `path` into its raster base. Returns
    /// the number of inputs replaced, 0 on refusal.
    pub fn merge_clipping_mask(mut self: Pin<&mut Self>, path: &QString) -> i32 {
        let path = path.to_string();
        self.as_mut()
            .merge_scope_changed(
                pictura_render::MergeScope::ClippingMask(&path),
                "Merge Clipping Mask",
            )
            .map_or(0, |outcome| outcome.replaced as i32)
    }

    /// Flatten the whole tree into one opaque Background layer. Returns the
    /// number of layers replaced, 0 on refusal.
    pub fn flatten_image(mut self: Pin<&mut Self>) -> i32 {
        let mut replaced = 0;
        self.as_mut().edit("Flatten Image", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            replaced = pictura_render::flatten(doc).map_or(0, |outcome| outcome.replaced);
            if replaced == 0 {
                return Edit::None;
            }
            rust.select_only_layer("0");
            Edit::Structural
        });
        replaced as i32
    }

    /// `Layer from Background…`: clear the flag and unlock the layer at `path`.
    /// Returns false (no history) when it is not the Background.
    pub fn layer_from_background(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let mut changed = false;
        self.as_mut().edit("Layer from Background", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::layer_from_background(doc, &path);
            if !changed {
                return Edit::None;
            }
            rust.select_only_layer(&path);
            Edit::Structural
        });
        changed
    }

    /// `Layer from Background…` with the dialog's chosen name and color: clear
    /// the flag, unlock, rename to `name` (or inherit the engine's next free
    /// `Layer N` when `name` is empty), and set the color label. Records one
    /// "Layer from Background" state; false (no state) for a non-Background or a
    /// color outside `0..=7`.
    pub fn convert_background(
        mut self: Pin<&mut Self>,
        path: &QString,
        name: &QString,
        color: i32,
    ) -> bool {
        if !(0..=7).contains(&color) {
            return false;
        }
        let path = path.to_string();
        let name = name.to_string();
        let mut changed = false;
        self.as_mut().edit("Layer from Background", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            if !pictura_render::layer_from_background(doc, &path) {
                return Edit::None;
            }
            if let Some(layer) = pictura_render::resolve_path_mut(doc, &path) {
                if !name.is_empty() {
                    layer.name = name.clone();
                }
                layer.color = ColorLabel::from_byte(color as u8);
            }
            changed = true;
            rust.select_only_layer(&path);
            Edit::Structural
        });
        changed
    }

    /// `Background From Layer`: flag the layer at `path`, fill its transparent
    /// pixels with the background color, and move it to the bottom. Returns
    /// false (no history) for a group, an adjustment/fill-content layer, or an
    /// existing Background.
    pub fn background_from_layer(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let mut changed = false;
        self.as_mut().edit("Background From Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::background_from_layer(doc, &path);
            if !changed {
                return Edit::None;
            }
            rust.select_only_layer("0");
            Edit::Structural
        });
        changed
    }

    fn merge_scope_changed(
        mut self: Pin<&mut Self>,
        scope: pictura_render::MergeScope<'_>,
        label: &str,
    ) -> Option<pictura_render::MergeOutcome> {
        let mut result = None;
        self.as_mut().edit(label, |rust| {
            let outcome = {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                pictura_render::merge_scope(doc, scope).ok()
            };
            let Some(path) = outcome
                .as_ref()
                .filter(|outcome| outcome.replaced > 0)
                .map(|outcome| outcome.path.clone())
            else {
                return Edit::None;
            };
            result = outcome;
            let target = rust.merge_target(&path);
            rust.set_layer_target(target);
            Edit::Structural
        });
        result
    }
}
