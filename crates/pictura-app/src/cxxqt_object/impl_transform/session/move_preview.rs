//! The Move tool's preview: the cached base composite, the moving image, the
//! view preview, and the Move commit. It shares the preview fields the transform
//! session draws with, through one teardown, and holds no lifetime rule of its
//! own.
//!
//! A pure move out of `session.rs`, which reached the file-size cap.

use super::super::super::helpers_composite::*;
use super::super::super::qobject;
use super::super::super::state::PictureViewRust;
use super::super::group::move_commit_cost;
use super::super::{
    build_move_preview_base, duplicate_move_target, link_members, movable_layer, move_group_key,
    move_group_union, move_origin_opacity, rendered_move_group, translate_move_group,
};
use super::preview::{backdrop_patch, single_layer_move_image};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::memory_gate;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::layer_move_locked;

impl qobject::PictureView {
    /// Cache the base composite (every move-group layer hidden), the moving
    /// image, its document-space origin, and opacity. One region composite at
    /// drag start, derived from the authoritative `doc.composite`.
    ///
    /// Reuses the cached base when the content revision and move-group key
    /// match; otherwise it recomputes.
    pub fn begin_move_preview(mut self: Pin<&mut Self>) -> bool {
        memory_gate::clear();
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        if let Some(path) = super::super::mask_move::target(self.rust()) {
            return self.begin_mask_move(path);
        }
        let Some(paths) = self.as_ref().move_group_paths() else {
            return false;
        };
        // A channel-less type layer has no alpha to measure its ink; render its
        // raster once, so this and later drags preview the words, not the whole
        // paragraph box.
        if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
            for path in &paths {
                pictura_render::materialize_type_layer(doc, path);
            }
        }
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
            let offset = if moved {
                self.rust().cached_move_offset(&paths)
            } else {
                (0, 0)
            };
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
                        // A commit trims a pixel layer to its content; a type
                        // layer keeps its box, so its ink offset is recomputed.
                        rust.move_layer_offset = offset;
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
            .snapshot()
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
    pub(in crate::cxxqt_object) fn move_group_paths(&self) -> Option<Vec<String>> {
        if let Some(path) = super::super::mask_move::target(self.rust()) {
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
            super::view_preview::above_patch(
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
            single_layer_move_image(doc, path, gpu_compute).map(|(image, x, y, crop_offset)| {
                offset = crop_offset;
                (image, x, y)
            })
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
        rust.move_view_pixels = Default::default();
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
        let Some((pixels, view)) =
            super::view_preview::follow(doc, path, view, dx, dy, rust.gpu_compute)
        else {
            return false;
        };
        rust.move_layer = Some(buffer_to_image(&pixels));
        rust.move_view_pixels.layer = Some(pixels);
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

    pub fn commit_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        memory_gate::clear();
        if !self.rust().quick_mask_content_allowed() {
            return false;
        }
        if dx == 0 && dy == 0 {
            return false;
        }
        if let Some(path) = super::super::mask_move::target(self.rust()) {
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
            if cost.is_some_and(|cost| !memory_gate::admit_commit(&cost).admitted()) {
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
        if self.rust().lazy_move_pending {
            let mut rust = self.as_mut().rust_mut();
            if rust.move_view.is_some() {
                rust.bake_move_stand_in(dx, dy);
            } else {
                rust.paint_reduced_stand_in(&[dirty]);
            }
        }
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
        let Some(preview) = super::view_preview::view_preview(
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
        let above = super::view_preview::above_patch(
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
        self.move_view_pixels = preview.pixels;
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
}
