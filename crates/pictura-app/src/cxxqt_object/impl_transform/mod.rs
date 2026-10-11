mod dispatch;
mod geometry;
mod session;

use super::helpers::*;
use super::helpers_composite::*;
use cxx_qt_lib::QImage;
use pictura_core::{layer_move_locked, Document};

/// Build the move-preview base for the layer at `path`: the authoritative
/// composite with the layer's rect recomposited as if hidden, then the layer's
/// prior `visible` flag restored, so starting a move never reveals an invisible
/// layer.
pub(super) fn build_move_preview_base(doc: &mut Document, path: &str, gpu_compute: bool) -> QImage {
    let Some(rect) = pictura_render::resolve_path(doc, path).map(|layer| layer.rect) else {
        return QImage::default();
    };
    let composite_ok = doc.composite.width == doc.width
        && doc.composite.height == doc.height
        && !doc.composite.data.is_empty();
    let was_visible = pictura_render::resolve_path(doc, path).is_some_and(|layer| layer.visible);
    let set_visible = |doc: &mut Document, visible: bool| {
        if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
            layer.visible = visible;
        }
    };
    match move_preview_region(rect, doc.width, doc.height, composite_ok) {
        Some((x0, y0, ..)) => {
            set_visible(doc, false);
            let (region, _backend) =
                pictura_render::composite_region_active(doc, rect, gpu_compute);
            set_visible(doc, was_visible);
            let mut base_buffer = doc.composite.clone();
            patch_buffer_region(&mut base_buffer, &region, x0, y0);
            buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &base_buffer))
        }
        None => {
            // ponytail: full-composite fallback for a missing/mismatched
            // composite; the region path covers the common case.
            set_visible(doc, false);
            let base = document_to_image(doc, gpu_compute);
            set_visible(doc, was_visible);
            base
        }
    }
}

/// Duplicate the active pixel layer, repointing `active` at the copy's path.
pub(super) fn duplicate_move_target(
    doc: &mut Document,
    active: &mut Option<String>,
) -> Option<String> {
    let path = active.clone()?;
    if layer_move_locked(active_pixel_layer(doc, Some(&path))?) {
        return None;
    }
    let new_path = pictura_render::duplicate_paths(doc, &[path.as_str()]).pop()?;
    *active = Some(new_path.clone());
    Some(new_path)
}
