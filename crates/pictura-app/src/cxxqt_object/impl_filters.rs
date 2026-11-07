mod adjustment;

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
use pictura_core::layer_pixel_locked;

impl qobject::PictureView {
    pub fn apply_filter(mut self: Pin<&mut Self>, kind: &QString) -> bool {
        if self.rust().modal_transform_blocks_edit() {
            return false;
        }
        let Some(filter) = filter_from_kind(&kind.to_string()) else {
            return false;
        };
        let routed = super::filter_tools::smart_apply::route_commit(
            self.as_mut(),
            &kind.to_string(),
            &[],
            "Filter",
        );
        if let Some(applied) = routed {
            return applied;
        }
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
                // Creation adds a row, so the panel must refresh even when
                // the recomposite is limited to the new mask's region.
                Some(rect) => Edit::regions(vec![rect], true),
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
        if self.rust().modal_transform_blocks_edit() {
            return false;
        }
        let region = {
            let mut rust = self.as_mut().rust_mut();
            apply_mask_op(&mut rust, path, &ActiveOp::Filter(filter), true, None)
        };
        let Some(region) = region else {
            return false;
        };
        self.as_mut().rust_mut().last_filter = Some((kind.to_string(), Vec::new()));
        let outcome = match region {
            Some(rect) => Edit::regions(vec![rect], false),
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
    if !commit {
        if let Some(region) = super::filter_tools::smart_apply::preview_kind(rust, kind, params) {
            return region;
        }
    }
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
    /// Image > Apply Image: its kernel needs document positions.
    ApplyImage(pictura_render::ApplyImageParams),
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
    if rust.modal_transform_blocks_edit() {
        rust.filter_error = Some("a transform is pending".to_string());
        return None;
    }
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
    let native = match (op, rust.session.doc.as_ref()) {
        (ActiveOp::ApplyImage(_), Some(doc)) => {
            match pictura_render::apply_image_native(doc, &path) {
                Ok(native) => native,
                Err(error) => {
                    rust.filter_error = Some(error.to_string());
                    return None;
                }
            }
        }
        _ => false,
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
    let cmyk = rust
        .session
        .doc
        .as_ref()
        .and_then(pictura_render::CmykMix::for_document);
    let Some(doc) = rust.session.doc.as_mut() else {
        rust.filter_error = Some("there is no document".to_string());
        return None;
    };
    let (doc_w, doc_h) = (doc.width, doc.height);
    let mode = pictura_render::document_color_mode(doc).into();
    let Some(layer) = pictura_render::resolve_path_mut(doc, &path) else {
        rust.filter_error = Some("the active layer no longer exists".to_string());
        return None;
    };
    // Re-filter from the pre-preview pixels, never from the previous preview.
    // A preview may have grown the layer, so put its rect back too.
    let restored = existing.map(|preview| {
        let restored = preview.baseline.restore(layer);
        pictura_core::layer_bounds::shrink_layer_to(layer, preview.rect);
        restored
    });
    if layer.is_group || layer.adjustment.is_some() {
        rust.filter_error = Some("the active layer cannot take a filter".to_string());
        return None;
    }
    let mut lrect = layer.rect;
    let filters = match op {
        ActiveOp::Filter(filter) => std::slice::from_ref(filter),
        ActiveOp::Filters(filters) => filters.as_slice(),
        ActiveOp::Adjustment(_) | ActiveOp::ApplyImage(_) => &[],
    };
    // Expand the visible rect by the support of every filter still to run, so
    // the viewport is exact after the whole stack; the renderer clamps it to
    // the layer. A point operation needs no apron.
    let mut apron: i32 = filters.iter().map(|filter| filter.preview_apron()).sum();
    // A filter may spread past the layer's rect: grow it by the stack's apron
    // first, so the edge is not clipped (#1208). A preview remembers the rect
    // it began on, and cancelling it shrinks the layer back.
    let preview_rect = lrect;
    if apron > 0 && !lrect.is_empty() {
        // Grow by the apron inside the canvas; the union keeps a layer that
        // already extends past the canvas a superset.
        let canvas = pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: doc_h as i32,
            right: doc_w as i32,
        };
        let grown = lrect.union(lrect.grow(apron).intersect(canvas).unwrap_or(lrect));
        match pictura_core::layer_bounds::grow_layer_to(layer, grown, "a filter") {
            Ok(true) => lrect = layer.rect,
            Ok(false) => {}
            Err(error) => {
                rust.filter_error = Some(error.to_string());
                return None;
            }
        }
    }
    let section = preview_region
        .filter(|_| !commit)
        .map(|visible| clamp_to(visible.grow(apron), lrect));
    // The composite changes where this call writes and where the last preview
    // was put back; a layer effect can spill anywhere.
    let region_out = (!layer_has_effects(layer)).then(|| {
        let written = section.unwrap_or(lrect);
        match restored {
            Some(r) if written.width() > 0 && written.height() > 0 => r.union(written),
            Some(r) => r,
            None => written,
        }
    });
    // A section baseline holds 8-bit channels only, so a store edit keeps the
    // whole layer, which shares its planes until they are written.
    let baseline = (!commit).then(|| PreviewBaseline::save(layer, section.filter(|_| !native)));
    let applied = match op {
        ActiveOp::Adjustment(adjustment) => pictura_render::apply_adjustment_region_in_mode(
            layer,
            adjustment,
            mask.as_ref(),
            section.unwrap_or(lrect),
            cmyk.as_ref(),
            mode,
        ),
        ActiveOp::ApplyImage(params) => pictura_render::apply_image_region(
            layer,
            params,
            mask.as_ref(),
            section.unwrap_or(lrect),
            pictura_render::ApplyImageTarget {
                origin: (0, 0),
                gray: false,
                native,
            },
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
                        visible.grow(apron),
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
            rect: preview_rect,
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
    // A Smart Object takes Smart Filters only (`filter_tools::smart_apply`).
    if layer.smart_object.is_some() {
        rust.filter_error = Some("the smart object is not directly editable".to_string());
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
    let mask_origin = mask.rect;
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
            ActiveOp::ApplyImage(params) => pictura_render::apply_image_region(
                layer,
                params,
                selection.as_ref(),
                rect,
                pictura_render::ApplyImageTarget {
                    origin: (mask_origin.left, mask_origin.top),
                    gray: true,
                    native: false,
                },
            ),
        }
    };
    if let Err(error) = result {
        // A failed preview restores the previous mask coverage; a failed commit
        // must not leave a previous successful mask preview live.
        let _ = cancel_filter_preview(rust);
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
            pictura_core::layer_bounds::shrink_layer_to(layer, preview.rect);
            Some((!layer_has_effects(layer)).then_some(restored))
        }
    }
}

#[cfg(test)]
mod tests;
