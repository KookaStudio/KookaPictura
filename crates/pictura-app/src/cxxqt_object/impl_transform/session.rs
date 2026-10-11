use super::super::helpers::*;
use super::super::helpers_composite::*;
use super::super::qobject;
use super::super::state::{TransformMode, TransformSession};
use super::geometry::{projective_coefficients, session_quad, source_corners};
use super::{build_move_preview_base, duplicate_move_target};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use pictura_core::{layer_move_locked, LockFlags};
use pictura_render::LayerTransform;

/// Build the transform overlay images for `path`: the document composited with
/// the target hidden, the target's own RGBA image (materializing a channel-less
/// embedded object into a clone), its document-space origin, and opacity.
fn build_transform_preview(
    doc: &pictura_core::Document,
    path: &str,
    gpu_compute: bool,
) -> Option<(QImage, QImage, i32, i32, i32)> {
    let layer = pictura_render::resolve_path(doc, path)?;
    let (x, y, opacity) = (layer.rect.left, layer.rect.top, layer.opacity as i32);
    let image = match layer_image(layer) {
        Some(image) => image,
        None => {
            let mut clone = doc.clone();
            if !pictura_render::rasterize_smart_object(&mut clone, path) {
                return None;
            }
            layer_image(pictura_render::resolve_path(&clone, path)?)?
        }
    };
    let mut base_doc = doc.clone();
    if let Some(target) = pictura_render::resolve_path_mut(&mut base_doc, path) {
        target.visible = false;
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
fn lifts_selection(rust: &super::super::state::PictureViewRust, path: &str) -> bool {
    rust.doc.as_ref().is_some_and(|doc| {
        pictura_render::resolve_path(doc, path).is_some_and(pictura_render::can_lift_selection)
    }) && lift_coverage(rust).is_some()
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

    /// The active session's mode name (`"free"`, `"skew"`, `"distort"`,
    /// `"perspective"`), or an empty string without a session. Test hook.
    pub fn transform_session_mode(&self) -> QString {
        self.rust()
            .transform_session
            .as_ref()
            .map(|session| QString::from(session.mode.name()))
            .unwrap_or_default()
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
        if !lift
            && !self
                .rust()
                .doc
                .as_ref()
                .is_some_and(|doc| can_free_transform(doc, &path))
        {
            return false;
        }
        self.as_mut().drop_transform_session();
        let mut lifted = None;
        if !lift {
            // The box hugs the layer's visible pixels, not its (often
            // canvas-sized) rect; the composite is unchanged.
            if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
                pictura_render::trim_to_content(doc, &path);
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
        let corners = source_corners(orig_rect);
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
            quad: mode.is_projective().then_some(corners),
            start_quad: corners,
            lifted,
        });
        true
    }

    /// Compute and store the transform overlay base/layer for `path`; keyed with
    /// the `-1` sentinel so the Move cache rebuilds when it next runs.
    fn compute_transform_preview(mut self: Pin<&mut Self>, path: &str) -> bool {
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

    /// Drop any session, restoring the document from before a lift.
    fn drop_transform_session(mut self: Pin<&mut Self>) {
        let session = self.as_mut().rust_mut().transform_session.take();
        if let Some((_, before)) = session.and_then(|session| session.lifted) {
            self.as_mut().rust_mut().doc = Some(*before);
            self.as_mut().recomposite();
        }
    }

    /// Commit the session: one `transform_layer` call in `Free` or
    /// `transform_layer_quad` in a projective mode, a recomposite, and one
    /// `"Free Transform"` history state on success. An identity transform records
    /// nothing; an engine refusal records nothing. The session always clears.
    pub fn commit_transform(mut self: Pin<&mut Self>) -> bool {
        let Some((path, mode, quad, transform, source, orig_rect)) =
            self.rust().transform_session.as_ref().map(|session| {
                (
                    session.path.clone(),
                    session.mode,
                    session_quad(session),
                    LayerTransform {
                        scale_x: session.scale_x,
                        scale_y: session.scale_y,
                        angle_radians: session.angle,
                        dx: session.dx,
                        dy: session.dy,
                    },
                    source_corners(session.orig_rect),
                    session.orig_rect,
                )
            })
        else {
            return false;
        };
        if mode == TransformMode::Free {
            let identity = transform.scale_x == 1.0
                && transform.scale_y == 1.0
                && transform.angle_radians == 0.0
                && transform.dx == 0.0
                && transform.dy == 0.0;
            if identity {
                self.as_mut().drop_transform_session();
                return false;
            }
        } else {
            let identity = quad
                .iter()
                .zip(source.iter())
                .all(|(q, s)| (q.0 - s.0).abs() <= 1e-9 && (q.1 - s.1).abs() <= 1e-9);
            if identity {
                self.as_mut().drop_transform_session();
                return false;
            }
        }
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => {
                if mode == TransformMode::Free {
                    pictura_render::transform_layer(doc, &path, transform)
                } else {
                    pictura_render::transform_layer_quad(doc, &path, quad)
                }
            }
            None => false,
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

    /// The nine coefficients of the projective map sending the source-rect
    /// corners to the live quad, in `QTransform` constructor order, or an empty
    /// string without a projective session.
    pub fn transform_preview_matrix(&self) -> QString {
        let Some(session) = self.rust().transform_session.as_ref() else {
            return QString::default();
        };
        if !session.mode.is_projective() {
            return QString::default();
        }
        let Some(coeffs) =
            projective_coefficients(source_corners(session.orig_rect), session_quad(session))
        else {
            return QString::default();
        };
        let encoded: Vec<String> = coeffs.iter().map(|c| format!("{c:.12}")).collect();
        QString::from(encoded.join(" "))
    }

    /// Whether a Free Transform session is active.
    pub fn transform_session_active(&self) -> bool {
        self.rust().transform_session.is_some()
    }

    /// The active session's target path, or an empty string.
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
