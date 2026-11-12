use super::super::document_session::{TransformMode, TransformSession};
use super::super::helpers::*;
use super::super::helpers_composite::*;
use super::super::qobject;
use super::super::state::PictureViewRust;
use super::geometry::{
    apply_session_transform, quad_driven, rect_center, session_quad, source_corners, MapParams,
};
use super::group::{
    can_free_transform, commit_cost, group_apply, lifted_apply, move_commit_cost,
    repeatable_transform, transform_group, transform_group_edit, transform_rect,
};
use super::{
    build_move_preview_base, duplicate_move_target, link_members, movable_layer, move_group_key,
    move_group_union, move_origin_opacity, rendered_move_group, translate_move_group,
};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::memory_gate;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QRect, QString};
use pictura_core::{layer_move_locked, BlendMode, LayerStack, PsdRect};
use pictura_render::Interpolation;

/// A transform whose layer rect reaches this area runs its engine on a worker.
const DEFERRED_TRANSFORM_PIXELS: u64 = 4_000_000;

mod preview;
mod readouts;
mod view_preview;

#[cfg(test)]
use preview::TransformPreview;
use preview::{
    backdrop_patch, build_transform_preview, moving_layer_image, needs_rendered_appearance,
    rendered_layer_image, transform_preview_bytes,
};
pub(in crate::cxxqt_object) use preview::{
    rendered_transform_group, set_transform_preview_pixels_for_test, transform_preview_layer_at,
};
use view_preview::above_patch;
pub(in crate::cxxqt_object) use view_preview::MoveView;

/// The selection's coverage when it is document-sized and not empty: Free
/// Transform then lifts the selected pixels instead of moving the whole layer.
fn lift_coverage(rust: &super::super::state::PictureViewRust) -> Option<Vec<u8>> {
    let doc = rust.session.doc.as_ref()?;
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .filter(|s| pictura_render::coverage_bounds(&s.data, doc.width, doc.height).is_some())
        .map(|s| s.data.to_vec())
}

/// Whether a selection on `path` lifts its pixels for the transform (CS6
/// transforms the selected pixels, a Background's included; a hidden layer's
/// does not).
fn lifts_selection(rust: &PictureViewRust, path: &str) -> bool {
    rust.session.doc.as_ref().is_some_and(|doc| {
        pictura_render::resolve_path(doc, path)
            .is_some_and(|layer| layer.visible && pictura_render::can_lift_selection(layer))
    }) && selection_covers(rust)
}

/// Whether [`lift_coverage`] would find a selection, without copying it.
fn selection_covers(rust: &PictureViewRust) -> bool {
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .is_some_and(|s| pictura_render::plane_has_coverage(&s.data, doc.width, doc.height))
}

/// Whether `mode` may begin on `path`: the Free Transform target rules, plus
/// Distort's and Perspective's refusal for editable type (a lifted selection targets pixels, not
/// the type layer).
pub(super) fn can_begin_mode(rust: &PictureViewRust, path: &str, mode: TransformMode) -> bool {
    if !rust.quick_mask_content_allowed() {
        return false;
    }
    let lift = lifts_selection(rust, path);
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    if !lift && !can_free_transform(doc, path) {
        return false;
    }
    let type_member = transform_group(rust, doc, path).iter().any(|member| {
        pictura_render::resolve_path(doc, member).is_some_and(|layer| layer.is_type())
    });
    !(matches!(mode, TransformMode::Distort | TransformMode::Perspective) && !lift && type_member)
}

impl qobject::PictureView {
    /// Whether `path` resolves to a transformable layer, or to a layer whose
    /// selected pixels can be lifted and transformed. Read-only.
    pub fn layer_can_free_transform(&self, path: &QString) -> bool {
        let path = path.to_string();
        let rust = self.rust();
        lifts_selection(rust, &path)
            || rust
                .session
                .doc
                .as_ref()
                .is_some_and(|doc| can_free_transform(doc, &path))
    }

    /// Begin a Free Transform session on the layer at `path`.
    ///
    /// Returns false without changing any session for an untransformable target.
    /// A second begin on the same active path is a no-op; a begin on a different
    /// path first cancels the active session.
    pub fn begin_free_transform(self: Pin<&mut Self>, path: &QString) -> bool {
        self.begin_transform_session(path, TransformMode::Free)
    }

    /// Begin a Skew / Distort / Perspective session on the layer at `path`.
    /// `mode` is `"skew"`, `"distort"`, or `"perspective"`; false for anything
    /// else or an untransformable target, with the same resolution and refusal
    /// rules as [`Self::begin_free_transform`].
    pub fn begin_transform_mode(self: Pin<&mut Self>, path: &QString, mode: &QString) -> bool {
        let Some(mode) = TransformMode::parse(&mode.to_string()) else {
            return false;
        };
        self.begin_transform_session(path, mode)
    }

    fn begin_transform_session(
        mut self: Pin<&mut Self>,
        path: &QString,
        mode: TransformMode,
    ) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if !self
            .rust()
            .slot_allows(super::super::document_session::SessionKind::Transform)
        {
            return false;
        }
        self.as_mut().rust_mut().transform_controls = false;
        let mut path = path.to_string();
        if path.is_empty() {
            let Some(active) = self.rust().layer_target.single().map(str::to_string) else {
                return false;
            };
            path = active;
        }
        {
            let rust = self.rust();
            if let Some(session) = rust.session.live.transform() {
                // A lifted session's target is its source; the floating layer's
                // path is an implementation detail that may name a sibling.
                let target = session.lifted.as_ref().map_or(&session.path, |(s, _)| s);
                if *target == path && session.mode == mode {
                    return true;
                }
            }
        }
        // The outgoing session's floating layer goes back into the document,
        // so the display has to follow the swap.
        let was_lifted = self
            .rust()
            .session
            .live
            .transform()
            .is_some_and(|session| session.lifted.is_some());
        let Some(session) = self.as_mut().rust_mut().prepare_transform(&path, mode) else {
            return false;
        };
        let bounded = session.lifted.is_none();
        let members: Vec<String> = session
            .members
            .iter()
            .map(|(member, _)| member.clone())
            .collect();
        if !self.as_mut().compute_transform_preview(
            &session.path,
            &members,
            session.orig_rect,
            bounded,
        ) {
            self.as_mut().rust_mut().restore_transform_begin(session);
            return false;
        }
        self.as_mut().rust_mut().session.live.set_transform(session);
        if was_lifted {
            self.as_mut().recomposite();
        }
        true
    }

    /// Compute and store the transform overlay base/layer for `path`; keyed with
    /// the `-1` sentinel so the Move cache rebuilds when it next runs.
    pub(super) fn compute_transform_preview(
        mut self: Pin<&mut Self>,
        path: &str,
        members: &[String],
        orig_rect: PsdRect,
        bounded: bool,
    ) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        // The preview is built before the session opens; one that does not
        // fit is refused rather than left to the OOM killer (#1297).
        let fits = rust.session.doc.as_ref().is_some_and(|doc| {
            memory_gate::admit_fresh(transform_preview_bytes(
                doc, path, members, orig_rect, bounded,
            ))
        });
        if !fits {
            return false;
        }
        let prepared = rust.session.doc.as_ref().and_then(|doc| {
            build_transform_preview(doc, path, members, orig_rect, gpu_compute, bounded)
        });
        let Some(preview) = prepared else {
            return false;
        };
        rust.move_base = Some(preview.base);
        rust.move_base_origin = preview.base_origin;
        rust.move_view = None;
        rust.move_layer = Some(preview.image);
        rust.transform_preview_level = preview.level;
        rust.move_x = preview.x;
        rust.move_y = preview.y;
        rust.move_above = None;
        rust.move_above_origin = None;
        rust.move_above_scale = 1;
        rust.move_above_presented = None;
        rust.move_opacity = preview.opacity;
        rust.move_prepared_revision = rust.session.content_revision;
        rust.move_prepared_layer = String::new();
        rust.move_prepared_union = None;
        true
    }

    /// Clear the session; a lifted selection's pixels go back where they were.
    pub fn cancel_transform(mut self: Pin<&mut Self>) {
        self.as_mut().drop_transform_session();
        self.as_mut().clear_transform_preview();
    }

    /// Drop any session, restoring the document from before a lift, or the
    /// layer's untrimmed extent (the Move tool's controls box opens on every
    /// selected layer, and a trimmed layer would clip later filters).
    fn drop_transform_session(mut self: Pin<&mut Self>) {
        let session = self.as_mut().rust_mut().session.live.take_transform();
        if let Some(session) = &session {
            for (member, rect) in &session.untrimmed {
                if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
        }
        if let Some((_, before)) = session.and_then(|session| session.lifted) {
            self.as_mut().rust_mut().session.doc = Some(*before);
            self.as_mut().recomposite();
        }
    }

    /// Commit the session: one transform op (the similarity, an affine quad
    /// when a shear is set, or `transform_layer_quad` in a projective mode), or
    /// when a warp is stored the warp followed by the box's frame, a
    /// recomposite, and one `"Free Transform"` history state on success. An
    /// identity transform or warp records nothing; an engine refusal records
    /// nothing. The session always clears.
    pub fn commit_transform(mut self: Pin<&mut Self>) -> bool {
        memory_gate::clear();
        let Some((
            path,
            members,
            projective,
            quad,
            preview_quad,
            params,
            orig_rect,
            interpolation,
            warp,
            history_label,
        )) = self.rust().session.live.transform().map(|session| {
            let quad = session_quad(session);
            let preview_quad = if session.warp.is_some() {
                super::warp::warp_preview_quad(session).unwrap_or(quad)
            } else {
                quad
            };
            let members: Vec<(String, PsdRect)> = session
                .members
                .iter()
                .map(|(member, rect)| (member.clone(), *rect))
                .collect();
            (
                session.path.clone(),
                members,
                quad_driven(session),
                quad,
                preview_quad,
                MapParams::of(session),
                session.orig_rect,
                session.interpolation,
                session.warp.clone(),
                session.history_label.clone(),
            )
        })
        else {
            self.as_mut().clear_transform_preview();
            return false;
        };
        let repeat = self
            .rust()
            .session
            .live
            .transform()
            .and_then(repeatable_transform);
        let unmoved = || {
            quad.iter()
                .zip(source_corners(orig_rect).iter())
                .all(|(q, s)| (q.0 - s.0).abs() <= 1e-9 && (q.1 - s.1).abs() <= 1e-9)
        };
        let identity = match &warp {
            Some(warp) => warp.is_identity() && unmoved(),
            None if projective => unmoved(),
            None => params.is_identity(),
        };
        let cost = commit_cost(&preview_quad, orig_rect);
        if identity || !memory_gate::admit_spilling(cost.working, cost.spillable) {
            self.as_mut().drop_transform_session();
            self.as_mut().clear_transform_preview();
            return false;
        }
        let area = u64::from(orig_rect.width().max(0) as u32)
            * u64::from(orig_rect.height().max(0) as u32);
        // A large engine runs as a cancellable job (#1278); the session stays
        // live until it lands, so a cancel resumes it.
        if area >= DEFERRED_TRANSFORM_PIXELS {
            let lifted = self
                .rust()
                .session
                .live
                .transform()
                .is_some_and(|s| s.lifted.is_some());
            let (paths, apply) = if lifted {
                let apply = lifted_apply(
                    path.clone(),
                    projective,
                    quad,
                    params,
                    interpolation,
                    warp,
                    preview_quad,
                    orig_rect,
                );
                (vec![path], apply)
            } else {
                let paths = members.iter().map(|(member, _)| member.clone()).collect();
                let apply = group_apply(
                    members,
                    projective,
                    quad,
                    params,
                    interpolation,
                    warp,
                    preview_quad,
                    orig_rect,
                );
                (paths, apply)
            };
            let spec = super::super::impl_core::TransformSpec {
                paths,
                orig_rect,
                apply,
            };
            return self
                .as_mut()
                .rust_mut()
                .defer_transform(&history_label, spec, lifted, repeat);
        }
        let mut session = self.as_mut().rust_mut().session.live.take_transform();
        let lifted = session.as_mut().and_then(|session| session.lifted.take());
        if let Some((_, before)) = lifted {
            let changed = match (self.as_mut().rust_mut().session.doc.as_mut(), &warp) {
                (Some(doc), Some(warp)) => pictura_render::transform_layer_warp(
                    doc,
                    &path,
                    warp,
                    Some(preview_quad),
                    interpolation,
                ),
                (Some(doc), None) => apply_session_transform(
                    doc,
                    &path,
                    projective,
                    quad,
                    params,
                    orig_rect,
                    interpolation,
                ),
                (None, _) => false,
            };
            let merged = changed
                && self
                    .as_mut()
                    .rust_mut()
                    .session
                    .doc
                    .as_mut()
                    .is_some_and(|doc| pictura_render::merge_lifted(doc, &path));
            if !merged {
                self.as_mut().rust_mut().session.doc = Some(*before);
                self.as_mut().recomposite();
                self.as_mut().clear_transform_preview();
                return false;
            }
            // ponytail: CS6 carries the selection border through the transform;
            // it is dropped here rather than left outlining the old pixels.
            self.as_mut().rust_mut().session.set_selection(None);
            self.as_mut().rust_mut().last_transform = repeat;
            // A lifted transform changes pixels, not the layer tree, so it
            // keeps the link set; only the pixels moved.
            self.as_mut()
                .commit_transform_edit(&history_label, true, |_| Edit::Full);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
            self.as_mut().clear_transform_preview();
            return true;
        }

        let apply = group_apply(
            members.clone(),
            projective,
            quad,
            params,
            interpolation,
            warp.clone(),
            preview_quad,
            orig_rect,
        );
        let changed = self
            .as_mut()
            .rust_mut()
            .session
            .doc
            .as_mut()
            .is_some_and(|doc| (apply)(doc));
        if changed {
            self.as_mut().rust_mut().last_transform = repeat;
            let edit = self.rust().session.doc.as_ref().map_or(Edit::Full, |doc| {
                transform_group_edit(doc, &members, orig_rect)
            });
            self.as_mut()
                .commit_transform_edit(&history_label, true, |_| edit);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
        }
        self.as_mut().clear_transform_preview();
        changed
    }
}

impl qobject::PictureView {
    /// Cache the base composite (every move-group layer hidden), the moving
    /// image, its document-space origin, and opacity. One region composite at
    /// drag start, derived from the authoritative `doc.composite`.
    ///
    /// Reuses the cached base when the content revision and move-group key
    /// match; otherwise it recomputes.
    pub fn begin_move_preview(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        memory_gate::clear();
        if let Some(path) = super::mask_move::target(self.rust()) {
            return self.begin_mask_move(path);
        }
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        // A committed move leaves the cached base valid: its content and the
        // hidden-layer backdrop do not change with the layer's position. Reuse
        // it before settling a pending composite, which is a whole-document
        // composite on a large document.
        if self.as_ref().move_cache_valid(&paths) {
            let union = self
                .rust()
                .session
                .doc
                .as_ref()
                .and_then(|doc| move_group_union(doc, &paths));
            let moved = union != self.rust().move_prepared_union;
            // A whole-document base is position-independent. A patch cropped to
            // the content rect is tied to the rect it was built over: its
            // backdrop (including the layers above the group) would shift with
            // the layer, so a committed move must rebuild it rather than
            // translate it (#1250).
            let whole = {
                let rust = self.rust();
                rust.move_base_origin.is_none()
                    || rust.session.doc.as_ref().is_some_and(|doc| {
                        rust.move_base_origin == Some((0, 0))
                            && rust.move_base.as_ref().is_some_and(|base| {
                                base.width() == doc.width as i32
                                    && base.height() == doc.height as i32
                            })
                    })
            };
            if moved && !whole && !self.as_mut().refresh_move_base(&paths) {
                // The patch could not be rebuilt; fall through to a full
                // preview compute.
            } else {
                {
                    let mut rust = self.as_mut().rust_mut();
                    if moved {
                        // A commit trims each member to its content.
                        rust.move_layer_offset = (0, 0);
                        rust.move_prepared_union = union;
                    }
                    let (x, y, opacity) = rust
                        .session
                        .doc
                        .as_ref()
                        .and_then(|doc| move_origin_opacity(doc, &paths))
                        .unwrap_or((0, 0, 0));
                    let (ox, oy) = rust.move_layer_offset;
                    rust.move_x = x + ox;
                    rust.move_y = y + oy;
                    rust.move_opacity = opacity;
                    rust.move_preview_cache_hit = true;
                    rust.move_drag_active = true;
                }
                return true;
            }
        }
        // A view preview reads the layer stack, not the stale composite, so a
        // pending commit can stay on its worker while the next drag starts.
        if self.as_ref().rust().move_composite_pending()
            && self.as_mut().rust_mut().compute_view_move_preview(&paths)
        {
            let mut rust = self.as_mut().rust_mut();
            rust.move_preview_cache_hit = false;
            rust.move_drag_active = true;
            return true;
        }
        self.as_mut().settle_pending_composite();
        let computed = self.as_mut().compute_move_preview(paths);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.move_preview_cache_hit = false;
            rust.move_drag_active = computed;
        }
        computed
    }

    /// Begin a duplicate drag. With a selection active this is the Alt
    /// selection-content case: the selected pixels are prepared as the live
    /// move-preview layer, no document change. Otherwise the active pixel layer
    /// is cloned, made active, and previewed.
    pub fn begin_move_duplicate(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        self.as_mut().settle_pending_composite();
        if self.rust().session.selection.is_some() {
            return self.as_mut().begin_selection_duplicate_preview();
        }
        let baseline = self
            .rust()
            .session
            .doc
            .clone()
            .map(|doc| (doc, self.rust().layer_target.clone()));
        let new_path = {
            let mut guard = self.as_mut().rust_mut();
            let rust = &mut *guard;
            let (doc, layer_target) = (&mut rust.session.doc, &mut rust.layer_target);
            let doc = doc.as_mut();
            let duplicate = doc.and_then(|d| duplicate_move_target(d, layer_target));
            let Some(new_path) = duplicate else {
                return false;
            };
            new_path
        };
        self.as_mut().recomposite();
        let begun = self.as_mut().compute_move_preview(vec![new_path]);
        if begun {
            self.as_mut().rust_mut().session.quick_mask.move_origin = baseline;
        }
        begun
    }

    /// Warm the move-preview cache without entering preview mode. A layer the
    /// Move tool may not move (a Background) is never previewed, so it is not
    /// warmed either: the warm composites the whole document.
    pub fn prepare_move_preview(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        if self.as_ref().move_cache_valid(&paths) {
            return true;
        }
        self.as_mut().settle_pending_composite();
        self.as_mut().compute_move_preview(paths)
    }

    pub fn move_preview_cache_hit(&self) -> bool {
        self.rust().move_preview_cache_hit
    }

    /// The paths the Move tool acts on: every selected layer plus every movable
    /// member of a selected layer's link set, restricted to movable
    /// (pixel/shape), visible, position-unlocked layers and ordered by panel
    /// position (topmost-first). `None` when the selection has no such layer.
    pub(super) fn move_group_paths(&self) -> Option<Vec<String>> {
        if let Some(path) = super::mask_move::target(self.rust()) {
            return Some(vec![path]);
        }
        let rust = self.rust();
        let doc = rust.session.doc.as_ref()?;
        if rust.layer_target.is_empty() {
            return None;
        }
        let mut wanted: Vec<String> = Vec::new();
        for path in rust.layer_target.selected() {
            for member in link_members(rust, doc, path) {
                if wanted.contains(&member) {
                    continue;
                }
                let Some(layer) = pictura_render::resolve_path(doc, &member) else {
                    continue;
                };
                if movable_layer(doc, &member).is_some()
                    && layer.visible
                    && !layer_move_locked(layer)
                {
                    wanted.push(member);
                }
            }
        }
        if wanted.is_empty() {
            return None;
        }
        let group: Vec<String> = pictura_render::flatten_rows(doc)
            .into_iter()
            .map(|(path, _)| path)
            .filter(|path| wanted.contains(path))
            .collect();
        Some(group)
    }

    /// Rebuild the cached Move backdrop patch for `paths` after their rects
    /// moved. The patch is cropped to the content rect, so its backdrop is tied
    /// to the rect it was built over; the moving image and the above-layer
    /// overlay stay valid, so only the base is recomposited.
    fn refresh_move_base(mut self: Pin<&mut Self>, paths: &[String]) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        let (base, origin) = match backdrop_patch(doc, paths, gpu_compute) {
            Some((base, origin)) => (base, Some(origin)),
            None => (build_move_preview_base(doc, paths, gpu_compute), None),
        };
        rust.move_base = Some(base);
        rust.move_base_origin = origin;
        true
    }

    fn move_cache_valid(&self, paths: &[String]) -> bool {
        let rust = self.rust();
        // The base image is the document with every move-group layer hidden, so
        // its pixels do not depend on their positions: a committed move leaves
        // it valid. Its *placement* does, though: a patch base is cropped to the
        // content rect at the press, and the moving image's origin follows the
        // layer rect. The move commit deliberately does not bump
        // `content_revision`, so the group's union rect is the cheap signal that
        // the layers moved; `begin_move_preview` refreshes a single layer's
        // patch. A rendered group's moving image is composited at the layers'
        // positions and clipped to the canvas, so any move makes it stale and
        // the group must recompute. A shape's moving image is rendered at its
        // outline, which a move shifts without changing the layer rect, so a
        // group with one is never reused.
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        if paths.len() > 1 && move_group_union(doc, paths) != rust.move_prepared_union {
            return false;
        }
        let shape = paths.iter().any(|path| {
            pictura_render::resolve_path(doc, path).is_some_and(pictura_render::is_shape_layer)
        });
        // A view preview covers only what the canvas showed at its press. The
        // above-layer image covers the presented rect, so a pan or zoom between
        // drags makes it stale even when the base is still valid.
        let presented = (rust.presented_level, rust.presented_rect);
        !shape
            && rust.move_view.is_none()
            && rust.move_base.is_some()
            && (rust.move_above.is_none() || rust.move_above_presented == Some(presented))
            && rust.move_prepared_revision == rust.session.content_revision
            && rust.move_prepared_layer == move_group_key(paths)
    }

    /// Compute and store the move-preview base/layer for `paths` and record the
    /// revision and group key they were built from.
    fn compute_move_preview(mut self: Pin<&mut Self>, paths: Vec<String>) -> bool {
        if self.as_mut().rust_mut().compute_view_move_preview(&paths) {
            return true;
        }
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let presented = (rust.presented_level, rust.presented_rect);
        let above = rust.session.doc.as_ref().and_then(|doc| {
            above_patch(
                doc,
                &paths,
                presented.0.max(0) as u32,
                presented.1,
                gpu_compute,
            )
        });
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        let mut offset = (0, 0);
        let moving = if let [path] = paths.as_slice() {
            let Some(layer) = pictura_render::resolve_path(doc, path) else {
                return false;
            };
            // A shape layer has no pixels of its own: its fill cut to the
            // outline is rendered.
            if pictura_render::is_shape_layer(layer) {
                rendered_layer_image(doc, path, gpu_compute)
            } else {
                // A painted layer's rect is often the whole canvas around a
                // small mark: the moving image covers the mark (#1207).
                let rect = pictura_render::content_rect(doc, path)
                    .and_then(|content| content.intersect(layer.rect))
                    .unwrap_or(layer.rect);
                // The planar copy, its RGBA bytes, the image, and the copy the
                // canvas takes of it.
                if !memory_gate::admit(16 * memory_gate::area(rect)) {
                    return false;
                }
                offset = (rect.left - layer.rect.left, rect.top - layer.rect.top);
                if needs_rendered_appearance(layer) {
                    moving_layer_image(doc, path, rect, 0, gpu_compute)
                        .map(|image| (image, rect.left, rect.top))
                } else {
                    layer_image_within(layer, rect).map(|image| (image, rect.left, rect.top))
                }
            }
        } else {
            rendered_move_group(doc, &paths, gpu_compute)
        };
        let Some((layer_image, x, y)) = moving else {
            return false;
        };
        // A single layer's own image carries no opacity; the preview applies it
        // once. A rendered group already has each member's opacity baked in.
        let opacity = if let [path] = paths.as_slice() {
            pictura_render::resolve_path(doc, path).map_or(255, |layer| layer.opacity as i32)
        } else {
            255
        };
        let (base, origin) = match backdrop_patch(doc, &paths, gpu_compute) {
            Some((base, origin)) => (base, Some(origin)),
            None => (build_move_preview_base(doc, &paths, gpu_compute), None),
        };
        rust.move_base = Some(base);
        rust.move_base_origin = origin;
        rust.move_view = None;
        rust.move_layer = Some(layer_image);
        rust.move_x = x;
        rust.move_y = y;
        rust.move_layer_offset = offset;
        rust.move_opacity = opacity;
        let (above_image, above_origin, above_scale) = match above {
            Some((image, origin)) => (Some(image), Some(origin), 1i32 << presented.0.clamp(0, 16)),
            None => (None, None, 1),
        };
        rust.move_above = above_image;
        rust.move_above_origin = above_origin;
        rust.move_above_scale = above_scale;
        rust.move_above_presented = above_origin.map(|_| presented);
        rust.move_prepared_revision = rust.session.content_revision;
        rust.move_prepared_layer = move_group_key(&paths);
        rust.move_prepared_union = move_group_union(doc, &paths);
        true
    }

    pub fn move_preview_base(&self) -> QImage {
        self.rust().move_base.clone().unwrap_or_default()
    }

    /// The layers above the moving group over transparency, drawn back over the
    /// moving image so the preview keeps the stacking order (#1250); null when
    /// the group is topmost.
    pub fn move_preview_above(&self) -> QImage {
        self.rust().move_above.clone().unwrap_or_default()
    }

    pub fn move_preview_layer(&self) -> QImage {
        self.rust().move_layer.clone().unwrap_or_default()
    }

    pub fn move_preview_x(&self) -> i32 {
        self.rust().move_x
    }

    pub fn move_preview_y(&self) -> i32 {
        self.rust().move_y
    }

    pub fn move_preview_opacity(&self) -> i32 {
        self.rust().move_opacity
    }

    /// Rebuild a view preview's moving image when a drag of `(dx, dy)` has
    /// carried the on-screen part of the layer past the region it covers.
    /// True when the image (and its origin) changed.
    pub(in crate::cxxqt_object) fn follow_move_preview(
        mut self: Pin<&mut Self>,
        dx: i32,
        dy: i32,
    ) -> bool {
        if self.rust().mask_move_baseline.is_some() {
            return self.render_mask_move_preview(dx, dy);
        }
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let (Some(view), Some(doc)) = (rust.move_view, rust.session.doc.as_ref()) else {
            return false;
        };
        let Some(path) = rust.move_prepared_layer.lines().next() else {
            return false;
        };
        let Some((image, view)) = view_preview::follow(doc, path, view, dx, dy, rust.gpu_compute)
        else {
            return false;
        };
        rust.move_layer = Some(image);
        rust.move_x = view.region.left;
        rust.move_y = view.region.top;
        rust.move_view = Some(view);
        true
    }

    pub fn end_move_preview(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().session.quick_mask.move_origin = None;
        self.as_mut().end_mask_move();
        // Keep the cache: the next press reuses a full-resolution preview when
        // the revision, topmost layer, and clamped rect are unchanged. A view
        // preview is rebuilt on every press (`move_cache_valid`).
        self.as_mut().rust_mut().move_drag_active = false;
    }

    /// Drop the transform overlay's prepared base/layer. The C++ canvas clears
    /// its copy separately; this keeps the Rust side from handing a stale
    /// preview to a later undo or move.
    pub fn clear_transform_preview(mut self: Pin<&mut Self>) {
        self.as_mut().end_mask_move();
        let mut rust = self.rust_mut();
        rust.move_base = None;
        rust.move_base_origin = None;
        rust.move_view = None;
        rust.move_layer = None;
        rust.transform_preview_level = 0;
        rust.move_x = 0;
        rust.move_y = 0;
        rust.move_above = None;
        rust.move_above_origin = None;
        rust.move_above_scale = 1;
        rust.move_above_presented = None;
        rust.move_layer_offset = (0, 0);
        rust.move_opacity = 0;
        rust.move_prepared_revision = 0;
        rust.move_prepared_layer.clear();
        rust.move_prepared_union = None;
        rust.move_preview_cache_hit = false;
        rust.move_drag_active = false;
    }

    pub fn commit_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        memory_gate::clear();
        if dx == 0 && dy == 0 {
            return false;
        }
        if let Some(path) = super::mask_move::target(self.rust()) {
            return self.commit_mask_move(path, dx, dy);
        }
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        let dirty = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            let original = doc.clone();
            let dirty = translate_move_group(doc, &paths, dx, dy);
            let cost = dirty.map(move_commit_cost);
            if cost.is_some_and(|c| !memory_gate::admit_spilling(c.working, c.spillable)) {
                *doc = original;
                return false;
            }
            if dirty.is_some() {
                for path in &paths {
                    pictura_render::trim_to_content(doc, path);
                }
            }
            dirty
        };
        let Some(dirty) = dirty else {
            return false;
        };
        self.as_mut()
            .commit_move_edit("Move Layer", |_| Edit::regions(vec![dirty], false));
        // A deferred commit reports when its composite lands; a synchronous one
        // reports now so the canvas drops the drag preview either way.
        if !self.rust().move_composite_pending() {
            self.as_mut().move_committed();
        }
        true
    }

    pub fn commit_move_legacy(mut self: Pin<&mut Self>) -> bool {
        if self.rust().session.doc.is_none() {
            return false;
        }
        self.as_mut().commit_move_edit("Move Layer", |_| Edit::Full);
        true
    }
}

impl PictureViewRust {
    /// Build and store a view preview of `paths` when the canvas presents one.
    /// It reads the layer stack, not `doc.composite`, so it stays valid while a
    /// Move commit's composite is still on its worker.
    pub(in crate::cxxqt_object) fn compute_view_move_preview(&mut self, paths: &[String]) -> bool {
        let Some(doc) = self.session.doc.as_ref() else {
            return false;
        };
        let Some(preview) = view_preview::view_preview(
            doc,
            paths,
            self.presented_level,
            self.presented_rect,
            self.gpu_compute,
        ) else {
            return false;
        };
        let Some(opacity) =
            pictura_render::resolve_path(doc, &paths[0]).map(|layer| layer.opacity as i32)
        else {
            return false;
        };
        let above = above_patch(
            doc,
            paths,
            self.presented_level.max(0) as u32,
            self.presented_rect,
            self.gpu_compute,
        );
        let union = move_group_union(doc, paths);
        self.move_base = Some(preview.base);
        self.move_base_origin = Some(preview.base_origin);
        self.move_layer = Some(preview.layer);
        self.move_x = preview.view.region.left;
        self.move_y = preview.view.region.top;
        self.move_opacity = opacity;
        self.move_view = Some(preview.view);
        let (above_image, above_origin) = match above {
            Some((image, origin)) => (Some(image), Some(origin)),
            None => (None, None),
        };
        self.move_above = above_image;
        self.move_above_origin = above_origin;
        self.move_above_scale = 1i32 << self.presented_level.clamp(0, 16);
        self.move_above_presented =
            above_origin.map(|_| (self.presented_level, self.presented_rect));
        self.move_prepared_revision = self.session.content_revision;
        self.move_prepared_layer = move_group_key(paths);
        self.move_prepared_union = union;
        true
    }

    /// Prepare a transform session on `path` in `mode`: the layer is trimmed to
    /// its content (or the selection lifted onto a floating layer) and the
    /// session's scalars are set. `None` when the target cannot be
    /// transformed; the caller restores the document when the preview fails.
    pub(in crate::cxxqt_object) fn prepare_transform(
        &mut self,
        path: &str,
        mode: TransformMode,
    ) -> Option<TransformSession> {
        let lift = lifts_selection(self, path);
        if !can_begin_mode(self, path, mode) {
            return None;
        }
        // The outgoing session goes first: its trims and its floating layer go
        // back, so the new session starts from the same document.
        let outgoing = self.session.live.take_transform();
        if let Some(session) = &outgoing {
            for (member, rect) in &session.untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
        }
        if let Some((_, before)) = outgoing.and_then(|session| session.lifted) {
            self.session.doc = Some(*before);
        }
        let mut path = path.to_string();
        let mut lifted = None;
        let mut untrimmed: Vec<(String, PsdRect)> = Vec::new();
        let mut members: Vec<(String, PsdRect)> = Vec::new();
        if lift {
            let coverage = lift_coverage(self).unwrap_or_default();
            let doc = self.session.doc.as_mut()?;
            let before = Box::new(doc.clone());
            let floating = pictura_render::lift_selection(doc, &path, &coverage);
            if floating.is_empty() {
                *doc = *before;
                return None;
            }
            lifted = Some((std::mem::replace(&mut path, floating), before));
            let rect = self
                .session
                .doc
                .as_ref()
                .and_then(|doc| transform_rect(doc, &path));
            if let Some(rect) = rect {
                members.push((path.clone(), rect));
            }
        } else {
            let group = {
                let doc = self.session.doc.as_ref()?;
                transform_group(self, doc, &path)
            };
            for member in group {
                // The box hugs each layer's visible pixels, not its (often
                // canvas-sized) rect; the composite is unchanged. A hidden
                // linked partner is trimmed too: it transforms with the set.
                let before = self
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &member))
                    .map(|layer| layer.rect);
                let trimmed = self
                    .session
                    .doc
                    .as_mut()
                    .is_some_and(|doc| pictura_render::trim_to_content(doc, &member));
                if trimmed {
                    if let Some(rect) = before {
                        untrimmed.push((member.clone(), rect));
                    }
                }
                if let Some(rect) = self
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| transform_rect(doc, &member))
                {
                    if rect.width() > 0 && rect.height() > 0 {
                        members.push((member, rect));
                    }
                }
            }
        }
        let Some(orig_rect) = members.iter().map(|(_, rect)| *rect).reduce(PsdRect::union) else {
            // A refused begin leaves every layer as it was.
            for (member, rect) in &untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
            if let Some((_, before)) = lifted {
                self.session.doc = Some(*before);
            }
            return None;
        };
        // Any editable type in the set keeps the projective gestures a
        // parallelogram (and refuses Distort/Perspective), as a single type
        // layer does.
        let type_layer = self.session.doc.as_ref().is_some_and(|doc| {
            members.iter().any(|(member, _)| {
                pictura_render::resolve_path(doc, member).is_some_and(|layer| layer.is_type())
            })
        });
        let corners = source_corners(orig_rect);
        let center = rect_center(orig_rect);
        Some(TransformSession {
            path,
            members,
            orig_rect,
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            dx: 0.0,
            dy: 0.0,
            handle: -1,
            press_x: 0.0,
            press_y: 0.0,
            start: [1.0, 1.0, 0.0, 0.0, 0.0],
            dragging: false,
            mode,
            type_layer,
            quad: mode.is_projective().then_some(corners),
            base_quad: None,
            free_perspective: false,
            free_distort: false,
            free_skew: false,
            perspective_side: None,
            start_quad: corners,
            pivot: center,
            pivot_origin: center,
            start_pivot: center,
            show_pivot: true,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            warp_editing: false,
            warp_post: None,
            lifted,
            untrimmed,
            history_label: "Free Transform".to_string(),
        })
    }

    /// Put back what a refused transform begin changed: the pre-trim layer
    /// extent and the pre-lift document.
    pub(in crate::cxxqt_object) fn restore_transform_begin(&mut self, session: TransformSession) {
        for (member, rect) in &session.untrimmed {
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::untrim_to(doc, member, *rect);
            }
        }
        if let Some((_, before)) = session.lifted {
            self.session.doc = Some(*before);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod cow_tests;
#[cfg(test)]
mod move_profile;
#[cfg(test)]
mod preview_tests;
