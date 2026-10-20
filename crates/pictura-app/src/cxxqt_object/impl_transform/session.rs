use super::super::helpers::*;
use super::super::helpers_composite::*;
use super::super::qobject;
use super::super::state::{PictureViewRust, TransformMode, TransformSession};
use super::geometry::{
    apply_session_transform, rect_center, session_quad, source_corners, MapParams,
};
use super::{build_move_preview_base, duplicate_move_target};
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

/// Whether `path` resolves to a transformable target: a raster pixel layer, or
/// a channel-less embedded smart object whose source materializes. False for a
/// group, adjustment layer, Background layer, position-locked layer, or a
/// zero-area target.
fn can_free_transform(doc: &pictura_core::Document, path: &str) -> bool {
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group
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
    let doc = rust.doc.as_ref()?;
    rust.selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .filter(|s| pictura_render::coverage_bounds(&s.data, doc.width, doc.height).is_some())
        .map(|s| s.data.clone())
}

/// Whether a selection on `path` lifts its pixels for the transform (CS6
/// transforms the selected pixels, a Background's included).
fn lifts_selection(rust: &PictureViewRust, path: &str) -> bool {
    rust.doc.as_ref().is_some_and(|doc| {
        pictura_render::resolve_path(doc, path).is_some_and(pictura_render::can_lift_selection)
    }) && lift_coverage(rust).is_some()
}

/// Whether `mode` may begin on `path`: the Free Transform target rules, plus
/// Distort's refusal for editable type (a lifted selection targets pixels, not
/// the type layer).
pub(super) fn can_begin_mode(rust: &PictureViewRust, path: &str, mode: TransformMode) -> bool {
    let lift = lifts_selection(rust, path);
    let Some(doc) = rust.doc.as_ref() else {
        return false;
    };
    if !lift && !can_free_transform(doc, path) {
        return false;
    }
    !(mode == TransformMode::Distort
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
        let mut path = path.to_string();
        if path.is_empty() {
            let Some(active) = self.rust().active_layer.clone() else {
                return false;
            };
            path = active;
        }
        {
            let rust = self.rust();
            if let Some(session) = rust.transform_session.as_ref() {
                // A lifted session's target is its source; the floating layer's
                // path is an implementation detail that may name a sibling.
                let target = session.lifted.as_ref().map_or(&session.path, |(s, _)| s);
                if *target == path && session.mode == mode {
                    return true;
                }
            }
        }
        let lift = lifts_selection(self.rust(), &path);
        if !can_begin_mode(self.rust(), &path, mode) {
            return false;
        }
        self.as_mut().drop_transform_session();
        let mut lifted = None;
        let mut untrimmed = None;
        if !lift {
            // The box hugs the layer's visible pixels, not its (often
            // canvas-sized) rect; the composite is unchanged.
            if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
                let before = pictura_render::resolve_path(doc, &path).map(|layer| layer.rect);
                if pictura_render::trim_to_content(doc, &path) {
                    untrimmed = before;
                }
            }
        }
        if lift {
            let coverage = lift_coverage(self.rust()).unwrap_or_default();
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            let before = Box::new(doc.clone());
            let floating = pictura_render::lift_selection(doc, &path, &coverage);
            if floating.is_empty() {
                *doc = *before;
                return false;
            }
            lifted = Some((std::mem::replace(&mut path, floating), before));
        }
        if !self.as_mut().compute_transform_preview(&path) {
            if let Some((_, before)) = lifted {
                self.as_mut().rust_mut().doc = Some(*before);
            }
            return false;
        }
        let rect = {
            let rust = self.rust();
            rust.doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
                .map(|layer| layer.rect)
        };
        let Some(orig_rect) = rect else {
            return false;
        };
        let type_layer = self
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, &path))
            .is_some_and(|layer| layer.is_type());
        let corners = source_corners(orig_rect);
        let center = rect_center(orig_rect);
        self.as_mut().rust_mut().transform_session = Some(TransformSession {
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
            lifted,
            untrimmed,
        });
        true
    }

    /// Compute and store the transform overlay base/layer for `path`; keyed with
    /// the `-1` sentinel so the Move cache rebuilds when it next runs.
    pub(super) fn compute_transform_preview(mut self: Pin<&mut Self>, path: &str) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let prepared = rust
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
        rust.move_prepared_revision = rust.content_revision;
        rust.move_prepared_layer = String::new();
        true
    }

    /// Clear the session; a lifted selection's pixels go back where they were.
    pub fn cancel_transform(self: Pin<&mut Self>) {
        self.drop_transform_session();
    }

    /// Drop any session, restoring the document from before a lift, or the
    /// layer's untrimmed extent (the Move tool's controls box opens on every
    /// selected layer, and a trimmed layer would clip later filters).
    fn drop_transform_session(mut self: Pin<&mut Self>) {
        let session = self.as_mut().rust_mut().transform_session.take();
        if let Some((path, rect)) = session
            .as_ref()
            .and_then(|s| s.untrimmed.map(|rect| (s.path.clone(), rect)))
        {
            if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
                pictura_render::untrim_to(doc, &path, rect);
            }
        }
        if let Some((_, before)) = session.and_then(|session| session.lifted) {
            self.as_mut().rust_mut().doc = Some(*before);
            self.as_mut().recomposite();
        }
    }

    /// Commit the session: one transform op (the similarity, an affine quad
    /// when a shear is set, or `transform_layer_quad` in a projective mode), or
    /// in Warp mode the warp alone (it already carries the box), a recomposite,
    /// and one `"Free Transform"` history state on success. An identity
    /// transform or warp records nothing; an engine refusal records nothing.
    /// The session always clears.
    pub fn commit_transform(mut self: Pin<&mut Self>) -> bool {
        let Some((path, mode, quad, params, orig_rect, interpolation, warp)) =
            self.rust().transform_session.as_ref().map(|session| {
                (
                    session.path.clone(),
                    session.mode,
                    session_quad(session),
                    MapParams::of(session),
                    session.orig_rect,
                    session.interpolation,
                    session.warp.clone(),
                )
            })
        else {
            return false;
        };
        let identity = match &warp {
            Some(warp) => warp.is_identity(),
            None if mode.is_projective() => {
                let source = source_corners(orig_rect);
                quad.iter()
                    .zip(source.iter())
                    .all(|(q, s)| (q.0 - s.0).abs() <= 1e-9 && (q.1 - s.1).abs() <= 1e-9)
            }
            None => params.is_identity(),
        };
        if identity {
            self.as_mut().drop_transform_session();
            return false;
        }
        let changed = match (self.as_mut().rust_mut().doc.as_mut(), &warp) {
            (Some(doc), Some(warp)) => {
                pictura_render::transform_layer_warp(doc, &path, warp, interpolation)
            }
            (Some(doc), None) => {
                apply_session_transform(doc, &path, mode, quad, params, orig_rect, interpolation)
            }
            (None, _) => false,
        };
        let lifted = self
            .as_mut()
            .rust_mut()
            .transform_session
            .take()
            .and_then(|session| session.lifted);
        if let Some((_, before)) = lifted {
            let merged = changed
                && self
                    .as_mut()
                    .rust_mut()
                    .doc
                    .as_mut()
                    .is_some_and(|doc| pictura_render::merge_lifted(doc, &path));
            if !merged {
                self.as_mut().rust_mut().doc = Some(*before);
                self.as_mut().recomposite();
                return false;
            }
            // ponytail: CS6 carries the selection border through the transform;
            // it is dropped here rather than left outlining the old pixels.
            self.as_mut().rust_mut().selection = None;
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Free Transform");
            return true;
        }
        if changed {
            self.as_mut().clear_link_sets();
            // The transform only moves the layer, so the union of its source rect
            // and its transformed rect bounds the composite change.
            let after = self
                .rust()
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
                .map(|layer| (layer.rect, layer_has_effects(layer)));
            match after {
                Some((after, false)) => self.as_mut().refresh_region(union_rect(orig_rect, after)),
                _ => self.as_mut().recomposite(),
            }
            self.as_mut().record("Free Transform");
        }
        changed
    }

    /// Whether a Free Transform session is active.
    pub fn transform_session_active(&self) -> bool {
        self.rust().transform_session.is_some()
    }
    pub fn transform_session_path(&self) -> QString {
        self.rust()
            .transform_session
            .as_ref()
            .map(|session| QString::from(session.path.as_str()))
            .unwrap_or_default()
    }

    pub fn transform_scale_x(&self) -> f64 {
        self.rust()
            .transform_session
            .as_ref()
            .map_or(1.0, |session| session.scale_x)
    }

    pub fn transform_scale_y(&self) -> f64 {
        self.rust()
            .transform_session
            .as_ref()
            .map_or(1.0, |session| session.scale_y)
    }

    pub fn transform_angle(&self) -> f64 {
        self.rust()
            .transform_session
            .as_ref()
            .map_or(0.0, |session| session.angle)
    }

    pub fn transform_dx(&self) -> f64 {
        self.rust()
            .transform_session
            .as_ref()
            .map_or(0.0, |session| session.dx)
    }

    pub fn transform_dy(&self) -> f64 {
        self.rust()
            .transform_session
            .as_ref()
            .map_or(0.0, |session| session.dy)
    }
}

impl qobject::PictureView {
    /// Cache the base composite (the active raster layer hidden), the layer
    /// image, its document-space origin, and opacity. One region composite at
    /// drag start, derived from the authoritative `doc.composite`.
    ///
    /// Reuses the cached base when the content revision, target path, and
    /// clamped rect all match; otherwise it recomputes.
    pub fn begin_move_preview(mut self: Pin<&mut Self>) -> bool {
        let Some(path) = self.as_ref().move_cache_target() else {
            return false;
        };
        {
            let rust = self.rust();
            let Some(layer) = rust
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
            else {
                return false;
            };
            if layer_move_locked(layer) {
                return false;
            }
        }
        if self.as_ref().move_cache_valid(&path) {
            {
                let mut rust = self.as_mut().rust_mut();
                let (x, y, opacity) = rust
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &path))
                    .map_or((0, 0, 0), |layer| {
                        (layer.rect.left, layer.rect.top, layer.opacity as i32)
                    });
                rust.move_x = x;
                rust.move_y = y;
                rust.move_opacity = opacity;
                rust.move_preview_cache_hit = true;
            }
            return true;
        }
        let computed = self.as_mut().compute_move_preview(path);
        self.as_mut().rust_mut().move_preview_cache_hit = false;
        computed
    }

    /// Begin a duplicate drag. With a selection active this is the Alt
    /// selection-content case: the selected pixels are prepared as the live
    /// move-preview layer, no document change. Otherwise the active pixel layer
    /// is cloned, made active, and previewed.
    pub fn begin_move_duplicate(mut self: Pin<&mut Self>) -> bool {
        if self.rust().selection.is_some() {
            return self.as_mut().begin_selection_duplicate_preview();
        }
        let new_path = {
            let mut guard = self.as_mut().rust_mut();
            let rust = &mut *guard;
            let (doc, active_layer) = (&mut rust.doc, &mut rust.active_layer);
            let doc = doc.as_mut();
            let duplicate = doc.and_then(|d| duplicate_move_target(d, active_layer));
            let Some(new_path) = duplicate else {
                return false;
            };
            new_path
        };
        self.as_mut().recomposite();
        self.as_mut().compute_move_preview(new_path)
    }

    /// Warm the move-preview cache without entering preview mode. A layer the
    /// Move tool may not move (a Background) is never previewed, so it is not
    /// warmed either: the warm composites the whole document.
    pub fn prepare_move_preview(mut self: Pin<&mut Self>) -> bool {
        let Some(path) = self.as_ref().move_cache_target() else {
            return false;
        };
        {
            let rust = self.rust();
            let Some(layer) = rust
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
            else {
                return false;
            };
            if layer_move_locked(layer) {
                return false;
            }
        }
        if self.as_ref().move_cache_valid(&path) {
            return true;
        }
        self.as_mut().compute_move_preview(path)
    }

    pub fn move_preview_cache_hit(&self) -> bool {
        self.rust().move_preview_cache_hit
    }

    /// The active pixel layer's path, or `None` without a document or a single
    /// active raster layer.
    pub(super) fn move_cache_target(&self) -> Option<String> {
        let rust = self.rust();
        let doc = rust.doc.as_ref()?;
        let path = rust.active_layer.as_deref()?;
        active_pixel_layer(doc, Some(path))?;
        Some(path.to_string())
    }

    fn move_cache_valid(&self, path: &str) -> bool {
        let rust = self.rust();
        // The base is the document with the target layer hidden, so it does not
        // depend on that layer's position: a committed move leaves it valid.
        // `record_move` deliberately does not bump `content_revision` for this
        // reason; keying on the layer rect here would throw the base away on
        // every drag and force a full region recomposite on the next press.
        rust.move_base.is_some()
            && rust.move_prepared_revision == rust.content_revision
            && rust.move_prepared_layer == path
    }

    /// Compute and store the move-preview base/layer for the layer at `path`
    /// and record the revision and path they were built from.
    fn compute_move_preview(mut self: Pin<&mut Self>, path: String) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        let gpu_compute = rust.gpu_compute;
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        let Some(layer) = pictura_render::resolve_path(doc, &path) else {
            return false;
        };
        let Some(layer_image) = layer_image(layer) else {
            return false;
        };
        let rect = layer.rect;
        let (x, y, opacity) = (rect.left, rect.top, layer.opacity as i32);
        let base = build_move_preview_base(doc, &path, gpu_compute);
        rust.move_base = Some(base);
        rust.move_layer = Some(layer_image);
        rust.move_x = x;
        rust.move_y = y;
        rust.move_opacity = opacity;
        rust.move_prepared_revision = rust.content_revision;
        rust.move_prepared_layer = path;
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

    pub fn commit_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }
        let Some(path) = self.as_ref().move_cache_target() else {
            return false;
        };
        let before = {
            let rust = self.rust();
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            match pictura_render::resolve_path(doc, &path) {
                Some(layer) => layer.rect,
                None => return false,
            }
        };
        let moved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::translate_layer_path(doc, &path, dx, dy)
        };
        if !moved {
            return false;
        }
        let dirty = {
            let rust = self.rust();
            let after = rust
                .doc
                .as_ref()
                .and_then(|doc| pictura_render::resolve_path(doc, &path))
                .map_or(before, |layer| layer.rect);
            union_rect(before, after)
        };
        self.as_mut().refresh_region(dirty);
        self.as_mut().record_move("Move Layer");
        true
    }

    pub fn commit_move_legacy(mut self: Pin<&mut Self>) -> bool {
        if self.rust().doc.is_none() {
            return false;
        }
        self.as_mut().recomposite();
        self.as_mut().record("Move Layer");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    use pictura_core::{BitDepth, Channel, ColorMode, Document};

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
}
