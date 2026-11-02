use super::*;

use super::super::filter_map::filter_from_kind_params;
use super::super::helpers::{active_layer_visible, active_mask_target, active_pixel_layer};
use super::super::helpers_composite::{
    buffer_to_image, layer_buffer, patch_buffer_region, premultiplied_display_image,
    selection_to_mask,
};
use super::super::impl_filters::{
    apply_filter_active, apply_filter_active_region, apply_op_active_region, cancel_filter_preview,
    ActiveOp,
};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QImage, QList, QString, QStringList};
use pictura_core::{layer_pixel_locked, Document, Layer, PixelBuffer};

pub(super) fn apply_filter_params(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
) -> bool {
    let kind_s = kind.to_string();
    if !super::super::filter_map::is_filter_command(&kind_s) {
        return false;
    }
    if let Some(reason) = depth_refusal(&kind_s, view.rust().session.doc.as_ref()) {
        eprintln!("pictura: filter '{kind_s}' refused: {reason}");
        view.as_mut().rust_mut().filter_error = Some(reason);
        return false;
    }
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    // On a Smart Object the filter becomes a Smart Filter.
    if let Some(applied) = smart_apply::route_commit(view.as_mut(), &kind_s, &params_v, "Filter") {
        return applied;
    }
    let params_v = super::super::filter_map::commit_params(&kind_s, &params_v);
    let params_v = lens_blur_depths::commit_slots(&kind_s, view.rust(), &params_v);
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_filter_active(&mut rust, &kind_s, &params_v, true)
    };
    let Some(region) = region else {
        let reason = view.rust().filter_error.clone().unwrap_or_default();
        eprintln!("pictura: filter '{kind_s}' refused: {reason}");
        return false;
    };
    let outcome = match region {
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    view.as_mut().edit("Filter", |_| outcome);
    true
}

pub(super) fn filter_preview(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
) -> bool {
    let kind_s = kind.to_string();
    if !super::super::filter_map::is_filter_command(&kind_s) {
        return false;
    }
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_filter_active(&mut rust, &kind_s, &params_v, false)
    };
    let Some(region) = region else {
        {
            let mut rust = view.as_mut().rust_mut();
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    true
}

/// Filters that resolve pixel position or a global statistic against the whole
/// layer, so a cropped section preview would not match the commit: Diffuse
/// hashes the pixel's absolute coordinates, Lighting resolves its light span
/// against the crop, Lens Flare places and sizes itself as a fraction of the
/// buffer, Shear shifts each row by
/// a fraction of the buffer's half-width, Displace stretches or tiles its
/// map from the buffer's corner, Lens Blur's depth map and Liquify's mesh
/// align to the layer's rectangle, Picture Frame sizes and insets itself as a
/// fraction of the buffer, Radial Blur normalizes its centre to the buffer,
/// Clouds, Difference Clouds and Fibers generate a field from the buffer
/// dimensions, and the Blur Gallery (Field Blur, Iris Blur, Tilt-Shift, Path
/// Blur, and Spin Blur) places its pins, paths, and spins as fractions of the
/// buffer.
pub(crate) fn filter_preview_needs_whole_layer(kind: &str) -> bool {
    super::super::filter_map::descriptor::descriptor(kind)
        .is_some_and(|descriptor| descriptor.whole_layer)
}

pub(super) fn filter_preview_section(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let kind_s = kind.to_string();
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    let visible = pictura_core::PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let section = (!filter_preview_needs_whole_layer(&kind_s)).then_some(visible);
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_filter_active_region(&mut rust, &kind_s, &params_v, false, section)
    };
    let Some(region) = region else {
        {
            let mut rust = view.as_mut().rust_mut();
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    true
}

pub(super) fn filter_preview_cancel(mut view: Pin<&mut PictureView>) -> bool {
    let region = {
        let mut rust = view.as_mut().rust_mut();
        cancel_filter_preview(&mut rust)
    };
    let Some(region) = region else {
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    true
}

pub(super) fn filter_has_last(view: Pin<&mut PictureView>) -> bool {
    view.rust().last_filter.is_some()
}

pub(super) fn filter_last_kind(view: Pin<&mut PictureView>) -> QString {
    view.rust()
        .last_filter
        .as_ref()
        .map(|(kind, _)| QString::from(kind.as_str()))
        .unwrap_or_default()
}

pub(super) fn filter_last_params(view: Pin<&mut PictureView>) -> QList<f64> {
    view.rust()
        .last_filter
        .as_ref()
        .map(|(_, params)| params.iter().copied().collect())
        .unwrap_or_default()
}

pub(super) fn filter_target_ready(view: Pin<&mut PictureView>) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    // A mask target routes filters and adjustments to coverage instead of the
    // layer, so a fill/adjustment/group layer is a valid target while its mask
    // is active.
    if active_mask_target(rust).is_some() {
        return true;
    }
    let active = rust.layer_target.single();
    active_layer_visible(doc, active)
        && active_pixel_layer(doc, active).is_some_and(|layer| !layer_pixel_locked(layer))
}

pub(super) fn filter_target_smart(view: &PictureView) -> bool {
    smart_apply::on_smart_object(view.rust())
}

pub(super) fn filter_target_empty(view: Pin<&mut PictureView>) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    active_pixel_layer(doc, rust.layer_target.single())
        .is_some_and(|layer| selected_area_empty(layer, rust.session.selection.as_ref()))
}

/// Whether `layer` has no non-transparent pixel inside `selection` (the
/// whole layer when there is none). Layer masks do not count: a filter still
/// writes the pixels a mask hides.
pub(super) fn selected_area_empty(
    layer: &Layer,
    selection: Option<&pictura_select::Selection>,
) -> bool {
    let r = layer.rect;
    let (w, h) = (r.width().max(0) as usize, r.height().max(0) as usize);
    if w == 0 || h == 0 || !layer.channels.iter().any(|c| c.id == 0) {
        return true;
    }
    let alpha = layer.channels.iter().find(|c| c.id == -1);
    let opaque = |x: usize, y: usize| alpha.is_none_or(|a| a.data.get(y * w + x) != Some(&0));
    let selected = |x: usize, y: usize| {
        selection.is_none_or(|s| {
            let (dx, dy) = (r.left + x as i32, r.top + y as i32);
            dx >= 0
                && dy >= 0
                && (dx as u32) < s.width
                && (dy as u32) < s.height
                && s.data[dy as usize * s.width as usize + dx as usize] > 0
        })
    };
    !(0..h).any(|y| (0..w).any(|x| opaque(x, y) && selected(x, y)))
}

pub(super) fn filter_kind_supported(kind: &QString) -> bool {
    super::super::filter_map::is_filter_command(&kind.to_string())
}

pub(super) fn filter_descriptors() -> Vec<ffi::FilterDescriptorRow> {
    super::super::filter_map::descriptor::all()
        .map(|descriptor| {
            let params = descriptor
                .params
                .iter()
                .map(|param| {
                    let mut choices = QStringList::default();
                    for choice in param.choices {
                        choices.append(QString::from(*choice));
                    }
                    let mut initial = QList::<f64>::default();
                    for value in param.initial {
                        initial.append(*value);
                    }
                    ffi::FilterParamRow {
                        control: param.control as i32,
                        label: QString::from(param.label),
                        minimum: param.minimum,
                        maximum: param.maximum,
                        value: param.value,
                        slot_count: param.slots() as i32,
                        decimals: param.decimals,
                        suffix: QString::from(param.suffix),
                        choices,
                        initial,
                        measures_pixels: param.pixel_distance,
                    }
                })
                .collect();
            let mut path = QStringList::default();
            for part in descriptor.path {
                path.append(QString::from(*part));
            }
            ffi::FilterDescriptorRow {
                kind: QString::from(descriptor.id),
                path,
                menu: descriptor.menu,
                label: QString::from(descriptor.label),
                params,
                whole_layer: descriptor.whole_layer,
                dialog: descriptor.dialog as i32,
                map_after_ok: descriptor.map_after_ok,
                preview_pane: descriptor.preview_pane,
                stacked: descriptor.stacked,
                preview_below: descriptor.preview_below,
                live_preview: descriptor.live_preview,
                gallery_rank: descriptor.gallery.map_or(-1, i32::from),
            }
        })
        .collect()
}

pub(super) fn filter_param_arity(kind: &QString) -> i32 {
    super::super::filter_map::filter_param_arity(&kind.to_string())
        .map(|n| n as i32)
        .unwrap_or(-1)
}

/// Read the active pixel layer's in-tree watermark. The reader inspects the
/// layer's own buffer, so embed and read share the layer's origin.
pub(super) fn read_watermark(view: &PictureView) -> ffi::WatermarkRow {
    let rust = view.rust();
    let payload = rust.session.doc.as_ref().and_then(|doc| {
        let layer = active_pixel_layer(doc, rust.layer_target.single())?;
        let buffer = layer_buffer(layer)?;
        pictura_filters::digimarc::read(&buffer)
    });
    match payload {
        Some(payload) => ffi::WatermarkRow {
            found: true,
            kind: QString::from(match payload.kind {
                pictura_filters::WatermarkKind::ImageId => "Image ID",
                pictura_filters::WatermarkKind::TransactionId => "Transaction ID",
            }),
            creator_id: payload.creator_id as i32,
            id: payload.id as i32,
        },
        None => ffi::WatermarkRow {
            found: false,
            kind: QString::default(),
            creator_id: 0,
            id: 0,
        },
    }
}

pub(super) fn filter_picture_frame_uses_lines(frame: i32) -> bool {
    pictura_filters::uses_lines(frame.max(1) as u32)
}

pub(super) fn filter_last_error(view: Pin<&mut PictureView>) -> QString {
    view.rust()
        .filter_error
        .as_deref()
        .map(QString::from)
        .unwrap_or_default()
}

// The Filter Gallery: preview and commit a stack of filters on the active
// layer through the same snapshot/preview/commit core as a single filter, and
// render one filter onto a small image for a gallery thumbnail.

pub(super) fn run_stack(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    commit: bool,
    section: Option<pictura_core::PsdRect>,
) -> bool {
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    let kind_list: Vec<String> = QList::<QString>::from(kinds)
        .iter()
        .map(|k| k.to_string())
        .collect();
    if let Some(applied) = smart_apply::route_stack(
        view.as_mut(),
        &kind_list,
        &params_v,
        commit,
        "Filter Gallery",
    ) {
        return applied;
    }
    let Some(filters) = resolve_stack(kinds, &params_v) else {
        view.as_mut().rust_mut().filter_error =
            Some("the gallery stack has an unknown filter or wrong parameters".to_string());
        return false;
    };
    if commit && filters.is_empty() {
        return false;
    }
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_op_active_region(
            &mut rust,
            &ActiveOp::Filters(filters),
            commit,
            section,
            false,
        )
    };
    let Some(region) = region else {
        return false;
    };
    let outcome = match region {
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    if commit {
        view.as_mut().edit("Filter Gallery", |_| outcome);
    } else {
        view.as_mut().preview(|_| outcome);
    }
    true
}

pub(super) fn filter_stack_preview(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let visible = pictura_core::PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let positional = QList::<QString>::from(kinds)
        .iter()
        .any(|k| filter_preview_needs_whole_layer(&k.to_string()));
    let shown = run_stack(
        view.as_mut(),
        kinds,
        params,
        false,
        (!positional).then_some(visible),
    );
    if !shown {
        {
            let mut rust = view.as_mut().rust_mut();
            let _ = cancel_filter_preview(&mut rust);
        }
        view.as_mut().recomposite();
    }
    shown
}

pub(super) fn apply_filter_stack(
    view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
) -> bool {
    run_stack(view, kinds, params, true, None)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn filter_gallery_preview(
    view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scale: f64,
) -> QImage {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    let region = pictura_core::PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let selection = rust
        .session
        .selection
        .as_ref()
        .map(|s| selection_to_mask(s, doc));
    let Some(proxy) = pictura_render::FilterProxy::new(doc, selection.as_ref(), region, scale)
    else {
        return QImage::default();
    };
    let scaled = scaled_stack_params(kinds, params, scale);
    let filtered = match (resolve_stack(kinds, &scaled), rust.layer_target.single()) {
        (Some(filters), Some(path)) => proxy.render(path, &filters, rust.gpu_compute).ok(),
        _ => None,
    };
    filtered
        .or_else(|| proxy.render("", &[], false).ok())
        .map_or_else(QImage::default, |buffer| buffer_to_image(&buffer))
}

pub(super) fn filter_thumbnail(
    rgba: &QByteArray,
    width: i32,
    height: i32,
    kind: &QString,
    params: &QList<f64>,
    scale: f64,
) -> QImage {
    if !super::super::filter_map::is_filter_command(&kind.to_string()) {
        return QImage::default();
    }
    let params: Vec<f64> = params.into_iter().copied().collect();
    let params =
        super::super::filter_map::descriptor::scale_pixel_params(&kind.to_string(), &params, scale);
    let (Ok(w), Ok(h)) = (u32::try_from(width), u32::try_from(height)) else {
        return QImage::default();
    };
    let Some(filter) = filter_from_kind_params(&kind.to_string(), &params) else {
        return QImage::default();
    };
    let Some(mut buffer) = rgba_to_buffer(rgba.as_slice(), w, h) else {
        return QImage::default();
    };
    if pictura_filters::apply(&filter, &mut buffer).is_err() {
        return QImage::default();
    }
    buffer_to_image(&buffer)
}

/// Packed RGBA bytes as a planar four-channel buffer.
pub(super) fn rgba_to_buffer(rgba: &[u8], width: u32, height: u32) -> Option<PixelBuffer> {
    let n = width as usize * height as usize;
    if n == 0 || rgba.len() != n * 4 {
        return None;
    }
    let mut buffer = PixelBuffer::new(width, height, 4);
    for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
        for (c, &v) in px.iter().enumerate() {
            buffer.data[c * n + i] = v;
        }
    }
    Some(buffer)
}

/// The document's composite with the layer at `path` hidden, patched over the
/// cached `doc.composite` so opening a dialog does not pay for a full
/// recomposite. `None` when the path does not resolve.
pub(super) fn hidden_layer_buffer(
    doc: &mut Document,
    path: &str,
    gpu_compute: bool,
) -> Option<PixelBuffer> {
    let (rect, was_visible) = {
        let layer = pictura_render::resolve_path(doc, path)?;
        (layer.rect, layer.visible)
    };
    let cached_ok = doc.composite.width == doc.width
        && doc.composite.height == doc.height
        && !doc.composite.data.is_empty()
        && rect.width() > 0
        && rect.height() > 0;
    pictura_render::resolve_path_mut(doc, path)?.visible = false;
    let base = if cached_ok {
        let (region, _) = pictura_render::composite_region_active(doc, rect, gpu_compute);
        let mut base = doc.composite.clone();
        patch_buffer_region(&mut base, &region, rect.left, rect.top);
        base
    } else {
        pictura_render::composite_rgba(doc)
    };
    if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
        layer.visible = was_visible;
    }
    Some(base)
}

pub(super) fn filter_proxy_composite(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    max_edge: i32,
) -> QImage {
    let Ok(max_edge) = u32::try_from(max_edge) else {
        return QImage::default();
    };
    let params: Vec<f64> = params.into_iter().copied().collect();
    let mut rust = view.as_mut().rust_mut();
    pane_buffer(&mut rust, &kind.to_string(), &params, max_edge)
        .map_or_else(QImage::default, |buffer| {
            premultiplied_display_image(&buffer)
        })
}

pub(super) fn filter_canvas_proxy(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scale: f64,
) -> QImage {
    let params: Vec<f64> = params.into_iter().copied().collect();
    let Some(filter) = filter_from_kind_params(&kind.to_string(), &params) else {
        return QImage::default();
    };
    let region = pictura_core::PsdRect {
        top: y,
        left: x,
        bottom: y + h.max(0),
        right: x + w.max(0),
    };
    let mut rust = view.as_mut().rust_mut();
    canvas_proxy_buffer(&mut rust, &filter, region, scale).map_or_else(QImage::default, |buffer| {
        premultiplied_display_image(&buffer)
    })
}

pub(super) fn filter_canvas_proxy_release(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().canvas_proxy = None;
}

pub(super) fn filter_kind_positional(kind: &QString) -> bool {
    filter_preview_needs_whole_layer(&kind.to_string())
}

pub(super) fn filter_displace_map_load(path: &QString) -> i32 {
    match displace_maps::load(&path.to_string()) {
        Ok(id) => id as i32,
        Err(reason) => {
            eprintln!("pictura: displacement map refused: {reason}");
            0
        }
    }
}

pub(super) fn filter_lens_blur_depth(view: &PictureView, source: i32) -> i32 {
    let rust = view.rust();
    match lens_blur_depths::materialize(rust, source) {
        Some(map) => lens_blur_depths::register(map) as i32,
        None => 0,
    }
}

#[cfg(test)]
mod tests;
