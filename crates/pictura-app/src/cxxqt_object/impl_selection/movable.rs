//! Whether the Move tool's selection drag can move the active target's pixels.

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::qobject;
use cxx_qt::CxxQtType;

impl qobject::PictureView {
    /// Whether the Move tool's selection drag can move the active target's
    /// selected pixels. False for a mask-less target that is a type or shape
    /// layer, hidden, or locked (position always, pixels always, transparency
    /// when cutting), so the tool moves the whole layer instead of previewing a
    /// content move the engine will refuse. A mask edit target is always
    /// movable, as `move_selection_content` handles it separately.
    pub fn selection_content_movable(&self, duplicate: bool) -> bool {
        let rust = self.rust();
        if !rust.quick_mask_content_allowed() {
            return false;
        }
        if active_mask_target(rust).is_some() {
            return true;
        }
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let Some(path) = rust.layer_target.single() else {
            return false;
        };
        let Some(layer) = active_pixel_layer(doc, Some(path)) else {
            return false;
        };
        // Mirrors the engine's refusals in `pictura_render::move_selection_content`;
        // a new lock flag must be added in both places.
        layer.visible
            && !layer.is_type()
            && !pictura_render::is_shape_layer(layer)
            && !pictura_core::layer_move_locked(layer)
            && !pictura_core::layer_pixel_locked(layer)
            && (duplicate || !pictura_core::layer_transparency_locked(layer))
    }
}
