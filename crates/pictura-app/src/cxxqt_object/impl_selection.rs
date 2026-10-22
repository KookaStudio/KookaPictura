use super::edit::Edit;
use super::helpers::*;
use super::helpers_composite::*;
use super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_codec::buffer_to_srgb;
use pictura_core::{Channel, ColorMode, Document, PsdRect};
use pictura_select::{CombineMode, Selection};

mod bridge;
mod refine;

impl qobject::PictureView {
    pub fn select_all(mut self: Pin<&mut Self>) {
        let dims = self
            .rust()
            .session
            .doc
            .as_ref()
            .map(|d| (d.width, d.height));
        if let Some((w, h)) = dims {
            {
                let mut rust = self.as_mut().rust_mut();
                if let Some(prev) = rust.session.selection.take() {
                    if prev.data.iter().any(|&v| v > 0) {
                        rust.session.deselected_selection = Some(prev);
                    }
                }
                rust.session.selection = Some(Selection::all(w, h));
            }
            self.as_mut().edit("Select All", |_| Edit::Meta);
        }
    }

    pub fn deselect(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            if let Some(prev) = rust.session.selection.take() {
                rust.session.deselected_selection = Some(prev);
            }
        }
        self.as_mut().edit("Deselect", |_| Edit::Meta);
    }

    /// Restore the last deselected selection. Refuses (no history) without a
    /// stored selection or when its dimensions no longer match the document.
    pub fn reselect(mut self: Pin<&mut Self>) -> bool {
        let restored = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            match rust.session.deselected_selection.as_ref() {
                Some(sel) if sel.width == doc.width && sel.height == doc.height => sel.clone(),
                _ => return false,
            }
        };
        self.as_mut().rust_mut().session.selection = Some(restored);
        self.as_mut().edit("Reselect", |_| Edit::Meta);
        true
    }

    pub fn has_deselected_selection(&self) -> bool {
        self.rust().session.deselected_selection.is_some()
    }

    pub fn invert_selection(mut self: Pin<&mut Self>) -> bool {
        let inverted = self
            .rust()
            .session
            .selection
            .as_ref()
            .map(Selection::invert);
        let Some(inverted) = inverted else {
            return false;
        };
        self.as_mut().rust_mut().session.selection = Some(inverted);
        self.as_mut().edit("Inverse Selection", |_| Edit::Meta);
        true
    }

    /// Capture the current selection as the origin of a move-selection drag.
    /// Refuses (false) without a selection.
    pub fn begin_selection_move(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().rust_mut().begin_selection_move()
    }

    /// Record the drag offset. No history and no `changed` emission: the tool
    /// drives the overlay. False without a drag.
    pub fn preview_selection_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        self.as_mut().rust_mut().preview_selection_move(dx, dy)
    }

    /// Commit the move: one "Move Selection" state when the selection actually
    /// moved, nothing when it lands on the origin. Clears the drag.
    pub fn commit_selection_move(mut self: Pin<&mut Self>) -> bool {
        if !self.as_mut().rust_mut().commit_selection_move() {
            return false;
        }
        self.as_mut().edit("Move Selection", |_| Edit::Meta);
        true
    }

    /// Restore the origin selection and drop the drag. No history.
    pub fn cancel_selection_move(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().rust_mut().cancel_selection_move()
    }

    /// Move (or, with `duplicate`, copy) the content covered by the selection
    /// origin, offset by `(dx, dy)`. A mask edit target moves mask coverage;
    /// otherwise the active layer's pixels move (with a new layer on duplicate).
    /// One "Move Selection" state; false (no state) on a zero offset, missing
    /// doc/selection, or engine refusal.
    pub fn move_selection_content(
        mut self: Pin<&mut Self>,
        dx: i32,
        dy: i32,
        duplicate: bool,
        background: u32,
    ) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }
        let editable = {
            let rust = self.rust();
            rust.session
                .doc
                .as_ref()
                .is_some_and(|doc| active_layer_visible(doc, rust.layer_target.single()))
        };
        if !editable {
            return false;
        }
        if let Some(path) = active_mask_target(self.rust()) {
            let prepared = {
                let rust = self.rust();
                let Some(doc) = rust.session.doc.as_ref() else {
                    return false;
                };
                let Some(mask) =
                    pictura_render::resolve_path(doc, &path).and_then(|layer| layer.mask.as_ref())
                else {
                    return false;
                };
                let Some(selection) = rust
                    .session
                    .live
                    .selection_move_origin()
                    .or(rust.session.selection.as_ref())
                else {
                    return false;
                };
                (
                    crop_selection(
                        &selection.data,
                        selection.width,
                        selection.height,
                        mask.rect,
                    ),
                    selection.translate(dx, dy),
                    luma_u8(rgba_from_argb(background)),
                    move_dirty_rect(selection, dx, dy),
                )
            };
            let (coverage, moved_selection, background, dirty) = prepared;
            let moved = {
                let mut rust = self.as_mut().rust_mut();
                let Some(doc) = rust.session.doc.as_mut() else {
                    return false;
                };
                pictura_render::move_mask_selection_content(
                    doc, &path, &coverage, dx, dy, duplicate, background,
                )
            };
            if !moved {
                return false;
            }
            {
                let mut rust = self.as_mut().rust_mut();
                rust.session.live.take_selection_move();
                rust.session.selection = Some(moved_selection);
            }
            let outcome = match dirty {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            };
            self.as_mut().edit("Move Selection", |_| outcome);
            return true;
        }
        let active = self.rust().layer_target.single().map(str::to_string);
        let prepared = {
            let rust = self.rust();
            let (Some(doc), Some(selection)) = (
                rust.session.doc.as_ref(),
                rust.session
                    .live
                    .selection_move_origin()
                    .cloned()
                    .or_else(|| rust.session.selection.clone()),
            ) else {
                return false;
            };
            let Some(path) = active.as_deref() else {
                return false;
            };
            if active_pixel_layer(doc, Some(path)).is_none() {
                return false;
            }
            (
                path.to_string(),
                selection_to_mask(&selection, doc),
                selection.translate(dx, dy),
                move_dirty_rect(&selection, dx, dy),
            )
        };
        let (path, mask, moved_selection, dirty) = prepared;
        let moved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            pictura_render::move_selection_content(doc, &path, &mask, dx, dy, duplicate)
        };
        if !moved {
            return false;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session.live.take_selection_move();
            rust.session.selection = Some(moved_selection);
            if duplicate {
                // `layer_via_copy` inserts the copy directly above the source
                // in the source's own container.
                if let Some(copy) = layer_above(&path) {
                    rust.select_only_layer(&copy);
                }
            }
        }
        let outcome = match dirty {
            Some(rect) => Edit::Regions {
                rects: vec![rect],
                notify: false,
            },
            None => Edit::Full,
        };
        self.as_mut().edit("Move Selection", |_| outcome);
        true
    }

    /// Build the live Alt selection-content preview without touching the
    /// document: the full composite as base and the masked copy of the selected
    /// pixels as the moving layer, stored in the move-preview slots. The copy is
    /// produced by the same engine call the commit uses, so preview and commit
    /// agree. No history. False without a document, selection, editable active
    /// layer, or when the engine refuses (a locked source).
    pub fn begin_selection_duplicate_preview(mut self: Pin<&mut Self>) -> bool {
        if active_mask_target(self.rust()).is_some() {
            return false;
        }
        let prepared = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let selection = rust
                .session
                .live
                .selection_move_origin()
                .or(rust.session.selection.as_ref());
            let (Some(selection), Some(path)) = (selection, rust.layer_target.single()) else {
                return false;
            };
            if !active_layer_visible(doc, Some(path)) {
                return false;
            }
            selection_duplicate_preview(doc, path, selection, rust.gpu_compute)
        };
        let Some((base, layer, opacity)) = prepared else {
            return false;
        };
        let mut rust = self.as_mut().rust_mut();
        rust.move_base = Some(base);
        rust.move_layer = Some(layer);
        rust.move_x = 0;
        rust.move_y = 0;
        rust.move_opacity = opacity;
        rust.move_prepared_revision = rust.session.content_revision;
        // The empty sentinel keeps the whole-layer cache from reusing this base.
        rust.move_prepared_layer = String::new();
        rust.move_preview_cache_hit = false;
        true
    }

    /// `border` width 1-200, `smooth`/`expand`/`contract` radius 1-100,
    /// `feather` radius >0 (clamped to 250). Zero/unknown amounts refuse
    /// (no history).
    pub fn modify_selection(mut self: Pin<&mut Self>, op: &QString, amount: f64) -> bool {
        if !amount.is_finite() {
            return false;
        }
        let op = op.to_string();
        let label = match op.as_str() {
            "border" => "Border Selection",
            "smooth" => "Smooth Selection",
            "expand" => "Expand Selection",
            "contract" => "Contract Selection",
            "feather" => "Feather Selection",
            _ => return false,
        };
        let modified = {
            let rust = self.rust();
            let Some(sel) = rust.session.selection.as_ref() else {
                return false;
            };
            let radius = amount.round();
            match op.as_str() {
                "border" => (radius >= 1.0).then(|| sel.border(radius as u32)),
                "smooth" => (radius >= 1.0).then(|| sel.smooth(radius as u32)),
                "expand" => (radius >= 1.0).then(|| sel.expand(radius as u32)),
                "contract" => (radius >= 1.0).then(|| sel.contract(radius as u32)),
                "feather" => (amount > 0.0).then(|| sel.feather(amount)),
                _ => None,
            }
        };
        let Some(modified) = modified else {
            return false;
        };
        self.as_mut().rust_mut().session.selection = Some(modified);
        self.as_mut().edit(label, |_| Edit::Meta);
        true
    }

    pub fn grow_selection(mut self: Pin<&mut Self>, tolerance: i32) -> bool {
        let grown = {
            let rust = self.rust();
            let (Some(doc), Some(sel)) =
                (rust.session.doc.as_ref(), rust.session.selection.as_ref())
            else {
                return false;
            };
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::grow(sel, &current_buffer(doc, rust.gpu_compute), tolerance) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        self.as_mut().rust_mut().session.selection = Some(grown);
        self.as_mut().edit("Grow Selection", |_| Edit::Meta);
        true
    }

    pub fn similar_selection(mut self: Pin<&mut Self>, tolerance: i32) -> bool {
        let matched = {
            let rust = self.rust();
            let (Some(doc), Some(sel)) =
                (rust.session.doc.as_ref(), rust.session.selection.as_ref())
            else {
                return false;
            };
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::similar(sel, &current_buffer(doc, rust.gpu_compute), tolerance) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        self.as_mut().rust_mut().session.selection = Some(matched);
        self.as_mut().edit("Similar Selection", |_| Edit::Meta);
        true
    }

    /// Append the coverage as a document extra channel. The dialog `name` is
    /// accepted but not stored: `Channel` has only `id`.
    // ponytail: names map positionally to `Alpha N` = channels[N-1]; add a
    // `Channel.name` field only if PSD round-tripping of names is required.
    pub fn save_selection(mut self: Pin<&mut Self>, _name: &QString) -> bool {
        let saved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(sel) = rust.session.selection.clone() else {
                return false;
            };
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            let id = doc.channels.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            doc.channels.push(sel.to_channel(id));
            true
        };
        if saved {
            self.as_mut().edit("Save Selection", |_| Edit::Record);
        }
        saved
    }

    pub fn load_selection(mut self: Pin<&mut Self>, name: &QString) -> bool {
        let loaded = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            selection_from_named_channel(&doc.channels, doc.width, doc.height, &name.to_string())
        };
        let Some(selection) = loaded else {
            return false;
        };
        self.as_mut().rust_mut().session.selection = Some(selection);
        self.as_mut().edit("Load Selection", |_| Edit::Meta);
        true
    }

    /// Load the Channels-panel colour row `row` (0 = the composite) as the
    /// selection, using the same grayscale derivation as the row thumbnails.
    /// `mask` is the visible RGB channels (bit 0 red, 1 green, 2 blue); a hidden
    /// channel is zeroed in the composite row, matching the canvas. One
    /// "Load Selection" state; false for a row outside the mode's colour rows.
    pub fn load_channel_selection(self: Pin<&mut Self>, row: i32, mask: i32) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            channel_row_selection(doc, rust.gpu_compute, row, mask as u8)
        };
        let Some(shape) = shape else {
            return false;
        };
        self.apply_selection_labeled(shape, CombineMode::New, "Load Selection")
    }

    pub fn selection_channel_count(&self) -> i32 {
        self.rust()
            .session
            .doc
            .as_ref()
            .map_or(0, |d| d.channels.len() as i32)
    }

    /// Every layer row path in panel order (depth-first, topmost-first).
    pub fn select_all_layers(&self) -> QStringList {
        self.rust()
            .session
            .doc
            .as_ref()
            .map(|doc| {
                pictura_render::flatten_rows(doc)
                    .into_iter()
                    .map(|(path, _)| QString::from(path))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn magic_wand(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        contiguous: bool,
        mode: &QString,
    ) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            if x < 0 || y < 0 {
                return false;
            }
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::magic_wand(
                &current_buffer(doc, rust.gpu_compute),
                x as u32,
                y as u32,
                tolerance,
                contiguous,
            ) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    /// Build a document-sized selection from the alpha channel of the layer at
    /// `path` (a missing alpha is fully opaque inside the layer's rect) and
    /// replace the current selection with it. Records one undo state; false for
    /// an unresolved path.
    pub fn select_layer_alpha(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let Some(layer) = pictura_render::resolve_path(doc, &path) else {
                return false;
            };
            pictura_select::selection_from_layer_alpha(layer, doc.width, doc.height)
        };
        self.as_mut().apply_selection(shape, CombineMode::New)
    }

    /// Load the raster mask on the layer at `path` as a selection, combined with
    /// `mode` (`add`/`subtract`/`intersect`, else `New`). White coverage is
    /// selected. One undo state; false for an unresolved path or a layer with no
    /// raster mask.
    pub fn mask_selection(mut self: Pin<&mut Self>, path: &QString, mode: &QString) -> bool {
        let path = path.to_string();
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let mask =
                pictura_render::resolve_path(doc, &path).and_then(|layer| layer.mask.as_ref());
            let Some(mask) = mask else {
                return false;
            };
            pictura_select::selection_from_mask(mask, doc.width, doc.height)
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut()
            .apply_selection_labeled(shape, mode, "Mask to Selection")
    }

    pub fn has_selection(&self) -> bool {
        self.rust().session.selection.is_some()
    }

    pub fn selection_count(&self) -> i32 {
        self.rust()
            .session
            .selection
            .as_ref()
            .map_or(0, |s| s.data.iter().filter(|&&v| v > 0).count() as i32)
    }

    /// Coverage byte at `(x, y)` (0 outside, 255 fully inside, intermediate on
    /// a feathered edge). 0 without a document/selection or out of bounds.
    pub fn selection_coverage(&self, x: i32, y: i32) -> i32 {
        i32::from(self.rust().selection_coverage_at(x, y))
    }

    /// Merge a document-sized `shape` into the current selection with `mode`,
    /// capturing history and emitting [`changed`]. Returns false without a doc.
    ///
    /// A New-mode commit that replaces a non-empty selection stores the old one
    /// as the `reselect` memory; every other commit clears it.
    fn apply_selection(self: Pin<&mut Self>, shape: Selection, mode: CombineMode) -> bool {
        self.apply_selection_labeled(shape, mode, "Selection")
    }

    /// [`apply_selection`](Self::apply_selection) under a history `label`.
    pub(super) fn apply_selection_labeled(
        mut self: Pin<&mut Self>,
        shape: Selection,
        mode: CombineMode,
        label: &str,
    ) -> bool {
        {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return false;
            }
            let previous = rust.session.selection.take();
            let replaced = matches!(mode, CombineMode::New)
                && previous
                    .as_ref()
                    .is_some_and(|s| s.data.iter().any(|&v| v > 0));
            if replaced {
                rust.session.deselected_selection = previous;
                rust.session.selection = Some(shape);
            } else {
                rust.session.deselected_selection = None;
                rust.session.selection = Some(match previous {
                    Some(mut base) => {
                        base.combine_with(&shape, mode);
                        base
                    }
                    // Combined into nothing, a new or added shape is itself.
                    None if matches!(mode, CombineMode::New | CombineMode::Add) => shape,
                    None => Selection::none(shape.width, shape.height),
                });
            }
        }
        self.as_mut().edit(label, |_| Edit::Meta);
        true
    }

    pub fn select_rect(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        mode: &QString,
        feather: f64,
    ) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            feathered(Selection::rect(doc.width, doc.height, x, y, w, h), feather)
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    pub fn select_ellipse(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        mode: &QString,
        feather: f64,
    ) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            feathered(
                Selection::ellipse(doc.width, doc.height, x, y, w, h),
                feather,
            )
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    pub fn begin_lasso(mut self: Pin<&mut Self>, mode: &QString) -> bool {
        self.as_mut()
            .rust_mut()
            .begin_lasso_session(&mode.to_string())
    }

    pub fn lasso_add_point(mut self: Pin<&mut Self>, x: i32, y: i32) {
        self.as_mut().rust_mut().add_lasso_point(x, y);
    }

    pub fn end_lasso(mut self: Pin<&mut Self>, feather: f64) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let points = rust.session.live.lasso_points().unwrap_or_default();
            if points.len() < 3 {
                return false;
            }
            feathered(Selection::polygon(doc.width, doc.height, points), feather)
        };
        let mode = combine_mode_from(self.rust().session.live.lasso_mode().unwrap_or_default());
        if !self.as_mut().apply_selection(shape, mode) {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        rust.session.live.take_selection_gesture();
        true
    }

    pub fn cancel_lasso(mut self: Pin<&mut Self>) {
        let mut rust = self.as_mut().rust_mut();
        rust.session.live.take_selection_gesture();
    }

    pub fn quick_select(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        mode: &QString,
    ) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            if x < 0 || y < 0 || x as u32 >= doc.width || y as u32 >= doc.height {
                return false;
            }
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::magic_wand(
                &current_buffer(doc, rust.gpu_compute),
                x as u32,
                y as u32,
                tolerance,
                true,
            ) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    pub fn crop(mut self: Pin<&mut Self>, x: i32, y: i32, w: i32, h: i32) -> bool {
        if w < 1 || h < 1 {
            return false;
        }
        let cropped = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            if !pictura_render::crop_document(doc, x, y, w as u32, h as u32) {
                return false;
            }
            // Canvas dimensions changed, so the old selection no longer maps.
            rust.session.selection = None;
            true
        };
        if cropped {
            self.as_mut().edit("Crop", |_| Edit::Full);
        }
        cropped
    }

    /// The selected pixels' bounding box as flat `[x, y, w, h]`; empty when
    /// nothing is selected. View-only: derives from the mask, records no history.
    pub fn selection_bounds_extents(&self) -> Vec<i32> {
        let Some(bounds) = self.rust().selection_bounds() else {
            return Vec::new();
        };
        vec![
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
        ]
    }

    /// The 50 %-coverage outline of the selection encoded as flat
    /// `[n0, x,y, x,y, …, n1, …]` (loop point counts then integer pixel
    /// corners); empty without a selection. View-only: derives from the mask,
    /// records no history.
    pub fn selection_contour_extents(&self) -> Vec<i32> {
        let rust = self.rust();
        let Some(sel) = rust.session.selection.as_ref() else {
            return Vec::new();
        };
        let loops = pictura_select::contour(sel, 128);
        let mut out = Vec::new();
        for poly in &loops {
            out.push(poly.len() as i32);
            for p in poly {
                out.push(p[0]);
                out.push(p[1]);
            }
        }
        out
    }
}

impl super::state::PictureViewRust {
    /// Start a lasso gesture in `mode`. False without a document or when
    /// another session owns the slot.
    pub(super) fn begin_lasso_session(&mut self, mode: &str) -> bool {
        if self.session.doc.is_none() {
            return false;
        }
        if !self.slot_allows(super::document_session::SessionKind::Lasso) {
            return false;
        }
        match self.session.live.selection_gesture_mut() {
            Some(gesture) => {
                gesture.mode = mode.to_string();
                gesture.points.clear();
            }
            None => {
                let gesture = super::document_session::SelectionGesture {
                    mode: mode.to_string(),
                    ..Default::default()
                };
                self.session.live.set_selection_gesture(gesture);
            }
        }
        true
    }

    /// Add a point to the live lasso.
    pub(super) fn add_lasso_point(&mut self, x: i32, y: i32) {
        if let Some(points) = self.session.live.lasso_points_mut() {
            points.push((x, y));
        }
    }
}

/// The `(base, moving layer, opacity)` for an Alt selection-content preview.
///
/// The base is the full composite (source pixels intact); the moving layer is a
/// document-sized copy of the selected pixels produced by the same engine call
/// The path directly above `path` in the same container, or `None` for a
/// malformed path. `layer_via_copy` inserts the copy at that index.
fn layer_above(path: &str) -> Option<String> {
    let (prefix, last) = path.rsplit_once('/').unwrap_or(("", path));
    let index: usize = last.parse().ok()?;
    Some(if prefix.is_empty() {
        (index + 1).to_string()
    } else {
        format!("{prefix}/{}", index + 1)
    })
}

/// The document rectangle a selection move can change: the union of the
/// selection's bounds and their translated destination, clipped to the canvas.
/// `None` when the selection is empty or the union misses the canvas.
pub(super) fn move_dirty_rect(selection: &Selection, dx: i32, dy: i32) -> Option<PsdRect> {
    let bounds = selection.bounds()?;
    let (w, h) = (selection.width as i32, selection.height as i32);
    let destination = PsdRect {
        top: bounds.top + dy,
        left: bounds.left + dx,
        bottom: bounds.bottom + dy,
        right: bounds.right + dx,
    };
    let rect = PsdRect {
        top: bounds.top.min(destination.top).clamp(0, h),
        left: bounds.left.min(destination.left).clamp(0, w),
        bottom: bounds.bottom.max(destination.bottom).clamp(0, h),
        right: bounds.right.max(destination.right).clamp(0, w),
    };
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}

/// the commit uses, so the preview matches the committed layer exactly. Returns
/// `None` for an unresolved path, a missing source layer, or an engine refusal.
///
/// ponytail: clones the document and recomposites once per Alt press, not per
/// pointer event; shave it only if press latency on large documents matters.
fn selection_duplicate_preview(
    doc: &pictura_core::Document,
    path: &str,
    selection: &Selection,
    gpu_compute: bool,
) -> Option<(QImage, QImage, i32)> {
    pictura_render::resolve_path(doc, path)?;
    let copy_path = layer_above(path)?;
    let mask = selection_to_mask(selection, doc);
    let mut preview = doc.clone();
    if !pictura_render::move_selection_content(&mut preview, path, &mask, 0, 0, true) {
        return None;
    }
    let copy = pictura_render::resolve_path(&preview, &copy_path)?;
    let layer = layer_image(copy)?;
    Some((
        document_to_image(doc, gpu_compute),
        layer,
        copy.opacity as i32,
    ))
}

/// The zero-based `doc.channels` index for a dialog label `"Alpha N"`.
///
/// Names are positional because [`Channel`] has no name field; anything that is
/// not exactly `"Alpha N"` (N >= 1) resolves to `None`.
fn parse_channel_name(name: &str) -> Option<usize> {
    let n: usize = name.strip_prefix("Alpha ")?.trim().parse().ok()?;
    n.checked_sub(1)
}

/// The selection stored in the named extra channel, or `None` for an unknown
/// name or a channel whose byte length is not `width * height`.
fn selection_from_named_channel(
    channels: &[Channel],
    width: u32,
    height: u32,
    name: &str,
) -> Option<Selection> {
    let channel = channels.get(parse_channel_name(name)?)?;
    Selection::from_channel(channel, width, height).ok()
}

/// The colour-channel row layout of a document `mode`: whether a composite row
/// leads the list, and the number of colour channels after it. Mirrors the
/// Channels panel's `channelsForMode`: a single-row mode's one row is the
/// composite, with no separate colour channel.
fn color_rows(mode: ColorMode) -> (bool, usize) {
    match mode {
        ColorMode::Rgb | ColorMode::Lab => (true, 3),
        ColorMode::Cmyk => (true, 4),
        ColorMode::Multichannel => (false, 3),
        ColorMode::Grayscale | ColorMode::Indexed | ColorMode::Bitmap | ColorMode::Duotone => {
            (true, 0)
        }
    }
}

/// Qt's `qGray`: `(r*11 + g*16 + b*5) >> 5`.
fn q_gray(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 11 + g as u32 * 16 + b as u32 * 5) >> 5) as u8
}

/// The naive CMYK ink level `0..=255` for one channel of an RGB pixel, matching
/// the panel's `toCmyk`.
fn cmyk_ink(r: u8, g: u8, b: u8, index: usize) -> u8 {
    let (rf, gf, bf) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let k = 1.0 - rf.max(gf).max(bf);
    let inv = if k < 1.0 { 1.0 / (1.0 - k) } else { 0.0 };
    let inks = [
        (1.0 - rf - k) * inv,
        (1.0 - gf - k) * inv,
        (1.0 - bf - k) * inv,
        k,
    ];
    (inks[index].clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The panel thumbnail's grayscale value for colour row `index`
/// (`channelValue`): RGB uses its own plane, CMYK and Multichannel their naive
/// ink coverage, everything else the composite luminance.
fn channel_value(mode: ColorMode, index: usize, r: u8, g: u8, b: u8) -> u8 {
    match mode {
        ColorMode::Rgb => [r, g, b][index.min(2)],
        ColorMode::Cmyk => 255 - cmyk_ink(r, g, b, index.min(3)),
        ColorMode::Multichannel => 255 - cmyk_ink(r, g, b, index.min(2)),
        _ => q_gray(r, g, b),
    }
}

/// The selection a Channels-panel colour `row` holds: the composite (row 0) is
/// its luminance over the visible `mask` channels, each colour row its own
/// grayscale plane (the same derivation the row thumbnails use). `None` for a
/// row outside the mode's colour rows or a buffer that does not match the
/// document.
// ponytail: the derived-mode planes (CMYK inks, Lab) are the naive
// conversions the thumbnails already use, not real separations.
fn channel_row_selection(
    doc: &Document,
    gpu_compute: bool,
    row: i32,
    mask: u8,
) -> Option<Selection> {
    let row = usize::try_from(row).ok()?;
    let mode = doc.source_mode.unwrap_or(doc.mode);
    let (composite, channels) = color_rows(mode);
    if row >= channels + usize::from(composite) {
        return None;
    }
    let buffer = current_buffer(doc, gpu_compute);
    let srgb = buffer_to_srgb(doc, &buffer);
    let buffer = srgb.as_ref();
    let plane = buffer.pixel_count();
    let depth = buffer.channels as usize;
    if plane == 0
        || depth == 0
        || buffer.width != doc.width
        || buffer.height != doc.height
        || buffer.data.len() != plane * depth
    {
        return None;
    }
    let mut data = vec![0u8; plane];
    for (i, out) in data.iter_mut().enumerate() {
        let r = buffer.data[i];
        let (g, b) = if depth >= 3 {
            (buffer.data[plane + i], buffer.data[2 * plane + i])
        } else {
            (r, r)
        };
        *out = if composite && row == 0 {
            // The composite row follows the visible channels.
            q_gray(
                if mask & 0x1 != 0 { r } else { 0 },
                if mask & 0x2 != 0 { g } else { 0 },
                if mask & 0x4 != 0 { b } else { 0 },
            )
        } else {
            // A colour row is its own plane regardless of the eyes: CS6 loads a
            // hidden channel too.
            channel_value(mode, if composite { row - 1 } else { row }, r, g, b)
        };
    }
    Some(Selection {
        width: doc.width,
        height: doc.height,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::BitDepth;

    fn channel(id: i16, len: usize) -> Channel {
        Channel {
            id,
            data: vec![7; len].into(),
        }
    }

    #[test]
    fn channel_name_maps_positionally() {
        assert_eq!(parse_channel_name("Alpha 1"), Some(0));
        assert_eq!(parse_channel_name("Alpha 3"), Some(2));
        assert_eq!(parse_channel_name("Alpha 0"), None);
        assert_eq!(parse_channel_name("alpha 1"), None);
        assert_eq!(parse_channel_name("Alpha x"), None);
        assert_eq!(parse_channel_name(""), None);
    }

    #[test]
    fn wrong_length_channel_is_refused() {
        let channels = [channel(1, 3)];
        assert!(selection_from_named_channel(&channels, 2, 2, "Alpha 1").is_none());
        assert!(selection_from_named_channel(&channels, 1, 3, "Alpha 1").is_some());
        assert!(selection_from_named_channel(&channels, 2, 2, "Alpha 2").is_none());
    }

    #[test]
    fn gray_and_ink_match_the_panel_derivation() {
        assert_eq!(q_gray(100, 150, 200), 140);
        assert_eq!(q_gray(0, 150, 200), 106);
        assert_eq!(q_gray(255, 255, 255), 255);
        // Pure red: no cyan or black ink, full magenta and yellow.
        assert_eq!(cmyk_ink(255, 0, 0, 0), 0);
        assert_eq!(cmyk_ink(255, 0, 0, 1), 255);
        assert_eq!(cmyk_ink(0, 0, 0, 3), 255);
    }

    #[test]
    fn channel_row_selection_reads_the_row_plane() {
        let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
        // Planes 0/1/2 are R/G/B.
        doc.composite.data = vec![100, 100, 150, 150, 200, 200].into();

        let red = channel_row_selection(&doc, false, 1, 0x7).unwrap();
        assert_eq!(red.data, vec![100, 100]);
        let green = channel_row_selection(&doc, false, 2, 0x7).unwrap();
        assert_eq!(green.data, vec![150, 150]);
        let blue = channel_row_selection(&doc, false, 3, 0x7).unwrap();
        assert_eq!(blue.data, vec![200, 200]);

        let composite = channel_row_selection(&doc, false, 0, 0x7).unwrap();
        assert_eq!(composite.data, vec![140, 140]);

        // A hidden red channel is zeroed in the composite row.
        let hidden = channel_row_selection(&doc, false, 0, 0x6).unwrap();
        assert_eq!(hidden.data, vec![106, 106]);

        // Rows past the colour channels are refused.
        assert!(channel_row_selection(&doc, false, 4, 0x7).is_none());
        assert!(channel_row_selection(&doc, false, -1, 0x7).is_none());
    }

    #[test]
    fn multichannel_rows_derive_naive_inks() {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.source_mode = Some(ColorMode::Multichannel);
        doc.composite.data = vec![255, 0, 0].into();
        // Row 0 is Cyan (ink 0), rows 1/2 Magenta and Yellow (full ink) — and
        // there is no composite row in Multichannel.
        assert_eq!(
            channel_row_selection(&doc, false, 0, 0x7).unwrap().data,
            vec![255]
        );
        assert_eq!(
            channel_row_selection(&doc, false, 1, 0x7).unwrap().data,
            vec![0]
        );
        assert_eq!(
            channel_row_selection(&doc, false, 2, 0x7).unwrap().data,
            vec![0]
        );
        assert!(channel_row_selection(&doc, false, 3, 0x7).is_none());
    }

    #[test]
    fn single_row_modes_have_no_second_channel() {
        let mut doc = Document::new(1, 1, ColorMode::Grayscale, BitDepth::Eight);
        doc.composite.data = vec![200].into();
        // The one Gray row is the composite; there is no colour channel after it.
        assert_eq!(
            channel_row_selection(&doc, false, 0, 0x7).unwrap().data,
            vec![200]
        );
        assert!(channel_row_selection(&doc, false, 1, 0x7).is_none());
    }
}
