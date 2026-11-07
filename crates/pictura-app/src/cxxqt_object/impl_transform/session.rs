use super::super::document_session::{RepeatTransform, TransformMode, TransformSession};
use super::super::helpers::*;
use super::super::helpers_composite::*;
use super::super::qobject;
use super::super::state::PictureViewRust;
use super::geometry::{
    apply_session_transform, quad_driven, rect_center, session_quad, source_corners, MapParams,
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
use pictura_core::{layer_move_locked, BlendMode, LayerStack, LockFlags, PsdRect};
use pictura_render::Interpolation;

/// A transform whose layer rect reaches this area runs its engine on a worker.
const DEFERRED_TRANSFORM_PIXELS: u64 = 4_000_000;

mod preview;
mod view_preview;

pub(in crate::cxxqt_object) use preview::transform_preview_layer;
#[cfg(test)]
use preview::TransformPreview;
use preview::{backdrop_patch, build_transform_preview, rendered_layer_image};
use view_preview::above_patch;
pub(in crate::cxxqt_object) use view_preview::MoveView;

/// Whether `path` resolves to a transformable target: a raster pixel layer, a
/// shape layer with an outline of some area, or a channel-less embedded smart
/// object whose source materializes. False for a
/// hidden layer, a group, adjustment layer, Background layer, position-locked
/// layer, or a zero-area target.
fn can_free_transform(doc: &pictura_core::Document, path: &str) -> bool {
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return false;
    };
    if pictura_render::is_shape_layer(layer) {
        return layer.visible
            && !layer.lock.contains(LockFlags::POSITION)
            && pictura_render::shape_layer_rect(layer, doc.width, doc.height)
                .is_some_and(|rect| rect.width() > 0 && rect.height() > 0);
    }
    if !layer.visible
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

/// The repeatable part of a committed session: the similarity parameters and
/// the reference point's offset from the source rect's centre. Projective and
/// Warp sessions are not repeatable through `Transform Again`.
fn repeatable_transform(session: &TransformSession) -> Option<RepeatTransform> {
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
    !(matches!(mode, TransformMode::Distort | TransformMode::Perspective)
        && !lift
        && pictura_render::resolve_path(doc, path).is_some_and(|layer| layer.is_type()))
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
        if !self
            .as_mut()
            .compute_transform_preview(&session.path, bounded)
        {
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
        bounded: bool,
    ) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let prepared = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| build_transform_preview(doc, path, gpu_compute, bounded));
        let Some(preview) = prepared else {
            return false;
        };
        rust.move_base = Some(preview.base);
        rust.move_base_origin = preview.base_origin;
        rust.move_view = None;
        rust.move_layer = Some(preview.image);
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
        if let Some((path, rect)) = session
            .as_ref()
            .and_then(|s| s.untrimmed.map(|rect| (s.path.clone(), rect)))
        {
            if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
                pictura_render::untrim_to(doc, &path, rect);
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
            (
                session.path.clone(),
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
        if identity || !memory_gate::admit(transform_commit_bytes(&preview_quad, orig_rect)) {
            self.as_mut().drop_transform_session();
            self.as_mut().clear_transform_preview();
            return false;
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
            self.as_mut()
                .commit_transform_edit(&history_label, true, |_| Edit::Structural { damage: None });
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
            self.as_mut().clear_transform_preview();
            return true;
        }

        // The engine resamples the layer's pixels; on a large layer it belongs
        // on a worker. The landing installs the transformed layer and hands its
        // composite to the same deferred edit a Move commit uses.
        let engine_path = path.clone();
        let engine_quad = quad;
        let engine_params = params;
        let engine_orig = orig_rect;
        let engine_interp = interpolation;
        let engine_projective = projective;
        let engine_warp = warp.clone();
        let engine_preview_quad = preview_quad;
        let apply =
            std::sync::Arc::new(move |doc: &mut pictura_core::Document| match &engine_warp {
                Some(warp) => pictura_render::transform_layer_warp(
                    doc,
                    &engine_path,
                    warp,
                    Some(engine_preview_quad),
                    engine_interp,
                ),
                None => apply_session_transform(
                    doc,
                    &engine_path,
                    engine_projective,
                    engine_quad,
                    engine_params,
                    engine_orig,
                    engine_interp,
                ),
            });
        let area = u64::from(orig_rect.width().max(0) as u32)
            * u64::from(orig_rect.height().max(0) as u32);
        if area < DEFERRED_TRANSFORM_PIXELS {
            let changed = self
                .as_mut()
                .rust_mut()
                .session
                .doc
                .as_mut()
                .is_some_and(|doc| (apply)(doc));
            if changed {
                self.as_mut().rust_mut().last_transform = repeat;
                self.as_mut().clear_link_sets();
                let after = self
                    .rust()
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &path))
                    .map(|layer| (layer.rect, layer_has_effects(layer)));
                let edit = match after {
                    Some((after, false)) => Edit::regions(vec![orig_rect.union(after)], false),
                    _ => Edit::Full,
                };
                self.as_mut()
                    .commit_transform_edit(&history_label, true, |_| edit);
                if !self.rust().move_composite_pending() {
                    self.as_mut().move_committed();
                }
            }
            self.as_mut().clear_transform_preview();
            return changed;
        }
        let spec = super::super::impl_core::TransformSpec {
            path: path.clone(),
            orig_rect,
            apply,
        };
        self.as_mut().rust_mut().last_transform = repeat;
        self.as_mut().clear_link_sets();
        let deferred = self
            .as_mut()
            .rust_mut()
            .defer_transform(&history_label, spec);
        if deferred {
            if let Some(job) = self.as_mut().rust_mut().pending_transform_job.take() {
                self.as_mut().build_transform(job);
            }
        }
        self.as_mut().clear_transform_preview();
        deferred
    }

    /// Whether a Free Transform session is active.
    pub fn transform_session_active(&self) -> bool {
        self.rust().session.live.transform().is_some()
    }

    /// The handle the live transform drag grabbed, or -1 when no drag is live.
    pub fn transform_drag_handle(&self) -> i32 {
        self.rust()
            .session
            .live
            .transform()
            .filter(|session| session.dragging)
            .map_or(-1, |session| session.handle)
    }
    pub fn transform_session_path(&self) -> QString {
        self.rust()
            .session
            .live
            .transform()
            .map(|session| QString::from(session.path.as_str()))
            .unwrap_or_default()
    }

    pub fn transform_scale_x(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(1.0, |session| session.scale_x)
    }

    pub fn transform_scale_y(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(1.0, |session| session.scale_y)
    }

    pub fn transform_angle(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.angle)
    }

    pub fn transform_dx(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.dx)
    }

    pub fn transform_dy(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.dy)
    }
}

/// What a transform commit holds at its peak: the transformed layer's four
/// planes over the quad's box, and the composite of the box before and after
/// with its sRGB copy and display patch.
fn transform_commit_bytes(quad: &[(f64, f64); 4], orig_rect: PsdRect) -> u64 {
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
    4 * memory_gate::area(out) + 12 * memory_gate::area(out.union(orig_rect))
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
            {
                let mut rust = self.as_mut().rust_mut();
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

    fn move_cache_valid(&self, paths: &[String]) -> bool {
        let rust = self.rust();
        // The base is the document with every move-group layer hidden, so it
        // does not depend on their positions: a committed move leaves it valid.
        // The move commit deliberately does not bump `content_revision` for this
        // reason; keying on a layer rect here would throw the base away on
        // every drag and force a full region recomposite on the next press.
        // A shape's moving image is rendered at its outline, which a move
        // shifts without changing the layer rect, so a group with one is never
        // reused. A rendered group's moving image is composited at the layers'
        // positions and clipped to the canvas, so any move makes it stale; the
        // group's union rect must match too.
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let union = if paths.len() > 1 {
            move_group_union(doc, paths)
        } else {
            None
        };
        if union != rust.move_prepared_union {
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
                layer_image_within(layer, rect).map(|image| (image, rect.left, rect.top))
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
        rust.move_prepared_union = if paths.len() > 1 {
            move_group_union(doc, &paths)
        } else {
            None
        };
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
        let Some((image, view)) = view_preview::follow(doc, path, view, dx, dy) else {
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
            // The damage's composite, its sRGB copy, and the display patch.
            if dirty.is_some_and(|rect| !memory_gate::admit(12 * memory_gate::area(rect))) {
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
        self.move_prepared_union = None;
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
        // The outgoing session goes first: its trim and its floating layer go
        // back, so the new session starts from the same document.
        let outgoing = self.session.live.take_transform();
        if let Some(session) = &outgoing {
            if let Some(rect) = session.untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, &session.path, rect);
                }
            }
        }
        if let Some((_, before)) = outgoing.and_then(|session| session.lifted) {
            self.session.doc = Some(*before);
        }
        let mut path = path.to_string();
        let mut lifted = None;
        let mut untrimmed = None;
        if !lift {
            // The box hugs the layer's visible pixels, not its (often
            // canvas-sized) rect; the composite is unchanged.
            if let Some(doc) = self.session.doc.as_mut() {
                let before = pictura_render::resolve_path(doc, &path).map(|layer| layer.rect);
                if pictura_render::trim_to_content(doc, &path) {
                    untrimmed = before;
                }
            }
        } else {
            let coverage = lift_coverage(self).unwrap_or_default();
            let doc = self.session.doc.as_mut()?;
            let before = Box::new(doc.clone());
            let floating = pictura_render::lift_selection(doc, &path, &coverage);
            if floating.is_empty() {
                *doc = *before;
                return None;
            }
            lifted = Some((std::mem::replace(&mut path, floating), before));
        }
        // A shape layer's box is its outline's; its own rect is the canvas.
        let rect = self.session.doc.as_ref().and_then(|doc| {
            let layer = pictura_render::resolve_path(doc, &path)?;
            Some(
                pictura_render::shape_layer_rect(layer, doc.width, doc.height)
                    .unwrap_or(layer.rect),
            )
        });
        let Some(orig_rect) = rect else {
            // A refused begin leaves the layer as it was.
            if let Some(rect) = untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, &path, rect);
                }
            }
            if let Some((_, before)) = lifted {
                self.session.doc = Some(*before);
            }
            return None;
        };
        let type_layer = self
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .is_some_and(|layer| layer.is_type());
        let corners = source_corners(orig_rect);
        let center = rect_center(orig_rect);
        Some(TransformSession {
            path,
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
        if let Some(rect) = session.untrimmed {
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::untrim_to(doc, &session.path, rect);
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
