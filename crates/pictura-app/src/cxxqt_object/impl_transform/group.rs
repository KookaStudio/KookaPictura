//! The linked-set side of Free Transform: which layers a session targets, the
//! per-member map, and the group's preview/commit image.
//!
//! A single layer is a group of one, so these helpers are also the single-layer
//! path.

use super::super::document_session::{RepeatTransform, TransformSession};
use super::super::helpers::layer_has_effects;
use super::geometry::{apply_session_transform, rect_center, source_corners, MapParams};
use super::link_members;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::memory_gate;
use crate::cxxqt_object::state::PictureViewRust;
use pictura_core::{Document, Homography, LockFlags, PsdRect, TILE};
use pictura_render::Interpolation;
use std::sync::Arc;

/// The repeatable part of a committed session: the similarity parameters and
/// the reference point's offset from the source rect's centre. Projective and
/// Warp sessions are not repeatable through `Transform Again`.
pub(super) fn repeatable_transform(session: &TransformSession) -> Option<RepeatTransform> {
    if session.mode.is_projective() || session.base_quad.is_some() || session.warp.is_some() {
        return None;
    }
    let (cx, cy) = rect_center(session.orig_rect);
    Some(RepeatTransform {
        scale_x: session.scale_x,
        scale_y: session.scale_y,
        angle: session.angle,
        skew_x: session.skew_x,
        skew_y: session.skew_y,
        dx: session.dx,
        dy: session.dy,
        // `pivot` is a source-frame point, so this offset is not shifted by a
        // gesture that carried the pivot through the map.
        pivot_offset: (session.pivot.0 - cx, session.pivot.1 - cy),
    })
}

/// The bytes one worker thread's source window and block coordinates hold in
/// the tiled resampler at most (4 Mpx of up to 32-bit samples is 16 MiB, kept
/// as 8 MiB because an 8-bit layer's windows dominate).
const WINDOW_BYTES: u64 = 8 << 20;

/// What a transform commit holds (#1290): `working` stays resident (the
/// resample strips, the composite's strip and group buffers, and one source
/// window per worker thread); `spillable` is written as tiles the evictor may
/// pack or spill (the new layer tiles and the region composite). A whole-pixel
/// move keeps the layer's planes, so it adds no layer tiles.
pub(super) struct CommitCost {
    pub(super) working: u64,
    pub(super) spillable: u64,
}

pub(super) fn commit_cost(quad: &[(f64, f64); 4], orig_rect: PsdRect) -> CommitCost {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in quad {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let clamp = |v: f64| v.clamp(i32::MIN as f64 / 2.0, i32::MAX as f64 / 2.0) as i32;
    let out = PsdRect {
        top: clamp(y0.floor()),
        left: clamp(x0.floor()),
        bottom: clamp(y1.ceil()),
        right: clamp(x1.ceil()),
    };
    let union = out.union(orig_rect);
    let layer = if whole_pixel_move(quad, orig_rect) {
        0
    } else {
        4 * memory_gate::area(out)
    };
    let threads = rayon::current_num_threads() as u64;
    CommitCost {
        working: composite_strip_bytes(out.width().max(union.width())) + threads * WINDOW_BYTES,
        spillable: layer + 4 * memory_gate::area(union),
    }
}

/// What a Move tool commit of the changed region `dirty` holds (#1290): a
/// translation keeps the layers' planes and resamples nothing, so it is the
/// composite alone, as for a whole-pixel transform.
pub(super) fn move_commit_cost(dirty: PsdRect) -> CommitCost {
    CommitCost {
        working: composite_strip_bytes(dirty.width()),
        spillable: 4 * memory_gate::area(dirty),
    }
}

/// The composite's strip and group buffers across `width` columns.
fn composite_strip_bytes(width: i32) -> u64 {
    3 * 4 * TILE as u64 * width.max(0) as u64
}

/// Whether `quad` is `rect` moved by one whole-pixel offset.
fn whole_pixel_move(quad: &[(f64, f64); 4], rect: PsdRect) -> bool {
    let corners = source_corners(rect);
    let (dx, dy) = (quad[0].0 - corners[0].0, quad[0].1 - corners[0].1);
    dx.fract() == 0.0
        && dy.fract() == 0.0
        && quad
            .iter()
            .zip(corners)
            .all(|(q, c)| q.0 - c.0 == dx && q.1 - c.1 == dy)
}

/// Whether `path` is a transform target. `require_visible` is true for the
/// layer the user began on; a linked partner still transforms while hidden
/// (Photoshop keeps the set rigid), so it is checked with visibility ignored.
pub(super) fn can_transform_target(doc: &Document, path: &str, require_visible: bool) -> bool {
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return false;
    };
    if pictura_render::is_shape_layer(layer) {
        return (!require_visible || layer.visible)
            && !layer.lock.contains(LockFlags::POSITION)
            && pictura_render::shape_layer_rect(layer, doc.width, doc.height)
                .is_some_and(|rect| rect.width() > 0 && rect.height() > 0);
    }
    if (require_visible && !layer.visible)
        || layer.is_group
        || layer.adjustment.is_some()
        || layer.background
        || layer.lock.contains(LockFlags::POSITION)
        || layer.rect.width() <= 0
        || layer.rect.height() <= 0
    {
        return false;
    }
    if layer.channels.iter().any(|c| c.id == 0) {
        return true;
    }
    pictura_render::can_materialize_smart_source(layer)
}

pub(super) fn can_free_transform(doc: &Document, path: &str) -> bool {
    can_transform_target(doc, path, true)
}

/// The rect a transform box uses for `path`: a shape layer's outline box, or
/// the layer's own (already content-trimmed) rect.
pub(super) fn transform_rect(doc: &Document, path: &str) -> Option<PsdRect> {
    let layer = pictura_render::resolve_path(doc, path)?;
    pictura_render::shape_layer_rect(layer, doc.width, doc.height).or(Some(layer.rect))
}

/// The session's transform targets for `path`: the layer alone, or its link
/// set with every transformable member, in panel order. A hidden,
/// non-transformable, or position-locked member is skipped like the Move tool
/// skips it.
pub(super) fn transform_group(rust: &PictureViewRust, doc: &Document, path: &str) -> Vec<String> {
    let mut group: Vec<String> = Vec::new();
    for member in link_members(rust, doc, path) {
        if (member == path || can_transform_target(doc, &member, false)) && !group.contains(&member)
        {
            group.push(member);
        }
    }
    if group.is_empty() {
        vec![path.to_string()]
    } else {
        group
    }
}

/// Apply the session's map to one member of a transform group. A projective
/// session's quad is solved for the group's union rect, so each member's quad
/// is its own corners carried through that same homography; a warp applies its
/// document-space mesh to every member unchanged.
#[allow(clippy::too_many_arguments)]
fn apply_transform_member(
    doc: &mut Document,
    path: &str,
    rect: PsdRect,
    projective: bool,
    params: MapParams,
    interpolation: Interpolation,
    warp: Option<&pictura_core::warp::Warp>,
    preview_quad: [(f64, f64); 4],
    group_map: Option<&Homography>,
) -> bool {
    if let Some(warp) = warp {
        return pictura_render::transform_layer_warp(
            doc,
            path,
            warp,
            Some(preview_quad),
            interpolation,
        );
    }
    if projective {
        let Some(group_map) = group_map else {
            return false;
        };
        let mut member_quad = source_corners(rect);
        for corner in &mut member_quad {
            let Some(mapped) = group_map.apply(corner.0, corner.1) else {
                return false;
            };
            *corner = mapped;
        }
        return pictura_render::transform_layer_quad_with(doc, path, member_quad, interpolation);
    }
    apply_session_transform(doc, path, false, preview_quad, params, rect, interpolation)
}

/// Build the engine call for a lifted selection's commit: the transform, then
/// the lifted pixels merged back into their layer.
#[allow(clippy::too_many_arguments)]
pub(super) fn lifted_apply(
    path: String,
    projective: bool,
    quad: [(f64, f64); 4],
    params: MapParams,
    interpolation: Interpolation,
    warp: Option<pictura_core::warp::Warp>,
    preview_quad: [(f64, f64); 4],
    orig_rect: PsdRect,
) -> Arc<dyn Fn(&mut Document) -> bool + Send + Sync> {
    Arc::new(move |doc: &mut Document| {
        let changed = match &warp {
            Some(warp) => pictura_render::transform_layer_warp(
                doc,
                &path,
                warp,
                Some(preview_quad),
                interpolation,
            ),
            None => apply_session_transform(
                doc,
                &path,
                projective,
                quad,
                params,
                orig_rect,
                interpolation,
            ),
        };
        changed && pictura_render::merge_lifted(doc, &path)
    })
}

/// Build the engine call for a commit: every member under the same map, run on
/// a clone so a refused member cannot leave the set half-transformed.
#[allow(clippy::too_many_arguments)]
pub(super) fn group_apply(
    members: Vec<(String, PsdRect)>,
    projective: bool,
    quad: [(f64, f64); 4],
    params: MapParams,
    interpolation: Interpolation,
    warp: Option<pictura_core::warp::Warp>,
    preview_quad: [(f64, f64); 4],
    orig_rect: PsdRect,
) -> Arc<dyn Fn(&mut Document) -> bool + Send + Sync> {
    let group_map = projective
        .then(|| Homography::from_quads(source_corners(orig_rect), quad))
        .flatten();
    Arc::new(move |doc: &mut Document| {
        if members.len() > 1 {
            let mut candidate = doc.clone();
            for (member, rect) in &members {
                if !apply_transform_member(
                    &mut candidate,
                    member,
                    *rect,
                    projective,
                    params,
                    interpolation,
                    warp.as_ref(),
                    preview_quad,
                    group_map.as_ref(),
                ) {
                    return false;
                }
            }
            *doc = candidate;
            return true;
        }
        let Some((member, rect)) = members.first() else {
            return false;
        };
        apply_transform_member(
            doc,
            member,
            *rect,
            projective,
            params,
            interpolation,
            warp.as_ref(),
            preview_quad,
            group_map.as_ref(),
        )
    })
}

/// The edit a transform group needs: every member's before/after reach, or the
/// whole document when a member's effects can spread past its rect.
pub(super) fn transform_group_edit(
    doc: &Document,
    members: &[(String, PsdRect)],
    orig_rect: PsdRect,
) -> Edit {
    let mut dirty = orig_rect;
    for (member, _) in members {
        let Some(layer) = pictura_render::resolve_path(doc, member) else {
            return Edit::Full;
        };
        if layer_has_effects(layer) {
            return Edit::Full;
        }
        dirty = dirty.union(layer.rect);
    }
    Edit::regions(vec![dirty], false)
}
