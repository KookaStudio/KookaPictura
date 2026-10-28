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
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QRect, QString};
use pictura_core::{layer_move_locked, BlendMode, LayerStack, LockFlags};
use pictura_render::Interpolation;

/// Whether the layer's channels alone are not its whole appearance: a layer
/// mask, vector mask, layer style, fill adjustment, or type needs the
/// compositor to render the preview.
fn needs_rendered_appearance(layer: &pictura_core::Layer) -> bool {
    layer.mask.is_some()
        || layer.vector_mask.is_some()
        || layer.adjustment.is_some()
        || layer.is_type()
        || pictura_render::is_shape_layer(layer)
        || pictura_render::decode_layer_fill(layer).is_some()
        || pictura_render::has_layer_style(layer)
}

/// Render the layer alone over transparency and crop to its visible rect, so
/// the transform preview shows a mask, style, vector mask, fill, or type that
/// its raw channels do not carry. The layer's opacity and blend are
/// neutralized: the preview applies the layer opacity once itself.
fn rendered_layer_image(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let needs_materialize = {
        let layer = pictura_render::resolve_path(doc, path)?;
        !layer.channels.iter().any(|c| c.id == 0) && layer.smart_object.is_some()
    };
    let mut materialized = None;
    if needs_materialize {
        let mut clone = doc.clone();
        if !pictura_render::rasterize_smart_object(&mut clone, path) {
            return None;
        }
        materialized = Some(clone);
    }
    let source_doc = materialized.as_ref().unwrap_or(doc);
    let layer = pictura_render::resolve_path(source_doc, path)?;
    let rect = layer.rect;
    let (x0, y0) = (rect.left.max(0), rect.top.max(0));
    let (x1, y1) = (
        rect.right.min(source_doc.width as i32),
        rect.bottom.min(source_doc.height as i32),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut source = layer.clone();
    source.opacity = 255;
    source.blend = BlendMode::Normal;
    // A clone shares the pixel planes; replacing the layer list keeps the
    // document's ICC profile, pattern library, and type stylesheet, which the
    // compositor needs to reproduce the layer's appearance.
    let mut scratch = source_doc.clone();
    scratch.layers = LayerStack::from_layers(vec![source]);
    let rendered = current_buffer(&scratch, gpu_compute);
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image.copy(&QRect::new(x0, y0, x1 - x0, y1 - y0)), x0, y0))
}

/// The target path and the contiguous clipped layers directly above it in the
/// same parent. Hiding only the base would drop a clipped fill from the
/// preview, so the moving image carries the whole clipping group.
fn clipping_group(doc: &pictura_core::Document, path: &str) -> Vec<String> {
    let mut group = vec![path.to_string()];
    let Some((parent, index)) = (match path.rsplit_once('/') {
        Some((parent, last)) => last
            .parse::<usize>()
            .ok()
            .map(|index| (Some(parent), index)),
        None => path.parse::<usize>().ok().map(|index| (None, index)),
    }) else {
        return group;
    };
    let siblings: Option<&[pictura_core::Layer]> = match parent {
        Some(parent) => {
            pictura_render::resolve_path(doc, parent).map(|layer| layer.children.as_slice())
        }
        None => Some(doc.layers.as_slice()),
    };
    let Some(siblings) = siblings else {
        return group;
    };
    let mut i = index + 1;
    while siblings
        .get(i)
        .is_some_and(|layer: &pictura_core::Layer| layer.clipping)
    {
        group.push(match parent {
            Some(parent) => format!("{parent}/{i}"),
            None => i.to_string(),
        });
        i += 1;
    }
    group
}

/// Render a clipping group (the target plus the clipped layers above it) alone
/// over transparency, cropped to the group's union rect. The target's opacity
/// and blend are neutralized; the clipped layers keep theirs.
///
/// ponytail: the preview moves the clipped layers with the base; the commit
/// transforms only the base, so the clipped pixels' final position can differ.
fn rendered_clipping_group(
    doc: &pictura_core::Document,
    group: &[String],
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let mut layers = Vec::with_capacity(group.len());
    let mut bounds: Option<pictura_core::PsdRect> = None;
    for (i, path) in group.iter().enumerate() {
        let mut layer = pictura_render::resolve_path(doc, path)?.clone();
        if i == 0 {
            layer.opacity = 255;
            layer.blend = BlendMode::Normal;
        }
        bounds = Some(bounds.map_or(layer.rect, |rect| union_rect(rect, layer.rect)));
        layers.push(layer);
    }
    let bounds = bounds?;
    let (x0, y0) = (bounds.left.max(0), bounds.top.max(0));
    let (x1, y1) = (
        bounds.right.min(doc.width as i32),
        bounds.bottom.min(doc.height as i32),
    );
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut scratch = doc.clone();
    scratch.layers = LayerStack::from_layers(layers);
    let rendered = current_buffer(&scratch, gpu_compute);
    let image = buffer_to_image(&pictura_codec::buffer_to_srgb(&scratch, &rendered));
    Some((image.copy(&QRect::new(x0, y0, x1 - x0, y1 - y0)), x0, y0))
}

/// Build the transform overlay images for `path`: the document composited with
/// the target hidden, the target's own RGBA image (materializing a channel-less
/// embedded object into a clone), its document-space origin, and opacity.
pub(super) fn build_transform_preview(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, QImage, i32, i32, i32)> {
    let opacity = pictura_render::resolve_path(doc, path)?.opacity as i32;
    let (image, x, y) = transform_preview_layer(doc, path, gpu_compute)?;
    let mut base_doc = doc.clone();
    for target_path in &clipping_group(doc, path) {
        if let Some(target) = pictura_render::resolve_path_mut(&mut base_doc, target_path) {
            target.visible = false;
        }
    }
    let rendered = current_buffer(&base_doc, gpu_compute);
    Some((
        buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &rendered)),
        image,
        x,
        y,
        opacity,
    ))
}

/// The moving pixels of [`build_transform_preview`] and their document-space
/// origin: the layer (or its clipping group, or its rendered appearance).
pub(super) fn transform_preview_layer(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, i32, i32)> {
    let layer = pictura_render::resolve_path(doc, path)?;
    let group = clipping_group(doc, path);
    let image = if group.len() > 1 {
        rendered_clipping_group(doc, &group, gpu_compute)?
    } else if needs_rendered_appearance(layer) {
        rendered_layer_image(doc, path, gpu_compute)?
    } else {
        match layer_image(layer) {
            Some(image) => (image, layer.rect.left, layer.rect.top),
            None => {
                let mut clone = doc.clone();
                if !pictura_render::rasterize_smart_object(&mut clone, path) {
                    return None;
                }
                let rendered = pictura_render::resolve_path(&clone, path)?;
                (
                    layer_image(rendered)?,
                    rendered.rect.left,
                    rendered.rect.top,
                )
            }
        }
    };
    Some(image)
}

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
    let mut clone = doc.clone();
    pictura_render::rasterize_smart_object(&mut clone, path)
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
        .map(|s| s.data.clone())
}

/// Whether a selection on `path` lifts its pixels for the transform (CS6
/// transforms the selected pixels, a Background's included; a hidden layer's
/// does not).
fn lifts_selection(rust: &PictureViewRust, path: &str) -> bool {
    rust.session.doc.as_ref().is_some_and(|doc| {
        pictura_render::resolve_path(doc, path)
            .is_some_and(|layer| layer.visible && pictura_render::can_lift_selection(layer))
    }) && lift_coverage(rust).is_some()
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
        if !self.as_mut().compute_transform_preview(&session.path) {
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
    pub(super) fn compute_transform_preview(mut self: Pin<&mut Self>, path: &str) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let prepared = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| build_transform_preview(doc, path, gpu_compute));
        let Some((base, image, x, y, opacity)) = prepared else {
            return false;
        };
        rust.move_base = Some(base);
        rust.move_layer = Some(image);
        rust.move_x = x;
        rust.move_y = y;
        rust.move_opacity = opacity;
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
        if identity {
            self.as_mut().drop_transform_session();
            self.as_mut().clear_transform_preview();
            return false;
        }
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
        let lifted = self
            .as_mut()
            .rust_mut()
            .session
            .live
            .take_transform()
            .and_then(|session| session.lifted);
        if let Some((_, before)) = lifted {
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
                .edit(&history_label, |_| Edit::Structural { damage: None });
            self.as_mut().clear_transform_preview();
            return true;
        }
        if changed {
            self.as_mut().rust_mut().last_transform = repeat;
            self.as_mut().clear_link_sets();
            // The transform only moves the layer, so the union of its source rect
            // and its transformed rect bounds the composite change.
            let after = self
                .rust()
                .session
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
                .map(|layer| (layer.rect, layer_has_effects(layer)));
            match after {
                Some((after, false)) => self.as_mut().refresh_region(union_rect(orig_rect, after)),
                _ => self.as_mut().recomposite(),
            }
            self.as_mut().record(&history_label);
        }
        self.as_mut().clear_transform_preview();
        changed
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

impl qobject::PictureView {
    /// Cache the base composite (every move-group layer hidden), the moving
    /// image, its document-space origin, and opacity. One region composite at
    /// drag start, derived from the authoritative `doc.composite`.
    ///
    /// Reuses the cached base when the content revision and move-group key
    /// match; otherwise it recomputes.
    pub fn begin_move_preview(mut self: Pin<&mut Self>) -> bool {
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        if self.as_ref().move_cache_valid(&paths) {
            {
                let mut rust = self.as_mut().rust_mut();
                let (x, y, opacity) = rust
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| move_origin_opacity(doc, &paths))
                    .unwrap_or((0, 0, 0));
                rust.move_x = x;
                rust.move_y = y;
                rust.move_opacity = opacity;
                rust.move_preview_cache_hit = true;
            }
            return true;
        }
        let computed = self.as_mut().compute_move_preview(paths);
        self.as_mut().rust_mut().move_preview_cache_hit = false;
        computed
    }

    /// Begin a duplicate drag. With a selection active this is the Alt
    /// selection-content case: the selected pixels are prepared as the live
    /// move-preview layer, no document change. Otherwise the active pixel layer
    /// is cloned, made active, and previewed.
    pub fn begin_move_duplicate(mut self: Pin<&mut Self>) -> bool {
        if self.rust().session.selection.is_some() {
            return self.as_mut().begin_selection_duplicate_preview();
        }
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
        self.as_mut().compute_move_preview(vec![new_path])
    }

    /// Warm the move-preview cache without entering preview mode. A layer the
    /// Move tool may not move (a Background) is never previewed, so it is not
    /// warmed either: the warm composites the whole document.
    pub fn prepare_move_preview(mut self: Pin<&mut Self>) -> bool {
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        if self.as_ref().move_cache_valid(&paths) {
            return true;
        }
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
        !shape
            && rust.move_base.is_some()
            && rust.move_prepared_revision == rust.session.content_revision
            && rust.move_prepared_layer == move_group_key(paths)
    }

    /// Compute and store the move-preview base/layer for `paths` and record the
    /// revision and group key they were built from.
    fn compute_move_preview(mut self: Pin<&mut Self>, paths: Vec<String>) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        let moving = if let [path] = paths.as_slice() {
            let Some(layer) = pictura_render::resolve_path(doc, path) else {
                return false;
            };
            // A shape layer has no pixels of its own: its fill cut to the
            // outline is rendered.
            if pictura_render::is_shape_layer(layer) {
                rendered_layer_image(doc, path, gpu_compute)
            } else {
                layer_image(layer).map(|image| (image, layer.rect.left, layer.rect.top))
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
        let base = build_move_preview_base(doc, &paths, gpu_compute);
        rust.move_base = Some(base);
        rust.move_layer = Some(layer_image);
        rust.move_x = x;
        rust.move_y = y;
        rust.move_opacity = opacity;
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

    pub fn end_move_preview(self: Pin<&mut Self>) {
        // Keep the cache: the next press reuses it when the revision, topmost
        // layer, and clamped rect are unchanged.
    }

    /// Drop the transform overlay's prepared base/layer. The C++ canvas clears
    /// its copy separately; this keeps the Rust side from handing a stale
    /// preview to a later undo or move.
    pub fn clear_transform_preview(self: Pin<&mut Self>) {
        let mut rust = self.rust_mut();
        rust.move_base = None;
        rust.move_layer = None;
        rust.move_x = 0;
        rust.move_y = 0;
        rust.move_opacity = 0;
        rust.move_prepared_revision = 0;
        rust.move_prepared_layer.clear();
        rust.move_prepared_union = None;
        rust.move_preview_cache_hit = false;
    }

    pub fn commit_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        let dirty = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            translate_move_group(doc, &paths, dx, dy)
        };
        let Some(dirty) = dirty else {
            return false;
        };
        self.as_mut()
            .commit_move_edit("Move Layer", |_| Edit::regions(vec![dirty], false));
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
mod tests {
    use super::super::super::document_session::{TransformMode, TransformSession};
    use super::super::geometry::{pivot_point, rect_center, set_pivot, source_corners};
    use super::*;
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    use pictura_core::{BitDepth, Channel, ColorMode, Document, PsdRect};
    use pictura_render::Interpolation;

    fn transform_session(rect: PsdRect) -> TransformSession {
        TransformSession {
            path: "0".to_string(),
            orig_rect: rect,
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
            mode: TransformMode::Free,
            type_layer: false,
            quad: None,
            base_quad: None,
            free_perspective: false,
            free_distort: false,
            free_skew: false,
            perspective_side: None,
            start_quad: source_corners(rect),
            pivot: rect_center(rect),
            pivot_origin: rect_center(rect),
            start_pivot: rect_center(rect),
            show_pivot: true,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            warp_editing: false,
            warp_post: None,
            lifted: None,
            untrimmed: None,
            history_label: "Free Transform".to_string(),
        }
    }

    #[test]
    fn repeatable_transform_records_the_source_pivot() {
        let rect = PsdRect {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
        };
        let mut session = transform_session(rect);
        // A translated session takes a document pivot at the origin; the
        // stored source point is its inverse, and the recorded offset is in
        // the source frame, not the translated document frame.
        session.dx = 5.0;
        session.dy = 5.0;
        assert!(set_pivot(&mut session, 0.0, 0.0));
        assert_eq!(session.pivot, (-5.0, -5.0));
        assert_eq!(pivot_point(&session), (0.0, 0.0));
        let repeat = repeatable_transform(&session).expect("repeatable");
        assert_eq!(repeat.pivot_offset, (-10.0, -10.0));
    }

    #[test]
    fn a_shape_preview_ignores_empty_channels() {
        let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
        let outline = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (4.0, 4.0, 24.0, 20.0),
        )
        .expect("outline");
        let path =
            pictura_render::add_shape_layer(&mut doc, "", [0, 0, 0, 255], "Shape", &outline, None);
        assert!(!path.is_empty());
        // A PSD shape layer may carry empty pixel channels: its visible fill
        // comes from the descriptor and vector mask, not the channels.
        let plane = (doc.width * doc.height) as usize;
        let layer = pictura_render::resolve_path_mut(&mut doc, &path).expect("shape");
        layer.channels = [0, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: vec![0; plane].into(),
            })
            .collect();

        let (_, preview, x, y, _) =
            build_transform_preview(&doc, &path, false).expect("shape preview");
        assert_eq!((x, y), (0, 0));
        let filled = (0..preview.height()).any(|py| {
            (0..preview.width()).any(|px| {
                let c = preview.pixel_color(px, py);
                c.alpha() > 200 && c.red() < 20 && c.green() < 20 && c.blue() < 20
            })
        });
        assert!(filled, "the shape's fill is in the preview layer");
    }

    #[test]
    fn a_shape_layer_is_a_transform_target_unless_position_locked() {
        let mut doc = Document::new(40, 30, ColorMode::Rgb, BitDepth::Eight);
        let outline = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (4.0, 4.0, 24.0, 20.0),
        )
        .expect("outline");
        let path =
            pictura_render::add_shape_layer(&mut doc, "", [0, 0, 0, 255], "Shape", &outline, None);
        assert!(can_free_transform(&doc, &path));
        pictura_render::resolve_path_mut(&mut doc, &path)
            .expect("shape")
            .lock = LockFlags::default().with(LockFlags::POSITION, true);
        assert!(!can_free_transform(&doc, &path));
    }
}
