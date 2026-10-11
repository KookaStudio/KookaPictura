use super::helpers::*;
use super::helpers_composite::*;
use super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::Channel;
use pictura_select::{CombineMode, Selection};

mod refine;

impl qobject::PictureView {
    pub fn select_all(mut self: Pin<&mut Self>) {
        let dims = self.rust().doc.as_ref().map(|d| (d.width, d.height));
        if let Some((w, h)) = dims {
            {
                let mut rust = self.as_mut().rust_mut();
                if let Some(prev) = rust.selection.take() {
                    if prev.data.iter().any(|&v| v > 0) {
                        rust.deselected_selection = Some(prev);
                    }
                }
                rust.selection = Some(Selection::all(w, h));
            }
            self.as_mut().record("Select All");
            self.changed();
        }
    }

    pub fn deselect(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            if let Some(prev) = rust.selection.take() {
                rust.deselected_selection = Some(prev);
            }
        }
        self.as_mut().record("Deselect");
        self.changed();
    }

    /// Restore the last deselected selection. Refuses (no history) without a
    /// stored selection or when its dimensions no longer match the document.
    pub fn reselect(mut self: Pin<&mut Self>) -> bool {
        let restored = {
            let rust = self.rust();
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            match rust.deselected_selection.as_ref() {
                Some(sel) if sel.width == doc.width && sel.height == doc.height => sel.clone(),
                _ => return false,
            }
        };
        self.as_mut().rust_mut().selection = Some(restored);
        self.as_mut().record("Reselect");
        self.changed();
        true
    }

    pub fn has_deselected_selection(&self) -> bool {
        self.rust().deselected_selection.is_some()
    }

    pub fn invert_selection(mut self: Pin<&mut Self>) -> bool {
        let inverted = self.rust().selection.as_ref().map(Selection::invert);
        let Some(inverted) = inverted else {
            return false;
        };
        self.as_mut().rust_mut().selection = Some(inverted);
        self.as_mut().record("Inverse Selection");
        self.changed();
        true
    }

    /// Capture the current selection as the origin of a move-selection drag.
    /// Refuses (false) without a selection.
    pub fn begin_selection_move(mut self: Pin<&mut Self>) -> bool {
        let Some(origin) = self.rust().selection.clone() else {
            return false;
        };
        self.as_mut().rust_mut().selection_move_origin = Some(origin);
        true
    }

    /// Set the selection to the origin translated by `(dx, dy)`. No history and
    /// no `changed` emission: the tool drives the overlay. False without a drag.
    pub fn preview_selection_move(mut self: Pin<&mut Self>, dx: i32, dy: i32) -> bool {
        let moved = {
            let rust = self.rust();
            let Some(origin) = rust.selection_move_origin.as_ref() else {
                return false;
            };
            origin.translate(dx, dy)
        };
        self.as_mut().rust_mut().selection = Some(moved);
        true
    }

    /// Commit the move: one "Move Selection" state when the mask actually moved,
    /// nothing when it lands on the origin. Clears the origin.
    pub fn commit_selection_move(mut self: Pin<&mut Self>) -> bool {
        let origin = self.as_mut().rust_mut().selection_move_origin.take();
        let Some(origin) = origin else {
            return false;
        };
        if self.rust().selection.as_ref() == Some(&origin) {
            return false;
        }
        self.as_mut().record("Move Selection");
        self.changed();
        true
    }

    /// Restore the origin selection and drop the drag. No history.
    pub fn cancel_selection_move(mut self: Pin<&mut Self>) -> bool {
        let Some(origin) = self.as_mut().rust_mut().selection_move_origin.take() else {
            return false;
        };
        self.as_mut().rust_mut().selection = Some(origin);
        true
    }

    /// Move (or, with `duplicate`, copy to a new layer) the pixels covered by the
    /// selection origin, offset by `(dx, dy)`. One "Move Selection" state; false
    /// (no state) on a zero offset, missing doc/selection, or engine refusal.
    pub fn move_selection_content(
        mut self: Pin<&mut Self>,
        dx: i32,
        dy: i32,
        duplicate: bool,
    ) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }
        let active = self.rust().active_layer.clone();
        let prepared = {
            let rust = self.rust();
            let (Some(doc), Some(selection)) = (
                rust.doc.as_ref(),
                rust.selection_move_origin
                    .clone()
                    .or_else(|| rust.selection.clone()),
            ) else {
                return false;
            };
            let Some(path) = active.as_deref() else {
                return false;
            };
            if active_pixel_layer(doc, Some(path)).is_none() {
                return false;
            }
            (path.to_string(), selection_to_mask(&selection, doc))
        };
        let (path, mask) = prepared;
        let moved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            pictura_render::move_selection_content(doc, &path, &mask, dx, dy, duplicate)
        };
        if !moved {
            return false;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            rust.selection_move_origin = None;
            if duplicate {
                // `layer_via_copy` inserts the copy directly above the source
                // in the source's own container.
                if let Some(copy) = layer_above(&path) {
                    rust.active_layer = Some(copy);
                }
            }
        }
        self.as_mut().recomposite();
        self.as_mut().record("Move Selection");
        true
    }

    /// Build the live Alt selection-content preview without touching the
    /// document: the full composite as base and the masked copy of the selected
    /// pixels as the moving layer, stored in the move-preview slots. The copy is
    /// produced by the same engine call the commit uses, so preview and commit
    /// agree. No history. False without a document, selection, editable active
    /// layer, or when the engine refuses (a locked source).
    pub fn begin_selection_duplicate_preview(mut self: Pin<&mut Self>) -> bool {
        let prepared = {
            let rust = self.rust();
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            let selection = rust
                .selection_move_origin
                .as_ref()
                .or(rust.selection.as_ref());
            let (Some(selection), Some(path)) = (selection, rust.active_layer.as_deref()) else {
                return false;
            };
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
        rust.move_prepared_revision = rust.content_revision;
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
            let Some(sel) = rust.selection.as_ref() else {
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
        self.as_mut().rust_mut().selection = Some(modified);
        self.as_mut().record(label);
        self.changed();
        true
    }

    pub fn grow_selection(mut self: Pin<&mut Self>, tolerance: i32) -> bool {
        let grown = {
            let rust = self.rust();
            let (Some(doc), Some(sel)) = (rust.doc.as_ref(), rust.selection.as_ref()) else {
                return false;
            };
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::grow(sel, &current_buffer(doc, rust.gpu_compute), tolerance) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        self.as_mut().rust_mut().selection = Some(grown);
        self.as_mut().record("Grow Selection");
        self.changed();
        true
    }

    pub fn similar_selection(mut self: Pin<&mut Self>, tolerance: i32) -> bool {
        let matched = {
            let rust = self.rust();
            let (Some(doc), Some(sel)) = (rust.doc.as_ref(), rust.selection.as_ref()) else {
                return false;
            };
            let tolerance = tolerance.clamp(0, 255) as u8;
            match pictura_select::similar(sel, &current_buffer(doc, rust.gpu_compute), tolerance) {
                Ok(selection) => selection,
                Err(_) => return false,
            }
        };
        self.as_mut().rust_mut().selection = Some(matched);
        self.as_mut().record("Similar Selection");
        self.changed();
        true
    }

    /// Append the coverage as a document extra channel. The dialog `name` is
    /// accepted but not stored: `Channel` has only `id`.
    // ponytail: names map positionally to `Alpha N` = channels[N-1]; add a
    // `Channel.name` field only if PSD round-tripping of names is required.
    pub fn save_selection(mut self: Pin<&mut Self>, _name: &QString) -> bool {
        let saved = {
            let mut rust = self.as_mut().rust_mut();
            let Some(sel) = rust.selection.clone() else {
                return false;
            };
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            let id = doc.channels.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            doc.channels.push(sel.to_channel(id));
            true
        };
        if saved {
            self.as_mut().record("Save Selection");
        }
        saved
    }

    pub fn load_selection(mut self: Pin<&mut Self>, name: &QString) -> bool {
        let loaded = {
            let rust = self.rust();
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            selection_from_named_channel(&doc.channels, doc.width, doc.height, &name.to_string())
        };
        let Some(selection) = loaded else {
            return false;
        };
        self.as_mut().rust_mut().selection = Some(selection);
        self.as_mut().record("Load Selection");
        self.changed();
        true
    }

    pub fn selection_channel_count(&self) -> i32 {
        self.rust()
            .doc
            .as_ref()
            .map_or(0, |d| d.channels.len() as i32)
    }

    /// Every layer row path in panel order (depth-first, topmost-first).
    pub fn select_all_layers(&self) -> QStringList {
        self.rust()
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
            let Some(doc) = rust.doc.as_ref() else {
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
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            let Some(layer) = pictura_render::resolve_path(doc, &path) else {
                return false;
            };
            pictura_select::selection_from_layer_alpha(layer, doc.width, doc.height)
        };
        self.as_mut().apply_selection(shape, CombineMode::New)
    }

    pub fn has_selection(&self) -> bool {
        self.rust().selection.is_some()
    }

    pub fn selection_count(&self) -> i32 {
        self.rust()
            .selection
            .as_ref()
            .map_or(0, |s| s.data.iter().filter(|&&v| v > 0).count() as i32)
    }

    /// Coverage byte at `(x, y)` (0 outside, 255 fully inside, intermediate on
    /// a feathered edge). 0 without a document/selection or out of bounds.
    pub fn selection_coverage(&self, x: i32, y: i32) -> i32 {
        let rust = self.rust();
        let Some(sel) = rust.selection.as_ref() else {
            return 0;
        };
        if x < 0 || y < 0 || x as u32 >= sel.width || y as u32 >= sel.height {
            return 0;
        }
        sel.data[y as usize * sel.width as usize + x as usize] as i32
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
            if rust.doc.is_none() {
                return false;
            }
            let previous = rust.selection.take();
            let replaced = matches!(mode, CombineMode::New)
                && previous
                    .as_ref()
                    .is_some_and(|s| s.data.iter().any(|&v| v > 0));
            if replaced {
                rust.deselected_selection = previous;
                rust.selection = Some(shape);
            } else {
                rust.deselected_selection = None;
                let mut base =
                    previous.unwrap_or_else(|| Selection::none(shape.width, shape.height));
                base.combine_with(&shape, mode);
                rust.selection = Some(base);
            }
        }
        self.as_mut().record(label);
        self.changed();
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
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            Selection::rect(doc.width, doc.height, x, y, w, h).feather(feather)
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
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            Selection::ellipse(doc.width, doc.height, x, y, w, h).feather(feather)
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    pub fn begin_lasso(mut self: Pin<&mut Self>, mode: &QString) -> bool {
        if self.rust().doc.is_none() {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        rust.pending_lasso_mode = mode.to_string();
        rust.pending_lasso.clear();
        true
    }

    pub fn lasso_add_point(mut self: Pin<&mut Self>, x: i32, y: i32) {
        self.as_mut().rust_mut().pending_lasso.push((x, y));
    }

    pub fn end_lasso(mut self: Pin<&mut Self>, feather: f64) -> bool {
        let shape = {
            let rust = self.rust();
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            if rust.pending_lasso.len() < 3 {
                return false;
            }
            Selection::polygon(doc.width, doc.height, &rust.pending_lasso).feather(feather)
        };
        let mode = combine_mode_from(&self.rust().pending_lasso_mode);
        if !self.as_mut().apply_selection(shape, mode) {
            return false;
        }
        let mut rust = self.as_mut().rust_mut();
        rust.pending_lasso.clear();
        rust.pending_lasso_mode.clear();
        true
    }

    pub fn cancel_lasso(mut self: Pin<&mut Self>) {
        let mut rust = self.as_mut().rust_mut();
        rust.pending_lasso.clear();
        rust.pending_lasso_mode.clear();
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
            let Some(doc) = rust.doc.as_ref() else {
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
            let Some(doc) = rust.doc.as_mut() else {
                return false;
            };
            if !pictura_render::crop_document(doc, x, y, w as u32, h as u32) {
                return false;
            }
            // Canvas dimensions changed, so the old selection no longer maps.
            rust.selection = None;
            true
        };
        if cropped {
            self.as_mut().recomposite();
            self.as_mut().record("Crop");
        }
        cropped
    }

    pub fn selection_bounds(&self) -> QString {
        let rust = self.rust();
        let Some(sel) = rust.selection.as_ref() else {
            return QString::default();
        };
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for (i, &v) in sel.data.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let x = i as u32 % sel.width;
            let y = i as u32 / sel.width;
            bounds = Some(match bounds {
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                None => (x, y, x, y),
            });
        }
        let Some((x0, y0, x1, y1)) = bounds else {
            return QString::default();
        };
        QString::from(format!("{x0} {y0} {} {}", x1 - x0 + 1, y1 - y0 + 1))
    }

    /// The 50 %-coverage outline of the selection as `"x,y x,y ..."` polylines
    /// joined by `;` (integer pixel corners). Empty string without a selection.
    /// View-only: derives from the mask, records no history.
    pub fn selection_contour(&self) -> QString {
        let rust = self.rust();
        let Some(sel) = rust.selection.as_ref() else {
            return QString::default();
        };
        let loops = pictura_select::contour(sel, 128);
        let mut parts: Vec<String> = Vec::with_capacity(loops.len());
        for poly in &loops {
            let points: Vec<String> = poly.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
            parts.push(points.join(" "));
        }
        QString::from(parts.join(";"))
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
