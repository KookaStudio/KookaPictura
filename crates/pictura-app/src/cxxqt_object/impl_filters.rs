use super::document_session::{FilterPreview, PreviewBaseline};
use super::filter_map::filter_from_kind_params;
use super::helpers::*;
use super::helpers_composite::*;
use super::qobject;
use super::state::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
#[cfg(test)]
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{layer_pixel_locked, Layer};
use rayon::prelude::*;

impl qobject::PictureView {
    pub fn add_adjustment(mut self: Pin<&mut Self>, kind: &QString) -> bool {
        let mask = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            if !rust.use_default_masks {
                None
            } else {
                rust.session
                    .selection
                    .as_ref()
                    .map(|selection| selection_to_mask(selection, doc))
            }
        };
        let Some(layer) = adjustment_layer(&kind.to_string(), mask) else {
            return false;
        };
        let region = layer_visibility_region(&layer);
        let pushed = match self.as_mut().rust_mut().session.doc.as_mut() {
            Some(doc) => doc.layers.push(layer).is_ok(),
            None => false,
        };
        if pushed {
            // A masked adjustment is confined to its mask; an unmasked one spans
            // the canvas and falls back to a full recomposite.
            let outcome = match region {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            };
            self.as_mut().edit("Adjustment", |_| outcome);
        }
        pushed
    }

    pub fn apply_filter(mut self: Pin<&mut Self>, kind: &QString) -> bool {
        let Some(filter) = filter_from_kind(&kind.to_string()) else {
            return false;
        };
        if let Some(path) = active_mask_target(self.rust()) {
            return self
                .as_mut()
                .apply_mask_filter_command(&path, &kind.to_string(), filter);
        }
        let mask = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            rust.session
                .selection
                .as_ref()
                .map(|selection| selection_to_mask(selection, doc))
        };
        let active = self.rust().layer_target.single().map(str::to_string);
        let applied = {
            let mut rust = self.as_mut().rust_mut();
            let gpu_compute = rust.gpu_compute;
            let Some(doc) = rust.session.doc.as_mut() else {
                return false;
            };
            if !active_layer_visible(doc, active.as_deref()) {
                return false;
            }
            let Some(layer) = active_pixel_layer_mut(doc, active.as_deref()) else {
                return false;
            };
            // A filter only changes the active layer, so its clamped rect bounds
            // the composited result — unless a layer effect spills past that
            // rect, which needs a full recomposite.
            let region = (!layer_has_effects(layer)).then_some(layer.rect);
            pictura_render::apply_filter(layer, &filter, mask.as_ref(), gpu_compute)
                .is_ok()
                .then_some(region)
        };
        if let Some(region) = applied {
            self.as_mut().rust_mut().last_filter = Some((kind.to_string(), Vec::new()));
            let outcome = match region {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            };
            self.as_mut().edit("Filter", |_| outcome);
            return true;
        }
        false
    }

    /// The `Filter` command with the active mask as the target: filter the
    /// mask's coverage over its rectangle and record one "Filter" state.
    fn apply_mask_filter_command(
        mut self: Pin<&mut Self>,
        path: &str,
        kind: &str,
        filter: pictura_filters::Filter,
    ) -> bool {
        let region = {
            let mut rust = self.as_mut().rust_mut();
            apply_mask_op(&mut rust, path, &ActiveOp::Filter(filter), true, None)
        };
        let Some(region) = region else {
            return false;
        };
        self.as_mut().rust_mut().last_filter = Some((kind.to_string(), Vec::new()));
        let outcome = match region {
            Some(rect) => Edit::Regions {
                rects: vec![rect],
                notify: false,
            },
            None => Edit::Full,
        };
        self.as_mut().edit("Filter", |_| outcome);
        true
    }
}

/// Apply a filter to the single editable layer, either committing it or
/// rendering a non-committing preview.
///
/// On a preview, the first parameter change snapshots the layer's pixels and
/// every later change re-filters that snapshot, so previews never compound. A
/// commit restores the snapshot first (so it never double-applies) and records
/// the `(kind, params)` as the last filter. Returns the region to refresh, or
/// `None` for an unknown kind, a bad arity, or no editable layer.
pub(super) fn apply_filter_active(
    rust: &mut PictureViewRust,
    kind: &str,
    params: &[f64],
    commit: bool,
) -> Option<Option<pictura_core::PsdRect>> {
    apply_filter_active_region(rust, kind, params, commit, None)
}

/// As [`apply_filter_active`], but a preview is restricted to `preview_region`
/// (a document rect), so dragging a control only filters the visible section.
/// A commit always filters the whole layer.
pub(super) fn apply_filter_active_region(
    rust: &mut PictureViewRust,
    kind: &str,
    params: &[f64],
    commit: bool,
    preview_region: Option<pictura_core::PsdRect>,
) -> Option<Option<pictura_core::PsdRect>> {
    rust.filter_error = None;
    let Some(filter) = filter_from_kind_params(kind, params) else {
        rust.filter_error = Some(format!(
            "filter '{kind}' is not recognised or its parameters are wrong"
        ));
        return None;
    };
    let region = apply_filter_obj_active_region(rust, filter, commit, preview_region)?;
    if commit {
        rust.last_filter = Some((kind.to_string(), params.to_vec()));
    }
    Some(region)
}

/// As [`apply_filter_active_region`], but from a resolved [`pictura_filters::Filter`]
/// instead of a kind/params pair. Filters that are not in the parameterised
/// mapper (HDR Toning) reach the same snapshot/preview/commit core this way.
pub(super) fn apply_filter_obj_active_region(
    rust: &mut PictureViewRust,
    filter: pictura_filters::Filter,
    commit: bool,
    preview_region: Option<pictura_core::PsdRect>,
) -> Option<Option<pictura_core::PsdRect>> {
    apply_op_active_region(
        rust,
        &ActiveOp::Filter(filter),
        commit,
        preview_region,
        false,
    )
}

/// A destructive edit of the active pixel layer: a filter, or an Image >
/// Adjustments adjustment (colour only, no neighbourhood).
pub(super) enum ActiveOp {
    Filter(pictura_filters::Filter),
    /// A Filter Gallery stack, applied in order.
    Filters(Vec<pictura_filters::Filter>),
    Adjustment(pictura_render::Adjustment),
}

/// Apply `op` to the active pixel layer within the selection, as a preview
/// (re-applied from the pre-preview pixels, restricted to `preview_region`)
/// or a commit. `ignore_selection` skips the selection confinement, so the
/// whole layer is adjusted (Match Color's "Ignore Selection When Applying
/// Adjustment"). Returns the region to refresh; `None` with
/// `filter_error` set when refused.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_op_active_region(
    rust: &mut PictureViewRust,
    op: &ActiveOp,
    commit: bool,
    preview_region: Option<pictura_core::PsdRect>,
    ignore_selection: bool,
) -> Option<Option<pictura_core::PsdRect>> {
    rust.filter_error = None;
    if !commit && !rust.slot_allows(super::document_session::SessionKind::FilterPreview) {
        rust.filter_error = Some("another session is live".to_string());
        return None;
    }
    if let Some(path) = active_mask_target(rust) {
        return apply_mask_op(rust, &path, op, commit, preview_region);
    }
    let path = pixel_filter_target(rust)?;
    let gpu_compute = rust.gpu_compute;
    let mask = {
        if ignore_selection {
            None
        } else {
            let doc = rust.session.doc.as_ref()?;
            rust.session
                .selection
                .as_ref()
                .map(|selection| selection_to_mask(selection, doc))
        }
    };
    let same_target = rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path == path);
    let existing = if same_target {
        rust.session.live.take_filter_preview()
    } else {
        None
    };
    let Some(doc) = rust.session.doc.as_mut() else {
        rust.filter_error = Some("there is no document".to_string());
        return None;
    };
    let Some(layer) = pictura_render::resolve_path_mut(doc, &path) else {
        rust.filter_error = Some("the active layer no longer exists".to_string());
        return None;
    };
    // Re-filter from the pre-preview pixels, never from the previous preview.
    let restored = existing.map(|preview| preview.baseline.restore(layer));
    if layer.is_group || layer.adjustment.is_some() {
        rust.filter_error = Some("the active layer cannot take a filter".to_string());
        return None;
    }
    let lrect = layer.rect;
    let filters = match op {
        ActiveOp::Filter(filter) => std::slice::from_ref(filter),
        ActiveOp::Filters(filters) => filters.as_slice(),
        ActiveOp::Adjustment(_) => &[],
    };
    // Expand the visible rect by the support of every filter still to run, so
    // the viewport is exact after the whole stack; the renderer clamps it to
    // the layer. A point operation needs no apron.
    let mut apron: i32 = filters.iter().map(|filter| filter.preview_apron()).sum();
    let section = preview_region
        .filter(|_| !commit)
        .map(|visible| clamp_to(expand(visible, apron), lrect));
    // The composite changes where this call writes and where the last preview
    // was put back; a layer effect can spill anywhere.
    let region_out = (!layer_has_effects(layer)).then(|| {
        let written = section.unwrap_or(lrect);
        match restored {
            Some(r) if written.width() > 0 && written.height() > 0 => union_rect(r, written),
            Some(r) => r,
            None => written,
        }
    });
    let baseline = (!commit).then(|| PreviewBaseline::save(layer, section));
    let applied = match op {
        ActiveOp::Adjustment(adjustment) => pictura_render::apply_adjustment_region(
            layer,
            adjustment,
            mask.as_ref(),
            section.unwrap_or(lrect),
        ),
        _ => {
            let mut applied = Ok(());
            for filter in filters {
                applied = match preview_region {
                    Some(visible) if !commit => pictura_render::apply_filter_region(
                        layer,
                        filter,
                        mask.as_ref(),
                        gpu_compute,
                        expand(visible, apron),
                    ),
                    _ => pictura_render::apply_filter(layer, filter, mask.as_ref(), gpu_compute),
                };
                if applied.is_err() {
                    break;
                }
                apron -= filter.preview_apron();
            }
            applied
        }
    };
    if let Err(error) = applied {
        // A refused preview leaves the original pixels and no preview open.
        if let Some(baseline) = baseline {
            baseline.restore(layer);
        }
        rust.filter_error = Some(error.to_string());
        return None;
    }
    match baseline {
        Some(baseline) => rust.session.live.set_filter_preview(FilterPreview {
            layer_path: path,
            baseline,
        }),
        None => rust.session.live.clear_filter_preview(),
    }
    Some(region_out)
}

/// The panel path of the active pixel layer a filter may change, or `None`
/// with `filter_error` set: no document, a hidden or pixel-locked layer, no
/// single pixel layer, or an open preview on another target.
pub(super) fn pixel_filter_target(rust: &mut PictureViewRust) -> Option<String> {
    let Some(doc) = rust.session.doc.as_ref() else {
        rust.filter_error = Some("there is no document".to_string());
        return None;
    };
    let active = rust.layer_target.single();
    if !active_layer_visible(doc, active) {
        rust.filter_error = Some("the active layer is hidden".to_string());
        return None;
    }
    let Some(layer) = active_pixel_layer(doc, active) else {
        rust.filter_error = Some("select a single pixel layer to filter".to_string());
        return None;
    };
    if layer_pixel_locked(layer) {
        rust.filter_error = Some("the active layer is pixel-locked".to_string());
        return None;
    }
    if layer.is_group || layer.adjustment.is_some() {
        rust.filter_error = Some("the active layer cannot take a filter".to_string());
        return None;
    }
    let Some(path) = active.map(str::to_string) else {
        rust.filter_error = Some("the active layer has no filterable target".to_string());
        return None;
    };
    // A preview for another layer must not be silently abandoned: refuse so its
    // baseline stays intact until it is cancelled or committed.
    if rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path != path)
    {
        rust.filter_error = Some("another layer has an open preview; cancel it first".to_string());
        return None;
    }
    if rust.session.live.mask_filter_preview().is_some() {
        rust.filter_error = Some("a mask preview is open; cancel it first".to_string());
        return None;
    }
    Some(path)
}

/// `rect` grown by `by` on every side.
fn expand(rect: pictura_core::PsdRect, by: i32) -> pictura_core::PsdRect {
    pictura_core::PsdRect {
        top: rect.top - by,
        left: rect.left - by,
        bottom: rect.bottom + by,
        right: rect.right + by,
    }
}

/// `rect` clamped to `bounds`; an empty result keeps a zero size.
fn clamp_to(rect: pictura_core::PsdRect, bounds: pictura_core::PsdRect) -> pictura_core::PsdRect {
    let top = rect.top.clamp(bounds.top, bounds.bottom);
    let left = rect.left.clamp(bounds.left, bounds.right);
    pictura_core::PsdRect {
        top,
        left,
        bottom: rect.bottom.clamp(top, bounds.bottom),
        right: rect.right.clamp(left, bounds.right),
    }
}

impl PreviewBaseline {
    /// Keep what a preview over `section` (the whole layer for `None`) is
    /// about to overwrite.
    fn save(layer: &Layer, section: Option<pictura_core::PsdRect>) -> Self {
        let Some(rect) = section else {
            return Self::Layer(Box::new(layer.clone()));
        };
        let lrect = layer.rect;
        let lw = lrect.width() as usize;
        let full = lw * lrect.height() as usize;
        let (ox, oy) = (
            (rect.left - lrect.left) as usize,
            (rect.top - lrect.top) as usize,
        );
        let (cw, ch) = (rect.width() as usize, rect.height() as usize);
        let planes = layer
            .channels
            .iter()
            .filter(|c| c.data.len() == full)
            .map(|c| {
                let mut saved = vec![0u8; cw * ch];
                if cw > 0 {
                    saved.par_chunks_mut(cw).enumerate().for_each(|(row, out)| {
                        let from = (oy + row) * lw + ox;
                        out.copy_from_slice(&c.data[from..from + cw]);
                    });
                }
                (c.id, saved)
            })
            .collect();
        Self::Section {
            layer_rect: lrect,
            rect,
            planes,
        }
    }

    /// Put the saved pixels back into `layer`; returns the rect restored.
    pub(super) fn restore(&self, layer: &mut Layer) -> pictura_core::PsdRect {
        let (layer_rect, rect, planes) = match self {
            Self::Layer(original) => {
                *layer = (**original).clone();
                return layer.rect;
            }
            Self::Section {
                layer_rect,
                rect,
                planes,
            } => (*layer_rect, *rect, planes),
        };
        let (cw, ch) = (rect.width() as usize, rect.height() as usize);
        if layer.rect != layer_rect || cw == 0 || ch == 0 {
            return rect;
        }
        let lw = layer_rect.width() as usize;
        let (ox, oy) = (
            (rect.left - layer_rect.left) as usize,
            (rect.top - layer_rect.top) as usize,
        );
        for (id, saved) in planes {
            let Some(channel) = layer.channels.iter_mut().find(|c| c.id == *id) else {
                continue;
            };
            let rows = oy * lw..(oy + ch) * lw;
            channel.data.as_mut_slice()[rows]
                .par_chunks_mut(lw)
                .zip(saved.par_chunks(cw))
                .for_each(|(out, src)| out[ox..ox + cw].copy_from_slice(src));
        }
        rect
    }

    /// The layer as it was before the preview, given the live one.
    pub(super) fn original<'a>(&'a self, live: &'a Layer) -> std::borrow::Cow<'a, Layer> {
        match self {
            Self::Layer(original) => std::borrow::Cow::Borrowed(original.as_ref()),
            Self::Section { .. } => {
                let mut original = live.clone();
                self.restore(&mut original);
                std::borrow::Cow::Owned(original)
            }
        }
    }
}

/// Apply `op` to the active layer's mask coverage instead of its pixels: the
/// mask is exposed as a mask-sized grayscale document, filtered whole (the
/// document selection confines it), and written back. A preview snapshots the
/// pre-preview coverage and re-filters from it; a commit drops the snapshot;
/// cancel restores it. Returns the mask rectangle to refresh.
fn apply_mask_op(
    rust: &mut PictureViewRust,
    path: &str,
    op: &ActiveOp,
    commit: bool,
    _preview_region: Option<pictura_core::PsdRect>,
) -> Option<Option<pictura_core::PsdRect>> {
    // A preview for another target must not be silently abandoned.
    if rust.session.live.filter_preview().is_some()
        || rust
            .session
            .live
            .mask_filter_preview()
            .is_some_and(|(p, _)| p != path)
    {
        rust.filter_error = Some("another layer has an open preview; cancel it first".to_string());
        return None;
    }
    let baseline = rust
        .session
        .live
        .mask_filter_preview()
        .filter(|(p, _)| p.as_str() == path)
        .map(|(_, coverage)| coverage.clone());
    let (mut mask, selection) = {
        let doc = rust.session.doc.as_ref()?;
        let Some(mut mask) = pictura_render::mask_document(doc, path) else {
            rust.filter_error = Some("the active layer has no mask to filter".to_string());
            return None;
        };
        // Re-filter from the pre-preview coverage, never from the last preview.
        if let Some(original) = &baseline {
            if let Some(channel) = mask.document.layers[0]
                .channels
                .iter_mut()
                .find(|c| c.id == 0)
            {
                if channel.data.len() == original.len() {
                    channel.data = original.clone().into();
                }
            }
        }
        let selection = rust
            .session
            .selection
            .as_ref()
            .map(|s| selection_mask_for(s, mask.rect));
        (mask, selection)
    };
    let snapshot = if !commit && baseline.is_none() {
        mask.document.layers[0]
            .channels
            .iter()
            .find(|c| c.id == 0)
            .map(|c| c.data.to_vec())
    } else {
        None
    };
    let result = {
        let layer = mask.document.layers.first_mut()?;
        let rect = layer.rect;
        match op {
            ActiveOp::Filter(filter) => {
                pictura_render::apply_filter(layer, filter, selection.as_ref(), false)
            }
            ActiveOp::Filters(filters) => filters.iter().try_for_each(|filter| {
                pictura_render::apply_filter(layer, filter, selection.as_ref(), false)
            }),
            ActiveOp::Adjustment(adjustment) => {
                pictura_render::apply_adjustment_region(layer, adjustment, selection.as_ref(), rect)
            }
        }
    };
    if let Err(error) = result {
        rust.filter_error = Some(error.to_string());
        return None;
    }
    let doc = rust.session.doc.as_mut()?;
    if !pictura_render::write_mask_back(doc, path, &mask.document, None) {
        rust.filter_error = Some("the mask disappeared during the filter".to_string());
        return None;
    }
    if commit {
        rust.session.live.clear_mask_filter_preview();
    } else {
        let original = baseline.or(snapshot).expect("a preview has a baseline");
        rust.session
            .live
            .set_mask_filter_preview(path.to_string(), original);
    }
    Some(Some(mask.rect))
}

/// Discard an open preview, restoring the pre-preview pixels bit-identically.
/// Returns the region to refresh, or `None` when no preview is open.
pub(super) fn cancel_filter_preview(
    rust: &mut PictureViewRust,
) -> Option<Option<pictura_core::PsdRect>> {
    let session = rust.session.live.take_filter_preview_session()?;
    cancel_filter_preview_session(rust, session)
}

/// Discard a filter preview session, restoring the pre-preview pixels
/// bit-identically. Returns the region to refresh, or `None` when the preview
/// had nothing to restore.
pub(super) fn cancel_filter_preview_session(
    rust: &mut PictureViewRust,
    session: super::document_session::FilterPreviewSession,
) -> Option<Option<pictura_core::PsdRect>> {
    match session {
        super::document_session::FilterPreviewSession::Mask { path, original } => {
            let doc = rust.session.doc.as_mut()?;
            let mut mask = pictura_render::mask_document(doc, &path)?;
            if let Some(channel) = mask.document.layers[0]
                .channels
                .iter_mut()
                .find(|c| c.id == 0)
            {
                if channel.data.len() == original.len() {
                    channel.data = original.into();
                }
            }
            pictura_render::write_mask_back(doc, &path, &mask.document, None);
            Some(Some(mask.rect))
        }
        super::document_session::FilterPreviewSession::Layer(preview) => {
            let doc = rust.session.doc.as_mut()?;
            let layer = pictura_render::resolve_path_mut(doc, &preview.layer_path)?;
            let restored = preview.baseline.restore(layer);
            Some((!layer_has_effects(layer)).then_some(restored))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cxxqt_object::tests::pixel_layer;
    use pictura_core::{Document, LockFlags};

    fn state() -> PictureViewRust {
        let mut doc = Document::new(
            16,
            16,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        let mut layer = pixel_layer("px", 16, 16, (40, 120, 200));
        // A left/right step so a blur has a gradient to smooth.
        for ch in layer.channels.iter_mut().filter(|c| c.id >= 0) {
            ch.data = (0..16 * 16)
                .map(|i| if i % 16 < 8 { 40u8 } else { 200 })
                .collect();
        }
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            gpu_compute: false,
            ..Default::default()
        }
    }

    fn red(rust: &PictureViewRust) -> Vec<u8> {
        rust.session.doc.as_ref().unwrap().layers[0]
            .channels
            .iter()
            .find(|c| c.id == 0)
            .unwrap()
            .data
            .to_vec()
    }

    /// A 256×256 layer large enough that a section preview's apron does not
    /// span the whole layer, so a crop changes a positional filter's result.
    fn big_state() -> PictureViewRust {
        let mut doc = Document::new(
            256,
            256,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        let mut layer = pixel_layer("px", 256, 256, (40, 120, 200));
        for (c, ch) in layer.channels.iter_mut().filter(|c| c.id >= 0).enumerate() {
            let plane: Vec<u8> = (0..256 * 256)
                .map(|i| {
                    let x = i % 256;
                    let y = i / 256;
                    ((x * 7 + y * 13 + c * 53) % 256) as u8
                })
                .collect();
            ch.data = plane.into();
        }
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        PictureViewRust {
            session: crate::cxxqt_object::document_session::DocumentSession::with_document(doc),
            layer_target: LayerTarget::only("0"),
            gpu_compute: false,
            ..Default::default()
        }
    }

    #[test]
    fn wrong_arity_is_refused_without_mutation() {
        let mut rust = state();
        let before = red(&rust);
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[1.0, 2.0], false).is_none());
        assert!(apply_filter_active(&mut rust, "bogus", &[], false).is_none());
        assert_eq!(red(&rust), before, "refusal must not mutate the layer");
        assert!(rust.session.live.filter_preview().is_none());
        assert!(rust.last_filter.is_none());
        assert!(
            rust.filter_error.as_deref().is_some_and(|e| !e.is_empty()),
            "a refusal leaves a reason"
        );
    }

    #[test]
    fn success_clears_the_refusal_reason() {
        let mut rust = state();
        assert!(apply_filter_active(&mut rust, "bogus", &[], false).is_none());
        assert!(rust.filter_error.is_some());
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_some());
        assert!(rust.filter_error.is_none(), "success clears the reason");
    }

    #[test]
    fn locked_layer_is_refused() {
        let mut rust = state();
        rust.session.doc.as_mut().unwrap().layers[0].lock =
            LockFlags::default().with(LockFlags::PIXELS, true);
        let before = red(&rust);
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_none());
        assert_eq!(red(&rust), before);
    }

    #[test]
    fn non_pixel_active_layer_is_refused() {
        let mut rust = state();
        let pixel_before = red(&rust);
        // An adjustment layer (no pixels) as the sole active target.
        let adj = adjustment_layer("invert", None).expect("adjustment");
        {
            let _ = rust.session.doc.as_mut().unwrap().layers.push(adj);
        };
        rust.select_only_layer("1");
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).is_none());
        assert_eq!(red(&rust), pixel_before, "pixel layer untouched");
        assert!(rust.last_filter.is_none());
    }

    #[test]
    fn preview_on_another_layer_is_refused_and_keeps_baseline() {
        let mut rust = state();
        let _ = rust.session.doc.as_mut().unwrap().layers.push(pixel_layer(
            "px2",
            16,
            16,
            (10, 20, 30),
        ));
        let original = red(&rust);
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).is_some());
        assert!(rust
            .session
            .live
            .filter_preview()
            .is_some_and(|p| p.layer_path == "0"));
        // Switching the active layer must not steal layer 0's baseline.
        rust.select_only_layer("1");
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).is_none());
        assert!(rust
            .session
            .live
            .filter_preview()
            .is_some_and(|p| p.layer_path == "0"));
        assert!(cancel_filter_preview(&mut rust).is_some());
        assert_eq!(red(&rust), original, "layer 0 baseline restored");
    }

    #[test]
    fn preview_re_filters_the_original_not_the_last_preview() {
        let mut rust = state();
        let original = red(&rust);
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).is_some());
        assert!(
            rust.session.live.filter_preview().is_some(),
            "preview state established"
        );
        assert!(rust.last_filter.is_none(), "preview commits nothing");
        let after_two = red(&rust);
        // A second, different radius must equal filtering the original once.
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[6.0], false).is_some());
        let after_six = red(&rust);
        let mut expected = state();
        apply_filter_active(&mut expected, "gaussian-blur", &[6.0], false);
        assert_eq!(after_six, red(&expected));
        assert_ne!(after_six, after_two);
        assert_ne!(after_six, original);
    }

    #[test]
    fn cancel_restores_bit_identical() {
        let mut rust = state();
        let original = red(&rust);
        apply_filter_active(&mut rust, "gaussian-blur", &[4.0], false).expect("preview");
        assert_ne!(red(&rust), original, "preview changed pixels");
        let region = cancel_filter_preview(&mut rust).expect("preview open");
        assert!(region.is_some());
        assert_eq!(red(&rust), original, "cancel is bit-identical");
        assert!(rust.session.live.filter_preview().is_none());
        assert!(rust.last_filter.is_none());
    }

    #[test]
    fn commit_clears_preview_and_records_last_filter() {
        let mut rust = state();
        apply_filter_active(&mut rust, "gaussian-blur", &[3.0], false).expect("preview");
        assert!(apply_filter_active(&mut rust, "gaussian-blur", &[3.0], true).is_some());
        assert!(
            rust.session.live.filter_preview().is_none(),
            "commit closes the preview"
        );
        assert_eq!(
            rust.last_filter,
            Some(("gaussian-blur".to_string(), vec![3.0]))
        );
        // Committing after a preview must equal a single fresh apply.
        let mut fresh = state();
        apply_filter_active(&mut fresh, "gaussian-blur", &[3.0], true).expect("commit");
        assert_eq!(red(&rust), red(&fresh), "no double application");
    }

    /// A section preview writes and reports only its section; the next one
    /// puts that section back first, so the layer never compounds and the
    /// refresh covers both.
    #[test]
    fn section_previews_restore_only_what_they_wrote() {
        let rect = |top, left, bottom, right| pictura_core::PsdRect {
            top,
            left,
            bottom,
            right,
        };
        let original = red(&big_state());
        let mut rust = big_state();
        let a = rect(16, 16, 48, 48);
        let b = rect(200, 200, 232, 232);
        // Gaussian radius 3 keeps the minimum 8 px apron.
        let (a_out, b_out) = (rect(8, 8, 56, 56), rect(192, 192, 240, 240));
        let region = apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(a))
            .expect("preview a");
        assert_eq!(region, Some(a_out));
        let region = apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(b))
            .expect("preview b");
        assert_eq!(region, Some(rect(8, 8, 240, 240)), "a put back, b written");

        let mut fresh = big_state();
        apply_filter_active_region(&mut fresh, "gaussian-blur", &[3.0], false, Some(b))
            .expect("fresh preview b");
        assert_eq!(red(&rust), red(&fresh), "section a was restored");
        let inside = |i: usize| {
            let (x, y) = ((i % 256) as i32, (i / 256) as i32);
            (b_out.left..b_out.right).contains(&x) && (b_out.top..b_out.bottom).contains(&y)
        };
        let px = red(&rust);
        assert!((0..px.len()).all(|i| inside(i) || px[i] == original[i]));
        assert!((0..px.len()).any(|i| px[i] != original[i]));

        let region = cancel_filter_preview(&mut rust).expect("preview open");
        assert_eq!(region, Some(b_out), "cancel refreshes the last section");
        assert_eq!(red(&rust), original, "cancel is bit-identical");

        apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], false, Some(a))
            .expect("preview a again");
        let region = apply_filter_active_region(&mut rust, "gaussian-blur", &[3.0], true, None)
            .expect("commit");
        assert_eq!(region, Some(rect(0, 0, 256, 256)));
        let mut fresh = big_state();
        apply_filter_active_region(&mut fresh, "gaussian-blur", &[3.0], true, None)
            .expect("fresh commit");
        assert_eq!(
            red(&rust),
            red(&fresh),
            "no section of the preview leaks into the commit"
        );
    }

    /// A section preview keeps a copy of its section alone, so the live
    /// planes are written where they are, never copied whole.
    #[test]
    fn section_previews_write_the_live_planes_in_place() {
        let mut rust = big_state();
        let plane = |rust: &PictureViewRust| {
            rust.session.doc.as_ref().unwrap().layers[0].channels[0]
                .data
                .as_ptr()
        };
        let before = plane(&rust);
        let section = pictura_core::PsdRect {
            top: 100,
            left: 100,
            bottom: 140,
            right: 140,
        };
        for radius in [3.0, 5.0] {
            apply_filter_active_region(&mut rust, "gaussian-blur", &[radius], false, Some(section))
                .expect("section preview");
            assert_eq!(plane(&rust), before, "radius {radius} copied the plane");
        }
        cancel_filter_preview(&mut rust).expect("preview open");
        assert_eq!(plane(&rust), before);
        assert_eq!(red(&rust), red(&big_state()));
    }

    #[test]
    fn positional_kinds_preview_the_whole_layer() {
        use super::super::filter_map::SHEAR_SLOTS;
        use super::super::filter_tools::filter_preview_needs_whole_layer;

        let viewport = pictura_core::PsdRect {
            top: 200,
            left: 200,
            bottom: 232,
            right: 232,
        };
        for (kind, params) in [
            ("diffuse", vec![0.0f64]),
            (
                "lighting-effects",
                vec![
                    255.0, 255.0, 255.0, 0.0, 0.0, 0.0, 0.0, 0.0, 50.0, // rig
                    0.0, 1.0, 255.0, 255.0, 255.0, 35.0, 69.0, 0.85, 0.85, -35.0, 0.3, 0.2, 50.0,
                ],
            ),
            ("lens-flare", vec![100.0, 0.5, 0.5, 0.0]),
            (
                "field-blur",
                vec![
                    2.0, 0.25, 0.5, 0.0, 0.75, 0.5, 40.0, // two pins
                    0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
                ],
            ),
            (
                "iris-blur",
                vec![
                    1.0, 0.85, 0.85, 0.05, 0.05, 0.0, 50.0, 0.3, 30.0, // one iris
                    0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
                ],
            ),
            (
                "tilt-shift",
                vec![
                    1.0, 0.5, 0.85, 15.0, 0.03, 0.03, 0.05, 0.05, 50.0, 0.0, 30.0, // one band
                    0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
                ],
            ),
            (
                "blur-gallery",
                vec![
                    0.0, // no Field pins
                    1.0, 0.85, 0.85, 0.05, 0.05, 0.0, 50.0, 0.3, 30.0, // one iris
                    1.0, 0.5, 0.85, 15.0, 0.03, 0.03, 0.05, 0.05, 50.0, 0.0, 30.0, // one band
                    0.0,  // no paths
                    0.0,  // no spins
                    50.0, 0.0, 1.0, 0.0, 0.0, 4.0, // motion
                    1.0, // flash duration
                    0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
                ],
            ),
            (
                "path-blur",
                vec![
                    1.0, 2.0, 0.8, 0.85, 0.95, 0.7, 40.0, 10.0, // one path
                    100.0, 0.0, 1.0, 0.0, 0.0, 4.0, // motion
                    0.0, 0.0, 0.0, 0.0, 0.0, // bokeh off
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, // noise off
                ],
            ),
            ("shear", {
                let mut shear = vec![0.0f64; SHEAR_SLOTS];
                shear[0] = 2.0;
                shear[1] = -1.0;
                shear[2] = -0.5;
                shear[3] = 1.0;
                shear[4] = 0.5;
                shear
            }),
        ] {
            assert!(filter_preview_needs_whole_layer(kind), "{kind}");

            // A cropped section preview does not match the commit...
            let mut cropped = big_state();
            apply_filter_active_region(&mut cropped, kind, &params, false, Some(viewport))
                .expect("section preview");

            // ...so the bridge escalates the kind to a whole-layer preview.
            let section = (!filter_preview_needs_whole_layer(kind)).then_some(viewport);
            let mut preview = big_state();
            apply_filter_active_region(&mut preview, kind, &params, false, section)
                .expect("whole-layer preview");

            let mut commit = big_state();
            apply_filter_active_region(&mut commit, kind, &params, true, None).expect("commit");

            let (preview_px, commit_px, cropped_px) = (red(&preview), red(&commit), red(&cropped));
            let w = 256usize;
            let mut cropped_differs = false;
            for y in viewport.top..viewport.bottom {
                for x in viewport.left..viewport.right {
                    let i = y as usize * w + x as usize;
                    assert_eq!(
                        preview_px[i], commit_px[i],
                        "{kind} whole-layer preview != commit at ({x},{y})"
                    );
                    if cropped_px[i] != commit_px[i] {
                        cropped_differs = true;
                    }
                }
            }
            assert!(
                cropped_differs,
                "{kind}: a cropped preview unexpectedly matched the commit"
            );
        }
        // A spin's apron is the widest (its reach is a fraction of the
        // buffer), so on this picture a crop is the whole layer and cannot
        // show the mismatch; the kind still escalates.
        assert!(filter_preview_needs_whole_layer("spin-blur"));
    }

    fn mask_coverage(rust: &PictureViewRust) -> Vec<u8> {
        rust.session.doc.as_ref().unwrap().layers[0]
            .mask
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .to_vec()
    }

    fn step_mask_state() -> PictureViewRust {
        let mut rust = state();
        let layer = &mut rust.session.doc.as_mut().unwrap().layers[0];
        layer.mask = Some(pictura_core::LayerMask {
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 16,
                right: 16,
            },
            default_color: 0,
            disabled: false,
            flags: 0,
            data: Some(
                (0..16 * 16)
                    .map(|i| if i % 16 < 8 { 0u8 } else { 255 })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            extra: Vec::new(),
            ..Default::default()
        });
        rust.mask_edit_target = Some("0".to_string());
        rust
    }

    #[test]
    fn a_filter_previews_and_commits_on_the_mask() {
        let mut rust = step_mask_state();
        let original = mask_coverage(&rust);
        let layer_red = red(&rust);

        let preview =
            apply_filter_active(&mut rust, "gaussian-blur", &[2.0], false).expect("mask preview");
        assert!(
            preview.is_some(),
            "the mask rectangle is the refresh region"
        );
        assert!(
            rust.session.live.mask_filter_preview().is_some(),
            "a baseline is kept"
        );
        let softened = mask_coverage(&rust);
        assert_ne!(softened, original, "the blur changed the edge");
        assert!(
            softened[7] > 0 && softened[7] < 255,
            "the black/white edge is softened"
        );
        assert_eq!(red(&rust), layer_red, "the layer's pixels are untouched");

        cancel_filter_preview(&mut rust).expect("cancel");
        assert_eq!(
            mask_coverage(&rust),
            original,
            "cancel restores bit-identically"
        );
        assert!(rust.session.live.mask_filter_preview().is_none());

        apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).expect("mask commit");
        assert!(rust.session.live.mask_filter_preview().is_none());
        assert_ne!(mask_coverage(&rust), original, "the commit keeps the blur");
        assert_eq!(red(&rust), layer_red, "the layer's pixels are untouched");
    }

    #[test]
    fn a_mask_filter_needs_the_target() {
        let mut rust = step_mask_state();
        rust.mask_edit_target = None;
        let original = mask_coverage(&rust);
        apply_filter_active(&mut rust, "gaussian-blur", &[2.0], true).expect("layer filter");
        assert_eq!(mask_coverage(&rust), original, "the mask is untouched");
    }
}
