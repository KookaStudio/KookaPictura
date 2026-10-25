//! Solid/gradient fill creation and the Rasterize subset bridge commands.
//!
//! A fill layer is an opaque `SoCo` or `GdFl` block (solid or gradient), so it
//! lives in the existing adjustment slot and needs no new `Layer` field. Each
//! successful command recomposites then records exactly one undo state; a
//! refusal records nothing.

use super::helpers_composite::{default_layer_mask, selection_to_mask};
use super::qobject;
use crate::cxxqt_object::document_session::DocumentSession;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

/// CS6 "Use Default Masks on Fill Layers": when enabled, the just-created fill
/// layer at `path` takes the active selection as a layer mask, or a white
/// document-sized mask when there is no selection. A no-op when the option is
/// off or the path did not resolve.
fn apply_default_mask(
    doc: &mut pictura_core::Document,
    path: &str,
    selection: Option<&pictura_select::Selection>,
    use_default_masks: bool,
) {
    if !use_default_masks || path.is_empty() {
        return;
    }
    let mask = default_layer_mask(
        doc,
        selection.map(|selection| selection_to_mask(selection, doc)),
    );
    if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
        layer.mask = Some(mask);
    }
}

impl qobject::PictureView {
    /// Append a solid-color fill layer for `rgba` (`0xAARRGGBB`) at the top of
    /// the stack. Records one "Color Fill" state on success. Returns the new
    /// path, or empty without a document.
    pub fn add_solid_fill(mut self: Pin<&mut Self>, rgba: u32) -> QString {
        let color = [
            (rgba >> 16) as u8,
            (rgba >> 8) as u8,
            rgba as u8,
            (rgba >> 24) as u8,
        ];
        let use_mask = self.rust().use_default_masks;
        let anchor = self.rust().layer_target.anchor().unwrap_or("").to_string();
        let mut created = String::new();
        self.as_mut().edit("Color Fill", |rust| {
            let DocumentSession { doc, selection, .. } = &mut rust.session;
            let Some(doc) = doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_solid_fill(doc, &anchor, color);
            apply_default_mask(doc, &created, selection.as_ref(), use_mask);
            if created.is_empty() {
                return Edit::None;
            }
            rust.select_only_layer(&created);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    /// Append a black-to-white Linear gradient fill layer at the top of the
    /// stack. Records one "Gradient Fill" state on success. Returns the new
    /// path, or empty without a document.
    pub fn add_gradient_fill(mut self: Pin<&mut Self>) -> QString {
        let use_mask = self.rust().use_default_masks;
        let anchor = self.rust().layer_target.anchor().unwrap_or("").to_string();
        let mut created = String::new();
        self.as_mut().edit("Gradient Fill", |rust| {
            let DocumentSession { doc, selection, .. } = &mut rust.session;
            let Some(doc) = doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_gradient_fill(doc, &anchor);
            apply_default_mask(doc, &created, selection.as_ref(), use_mask);
            if created.is_empty() {
                return Edit::None;
            }
            rust.select_only_layer(&created);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    /// Whether the layer at `path` is a decodable solid-color or gradient fill
    /// layer.
    pub fn layer_is_fill_content(&self, path: &QString) -> bool {
        self.rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
            .is_some_and(pictura_render::is_fill_content_layer)
    }

    /// `Rasterize Fill Content`: bake the fill at `path` into pixels and clear
    /// its fill data. Records one "Rasterize Fill Content" state on success.
    /// Returns false (no state) when `path` is not a decodable fill layer.
    pub fn rasterize_fill_content(mut self: Pin<&mut Self>, path: &QString) -> bool {
        self.as_mut()
            .rasterize_path(&path.to_string(), "Rasterize Fill Content")
    }

    /// `Rasterize Layer` on a type layer or a fill-content layer. A type layer
    /// is rasterized through the bundled text backend and records one
    /// "Rasterize Type" state; a fill layer records "Rasterize Layer". Returns
    /// false (no state) for any other kind.
    pub fn rasterize_layer(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        if path.is_empty() {
            return false;
        }
        let is_type = self
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .is_some_and(pictura_core::Layer::is_type);
        let label = if is_type {
            "Rasterize Type"
        } else {
            "Rasterize Layer"
        };
        let mut changed = false;
        self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::render_text_layer(doc, &path)
                || pictura_render::rasterize_fill_content(doc, &path);
            if changed {
                Edit::Structural
            } else {
                Edit::None
            }
        });
        changed
    }

    /// `Rasterize All Layers`: rasterize every fill-content or type layer. Records one
    /// "Rasterize All Layers" state only when at least one was rasterized.
    /// Returns how many were rasterized.
    pub fn rasterize_all_layers(mut self: Pin<&mut Self>) -> i32 {
        let mut count = 0;
        self.as_mut().edit("Rasterize All Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            count = pictura_render::rasterize_all_layers(doc);
            if count == 0 {
                Edit::None
            } else {
                Edit::Structural
            }
        });
        count as i32
    }

    /// `Rasterize Type`: render the type layer at `path` through the Qt font
    /// backend, falling back to the bundled face when Qt yields no pixels, and
    /// record one "Rasterize Type" state on success. Returns false (no state)
    /// for a non-type layer.
    pub fn rasterize_type(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        if path.is_empty() {
            return false;
        }
        let mut changed = false;
        self.as_mut().edit("Rasterize Type", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let rendered = pictura_render::resolve_path(doc, &path).and_then(|layer| {
                let tool = layer.type_tool.as_ref()?;
                let style = tool.style.as_ref()?;
                let family = style.font.as_deref().unwrap_or("sans-serif");
                let fill = style.rgba();
                Some(super::qobject::render_text_rgba(
                    family,
                    style.character.size,
                    &tool.text,
                    i32::from(style.paragraph.justify.index()),
                    fill[0],
                    fill[1],
                    fill[2],
                    fill[3],
                    layer.rect.width(),
                    layer.rect.height(),
                ))
            });
            changed = match rendered {
                Some(rgba) if !rgba.is_empty() => {
                    pictura_render::materialize_text_rgba(doc, &path, &rgba)
                }
                _ => pictura_render::render_text_layer(doc, &path),
            };
            if changed {
                Edit::Structural
            } else {
                Edit::None
            }
        });
        changed
    }

    /// Whether the layer at `path` is a type layer. Read-only.
    pub fn layer_is_type(&self, path: &QString) -> bool {
        self.rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
            .is_some_and(|layer| layer.is_type())
    }

    fn rasterize_path(mut self: Pin<&mut Self>, path: &str, label: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        let path = path.to_string();
        let mut changed = false;
        self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::rasterize_fill_content(doc, &path);
            if changed {
                Edit::Structural
            } else {
                Edit::None
            }
        });
        changed
    }
}
