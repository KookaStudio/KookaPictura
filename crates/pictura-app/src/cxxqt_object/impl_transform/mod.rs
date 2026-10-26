mod bridge;
mod convex;
mod dispatch;
mod distort;
mod geometry;
mod perspective;
mod session;
mod skew;
mod warp;

use super::helpers::*;
use super::helpers_composite::*;
use super::state::PictureViewRust;
use crate::layer_target::LayerTarget;
use cxx_qt_lib::{QImage, QRect};
use pictura_core::{layer_move_locked, Document, LayerStack, PsdRect};

/// Build the move-preview base for the layers at `paths`: the authoritative
/// composite with each layer's rect recomposited as if hidden, then each layer's
/// prior `visible` flag restored, so starting a move never reveals an invisible
/// layer. A single path is the common case; a link set hides every member.
pub(super) fn build_move_preview_base(
    doc: &mut Document,
    paths: &[String],
    gpu_compute: bool,
) -> QImage {
    let mut members: Vec<(String, bool, PsdRect)> = Vec::new();
    for path in paths {
        if let Some(layer) = pictura_render::resolve_path(doc, path) {
            members.push((path.clone(), layer.visible, layer.rect));
        }
    }
    let Some(union) = members.iter().map(|(_, _, rect)| *rect).reduce(union_rect) else {
        return QImage::default();
    };
    let composite_ok = doc.composite.width == doc.width
        && doc.composite.height == doc.height
        && !doc.composite.data.is_empty();
    match move_preview_region(union, doc.width, doc.height, composite_ok) {
        Some((x0, y0, ..)) => {
            set_member_visibility(doc, &members, false);
            let (region, _backend) =
                pictura_render::composite_region_active(doc, union, gpu_compute);
            restore_member_visibility(doc, &members);
            let mut base_buffer = doc.composite.clone();
            patch_buffer_region(&mut base_buffer, &region, x0, y0);
            buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &base_buffer))
        }
        None => {
            // ponytail: full-composite fallback for a missing/mismatched
            // composite; the region path covers the common case.
            set_member_visibility(doc, &members, false);
            let base = document_to_image(doc, gpu_compute);
            restore_member_visibility(doc, &members);
            base
        }
    }
}

/// Hide (`visible` false) or restore each member's own prior `visible` flag.
fn set_member_visibility(doc: &mut Document, members: &[(String, bool, PsdRect)], visible: bool) {
    for (path, _, _) in members {
        if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
            layer.visible = visible;
        }
    }
}

fn restore_member_visibility(doc: &mut Document, members: &[(String, bool, PsdRect)]) {
    for (path, was_visible, _) in members {
        if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
            layer.visible = *was_visible;
        }
    }
}

/// Translate every layer in `paths` by `(dx, dy)` through
/// `translate_layer_path` and return the union of each moved layer's
/// before/after rect. A member that cannot be translated (a group, an
/// adjustment, a position-locked layer, or an unresolved path) is skipped;
/// `None` when nothing moved.
pub(super) fn translate_move_group(
    doc: &mut Document,
    paths: &[String],
    dx: i32,
    dy: i32,
) -> Option<PsdRect> {
    let mut dirty: Option<PsdRect> = None;
    for path in paths {
        let Some(before) = pictura_render::resolve_path(doc, path).map(|layer| layer.rect) else {
            continue;
        };
        if !pictura_render::translate_layer_path(doc, path, dx, dy) {
            continue;
        }
        let after = pictura_render::resolve_path(doc, path).map_or(before, |layer| layer.rect);
        let rect = union_rect(before, after);
        dirty = Some(dirty.map_or(rect, |acc| union_rect(acc, rect)));
    }
    dirty
}

/// The move group's union rect, or `None` when a path does not resolve. The
/// preview cache keys on it: a rendered group's moving image is composited at
/// the layers' positions and clipped to the canvas, so any move makes it stale.
pub(super) fn move_group_union(doc: &Document, paths: &[String]) -> Option<(i32, i32, i32, i32)> {
    let mut bounds: Option<PsdRect> = None;
    for path in paths {
        let layer = pictura_render::resolve_path(doc, path)?;
        bounds = Some(bounds.map_or(layer.rect, |rect| union_rect(rect, layer.rect)));
    }
    bounds.map(|rect| (rect.left, rect.top, rect.right, rect.bottom))
}

/// Render a link set (its members composited over transparency, each keeping
/// its own opacity and blend) cropped to the members' union rect, for the Move
/// preview. `paths` is panel order (topmost-first); the compositor wants
/// bottom-first, so they are reversed.
///
/// ponytail: a member whose blend mode reads the layers below previews as if
/// over transparency (the base is excluded from this render, as the existing
/// clipping-group preview does); the commit recomposites correctly.
pub(super) fn rendered_move_group(
    doc: &Document,
    paths: &[String],
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let mut layers = Vec::with_capacity(paths.len());
    for path in paths.iter().rev() {
        layers.push(pictura_render::resolve_path(doc, path)?.clone());
    }
    let (left, top, right, bottom) = move_group_union(doc, paths)?;
    let (x0, y0) = (left.max(0), top.max(0));
    let (x1, y1) = (right.min(doc.width as i32), bottom.min(doc.height as i32));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut scratch = doc.clone();
    scratch.layers = LayerStack::from_layers(layers);
    let rendered = current_buffer(&scratch, gpu_compute);
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image.copy(&QRect::new(x0, y0, x1 - x0, y1 - y0)), x0, y0))
}

/// The paths of `path`'s link set in panel order (topmost-first), or just
/// `path` when it is not linked.
pub(super) fn link_members(rust: &PictureViewRust, doc: &Document, path: &str) -> Vec<String> {
    let Some(&id) = rust.link_sets.get(path) else {
        return vec![path.to_string()];
    };
    let members: Vec<String> = pictura_render::flatten_rows(doc)
        .into_iter()
        .filter(|(candidate, _)| rust.link_sets.get(candidate) == Some(&id))
        .map(|(candidate, _)| candidate)
        .collect();
    if members.is_empty() {
        vec![path.to_string()]
    } else {
        members
    }
}

/// The cache key of a move group: its paths joined so a different set or order
/// misses the cache.
pub(super) fn move_group_key(paths: &[String]) -> String {
    paths.join("\n")
}

/// The moving image's document-space origin and the opacity the preview applies:
/// a single layer's own rect/opacity, or a rendered group's union origin with
/// the opacity already baked in (255). A group's origin is clamped to the canvas
/// exactly as [`rendered_move_group`] crops it, so a cached preview and a fresh
/// one place the image identically.
pub(super) fn move_origin_opacity(doc: &Document, paths: &[String]) -> Option<(i32, i32, i32)> {
    if let [path] = paths {
        let layer = pictura_render::resolve_path(doc, path)?;
        return Some((layer.rect.left, layer.rect.top, layer.opacity as i32));
    }
    let mut bounds: Option<PsdRect> = None;
    for path in paths {
        let layer = pictura_render::resolve_path(doc, path)?;
        bounds = Some(bounds.map_or(layer.rect, |rect| union_rect(rect, layer.rect)));
    }
    bounds.map(|rect| (rect.left.max(0), rect.top.max(0), 255))
}

/// The layer at `path` the Move tool drags: a pixel layer, or a shape layer,
/// whose outline moves.
pub(super) fn movable_layer<'a>(doc: &'a Document, path: &str) -> Option<&'a pictura_core::Layer> {
    active_pixel_layer(doc, Some(path)).or_else(|| {
        pictura_render::resolve_path(doc, path).filter(|l| pictura_render::is_shape_layer(l))
    })
}

/// Duplicate the active movable layer, repointing `active` at the copy's path.
pub(super) fn duplicate_move_target(
    doc: &mut Document,
    target: &mut LayerTarget,
) -> Option<String> {
    let path = target.single()?.to_string();
    let layer = movable_layer(doc, &path)?;
    if !layer.visible || layer_move_locked(layer) {
        return None;
    }
    let new_path = pictura_render::duplicate_paths(doc, &[path.as_str()]).pop()?;
    target.select_only(&new_path);
    Some(new_path)
}
