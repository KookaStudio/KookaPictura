//! The Filter-menu bridges: apply a filter with runtime parameters, render a
//! non-committing preview, keep the last filter, and the Filter Gallery's stack
//! preview/commit and thumbnails. Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` declaration list does
//! not grow). Ported from perfecto25/photorust's filter commit path.
//!
//! Source: https://github.com/perfecto25/photorust
//!
//! [`PictureView`]: super::qobject::PictureView

use super::filter_map::filter_from_kind_params;
use super::helpers::{active_layer_visible, active_pixel_layer};
use super::helpers_composite::{
    buffer_to_image, patch_buffer_region, premultiplied_display_image, selection_to_mask,
};
use super::impl_filters::{
    apply_filter_active, apply_filter_active_region, apply_op_active_region, cancel_filter_preview,
    ActiveOp,
};
use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
pub(super) use canvas_proxy::canvas_proxy_buffer;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QImage, QList, QString, QStringList};
use pictura_core::{layer_pixel_locked, Document, Layer, PixelBuffer};

mod canvas_proxy;
pub(super) mod displace_maps;
pub(super) mod jobs;
pub(super) mod lens_blur_depths;
pub(super) mod liquify;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Commit a filter `kind` with an ordered `params` slot list. When a
        /// preview is open the layer already holds the result, so only one
        /// `"Filter"` history state is recorded; otherwise the filter is applied
        /// fresh. Stores the filter for Last Filter. Returns false on an unknown
        /// kind, a bad arity, or no editable pixel layer.
        fn apply_filter_params(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
        ) -> bool;

        /// Preview a filter `kind` with `params` on the canvas without recording
        /// history. Each call re-filters the pre-preview pixels, so parameter
        /// changes do not compound. Returns false when the active layer is not
        /// an unlocked normal pixel layer or the parameters are invalid.
        fn filter_preview(view: Pin<&mut PictureView>, kind: &QString, params: &QList_f64) -> bool;

        /// Preview `kind` restricted to the document rect `(x, y, w, h)` (the
        /// visible viewport), expanded by the filter's support. Cheaper than a
        /// full-layer preview while exact across the visible area. Positional
        /// kinds (see [`filter_preview_needs_whole_layer`]) ignore the rect and
        /// preview the whole layer, because a crop changes their result.
        fn filter_preview_section(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Discard an open preview, restoring the pre-preview pixels
        /// bit-identically. Returns false when no preview is open.
        fn filter_preview_cancel(view: Pin<&mut PictureView>) -> bool;

        /// Whether a filter has been committed this session.
        fn filter_has_last(view: Pin<&mut PictureView>) -> bool;

        /// The last committed filter's kind, or an empty string.
        fn filter_last_kind(view: Pin<&mut PictureView>) -> QString;

        /// The last committed filter's parameter slot values, or an empty list.
        fn filter_last_params(view: Pin<&mut PictureView>) -> QList_f64;

        /// Whether the active layer can take a destructive filter (an unlocked
        /// normal pixel layer in an open document).
        fn filter_target_ready(view: Pin<&mut PictureView>) -> bool;

        /// Whether the active pixel layer has nothing to filter: no opaque
        /// pixel at all, or none inside the selection. False when there is no
        /// pixel layer (that is [`filter_target_ready`]'s refusal).
        fn filter_target_empty(view: Pin<&mut PictureView>) -> bool;

        /// Whether `kind` is a filter the parameterised mapper understands.
        fn filter_kind_supported(kind: &QString) -> bool;

        /// The number of parameter slots `kind` expects, or -1 for an unknown
        /// kind.
        fn filter_param_arity(kind: &QString) -> i32;

        /// Whether a Picture Frame frame number draws lines that the Number of
        /// Lines control sets the density of; CS6 greys the control out for the
        /// rest.
        fn filter_picture_frame_uses_lines(frame: i32) -> bool;

        /// The reason the most recent apply/preview was refused, or an empty
        /// string when it succeeded.
        fn filter_last_error(view: Pin<&mut PictureView>) -> QString;

        /// Preview the gallery stack — `kinds` in order, their parameter slots
        /// concatenated in `params` — on the canvas, restricted to the document
        /// rect `(x, y, w, h)`. Re-filters the pre-preview pixels each call. An
        /// empty stack shows the layer unfiltered. Returns false on an unknown
        /// kind, a bad arity, or no editable pixel layer.
        fn filter_stack_preview(
            view: Pin<&mut PictureView>,
            kinds: &QStringList,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Commit the gallery stack to the active layer as one "Filter
        /// Gallery" history state. Returns false for an empty stack or on the
        /// same refusals as [`filter_stack_preview`].
        fn apply_filter_stack(
            view: Pin<&mut PictureView>,
            kinds: &QStringList,
            params: &QList_f64,
        ) -> bool;

        /// The gallery stack (as for [`filter_stack_preview`]) rendered on a
        /// reduced copy of the document rect `(x, y, w, h)`, `scale` proxy
        /// pixels per document pixel (at most 1), as a straight-alpha image.
        /// Neither the document nor the canvas changes. A stack that cannot
        /// apply shows the region unfiltered; an empty region yields a null
        /// image.
        #[allow(clippy::too_many_arguments)]
        fn filter_gallery_preview(
            view: Pin<&mut PictureView>,
            kinds: &QStringList,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            scale: f64,
        ) -> QImage;

        /// `kind` with `params` applied to a `width`×`height` straight-alpha
        /// RGBA8888 image, for a gallery thumbnail. A null image when the
        /// filter is refused.
        fn filter_thumbnail(
            rgba: &QByteArray,
            width: i32,
            height: i32,
            kind: &QString,
            params: &QList_f64,
        ) -> QImage;

        /// A proxy of the document with the active layer hidden, scaled to
        /// `max_edge` on the long side, for a whole-layer filter dialog's
        /// bottom pane. The pane composites [`filter_proxy_layer`] over it, so
        /// the other layers are never filtered with the active one.
        fn filter_proxy_base(view: Pin<&mut PictureView>, max_edge: i32) -> QImage;

        /// The active layer's pixels filtered with `kind`/`params`, at the
        /// same proxy size as [`filter_proxy_base`] and positioned in its
        /// document frame. The filter runs over the layer's own bounds, as the
        /// canvas preview and the commit do, and the layer's mask, opacity and
        /// fill are baked into the alpha. The unfiltered layer is returned when
        /// the filter is refused, so the pane still shows the picture.
        fn filter_proxy_layer(
            view: &PictureView,
            kind: &QString,
            params: &QList_f64,
            max_edge: i32,
        ) -> QImage;

        /// The canvas preview of `kind`/`params` below 100% zoom: the document
        /// rect `(x, y, w, h)` reduced to `scale` (at most 1), composited with
        /// the active layer filtered, as a premultiplied display image. The
        /// caller scales pixel-sized slots by `scale`. The reduced copy is
        /// built from the pre-preview pixels and kept while the region, scale
        /// and document are unchanged, so a parameter change filters only the
        /// copy. Neither the document nor the canvas changes. Null when the
        /// target has no proxy (a layer mask) or the filter is refused, so the
        /// caller previews exactly instead.
        #[allow(clippy::too_many_arguments)]
        fn filter_canvas_proxy(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            scale: f64,
        ) -> QImage;

        /// Drop the reduced copy [`filter_canvas_proxy`] keeps.
        fn filter_canvas_proxy_release(view: Pin<&mut PictureView>);

        /// Whether `kind` resolves position or a statistic against the whole
        /// layer, so a section of it cannot be previewed on its own.
        fn filter_kind_positional(kind: &QString) -> bool;

        /// Read a flattened PSD at `path` as a Displace map and keep it for
        /// the `displace` kind's map slot. Returns the map's id, or 0 when the
        /// file is unreadable or in Bitmap mode.
        fn filter_displace_map_load(path: &QString) -> i32;

        /// Build and keep the active layer's depth map for the `lens-blur`
        /// Source slot (0 None, 1 Transparency, 2 Layer Mask, 3 Selection) and
        /// return its id, or 0 when the source is unavailable.
        fn filter_lens_blur_depth(view: &PictureView, source: i32) -> i32;
    }
}

fn apply_filter_params(
    mut view: Pin<&mut PictureView>,
    kind: &QString,
    params: &QList<f64>,
) -> bool {
    let kind_s = kind.to_string();
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    let params_v = super::filter_map::commit_params(&kind_s, &params_v);
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
        Some(rect) => Edit::Regions {
            rects: vec![rect],
            notify: false,
        },
        None => Edit::Full,
    };
    view.as_mut().edit("Filter", |_| outcome);
    true
}

fn filter_preview(mut view: Pin<&mut PictureView>, kind: &QString, params: &QList<f64>) -> bool {
    let kind_s = kind.to_string();
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    let region = {
        let mut rust = view.as_mut().rust_mut();
        apply_filter_active(&mut rust, &kind_s, &params_v, false)
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

/// Filters that resolve pixel position or a global statistic against the whole
/// layer, so a cropped section preview would not match the commit: Diffuse
/// hashes the pixel's absolute coordinates, Lighting resolves its light span
/// against the crop, HDR Toning computes a global pivot, Lens Flare places
/// and sizes itself as a fraction of the buffer, Shear shifts each row by
/// a fraction of the buffer's half-width, Displace stretches or tiles its
/// map from the buffer's corner, Lens Blur's depth map and Liquify's mesh
/// align to the layer's rectangle, Picture Frame sizes and insets itself as a
/// fraction of the buffer, and the Blur Gallery (Field Blur, Iris Blur,
/// Tilt-Shift, Path Blur, and Spin Blur) places its pins, paths, and spins as
/// fractions of the buffer.
pub(crate) fn filter_preview_needs_whole_layer(kind: &str) -> bool {
    matches!(
        kind,
        "diffuse"
            | "lighting-effects"
            | "hdr-toning"
            | "lens-flare"
            | "shear"
            | "displace"
            | "lens-blur"
            | "field-blur"
            | "iris-blur"
            | "tilt-shift"
            | "path-blur"
            | "spin-blur"
            | "blur-gallery"
            | "liquify"
            | "picture-frame"
    )
}

fn filter_preview_section(
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
        return false;
    };
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    true
}

fn filter_preview_cancel(mut view: Pin<&mut PictureView>) -> bool {
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

fn filter_has_last(view: Pin<&mut PictureView>) -> bool {
    view.rust().last_filter.is_some()
}

fn filter_last_kind(view: Pin<&mut PictureView>) -> QString {
    view.rust()
        .last_filter
        .as_ref()
        .map(|(kind, _)| QString::from(kind.as_str()))
        .unwrap_or_default()
}

fn filter_last_params(view: Pin<&mut PictureView>) -> QList<f64> {
    view.rust()
        .last_filter
        .as_ref()
        .map(|(_, params)| params.iter().copied().collect())
        .unwrap_or_default()
}

fn filter_target_ready(view: Pin<&mut PictureView>) -> bool {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let active = rust.layer_target.single();
    active_layer_visible(doc, active)
        && active_pixel_layer(doc, active).is_some_and(|layer| !layer_pixel_locked(layer))
}

fn filter_target_empty(view: Pin<&mut PictureView>) -> bool {
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
fn selected_area_empty(layer: &Layer, selection: Option<&pictura_select::Selection>) -> bool {
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

fn filter_kind_supported(kind: &QString) -> bool {
    super::filter_map::filter_param_arity(&kind.to_string()).is_some()
}

fn filter_param_arity(kind: &QString) -> i32 {
    super::filter_map::filter_param_arity(&kind.to_string())
        .map(|n| n as i32)
        .unwrap_or(-1)
}

fn filter_picture_frame_uses_lines(frame: i32) -> bool {
    pictura_filters::uses_lines(frame.max(1) as u32)
}

fn filter_last_error(view: Pin<&mut PictureView>) -> QString {
    view.rust()
        .filter_error
        .as_deref()
        .map(QString::from)
        .unwrap_or_default()
}

// The Filter Gallery: preview and commit a stack of filters on the active
// layer through the same snapshot/preview/commit core as a single filter, and
// render one filter onto a small image for a gallery thumbnail.

/// Resolve a stack: each kind takes the next `arity` slots of `params`.
fn resolve_stack(kinds: &QStringList, params: &QList<f64>) -> Option<Vec<pictura_filters::Filter>> {
    let params: Vec<f64> = params.into_iter().copied().collect();
    let mut at = 0;
    let mut filters = Vec::new();
    for kind in &QList::<QString>::from(kinds) {
        let kind = kind.to_string();
        let arity = super::filter_map::filter_param_arity(&kind)?;
        let slots = params.get(at..at + arity)?;
        filters.push(filter_from_kind_params(&kind, slots)?);
        at += arity;
    }
    (at == params.len()).then_some(filters)
}

fn run_stack(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    commit: bool,
    section: Option<pictura_core::PsdRect>,
) -> bool {
    let Some(filters) = resolve_stack(kinds, params) else {
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
        Some(rect) => Edit::Regions {
            rects: vec![rect],
            notify: false,
        },
        None => Edit::Full,
    };
    if commit {
        view.as_mut().edit("Filter Gallery", |_| outcome);
    } else {
        view.as_mut().preview(|_| outcome);
    }
    true
}

fn filter_stack_preview(
    view: Pin<&mut PictureView>,
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
    run_stack(view, kinds, params, false, (!positional).then_some(visible))
}

fn apply_filter_stack(
    view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
) -> bool {
    run_stack(view, kinds, params, true, None)
}

#[allow(clippy::too_many_arguments)]
fn filter_gallery_preview(
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
    let filtered = match (resolve_stack(kinds, params), rust.layer_target.single()) {
        (Some(filters), Some(path)) => proxy.render(path, &filters, rust.gpu_compute).ok(),
        _ => None,
    };
    filtered
        .or_else(|| proxy.render("", &[], false).ok())
        .map_or_else(QImage::default, |buffer| buffer_to_image(&buffer))
}

fn filter_thumbnail(
    rgba: &QByteArray,
    width: i32,
    height: i32,
    kind: &QString,
    params: &QList<f64>,
) -> QImage {
    let params: Vec<f64> = params.into_iter().copied().collect();
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
fn rgba_to_buffer(rgba: &[u8], width: u32, height: u32) -> Option<PixelBuffer> {
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

/// The active path resolved to a proxyable raster layer (not a group or an
/// adjustment), or `None` for an absent, malformed, or ineligible path.
fn active_proxy_layer<'a>(doc: &'a Document, active: Option<&str>) -> Option<&'a Layer> {
    let layer = pictura_render::resolve_path(doc, active?)?;
    (!layer.is_group && layer.adjustment.is_none()).then_some(layer)
}

/// Proxy frame dimensions: the document scaled to `max_edge` on the long side,
/// aspect kept, at least one pixel per axis.
fn proxy_dims(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let scale = f64::from(max_edge) / f64::from(width.max(height).max(1));
    let scaled = |v: u32| ((f64::from(v) * scale).round() as u32).max(1);
    (scaled(width), scaled(height))
}

/// The document's composite with the layer at `path` hidden, patched over the
/// cached `doc.composite` so opening a dialog does not pay for a full
/// recomposite. `None` when the path does not resolve.
fn hidden_layer_buffer(doc: &mut Document, path: &str, gpu_compute: bool) -> Option<PixelBuffer> {
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

/// The raster mask value at document pixel `(x, y)`: `255` when there is no
/// enabled mask, the mask's default colour outside its rect.
fn proxy_mask(layer: &Layer, x: i32, y: i32) -> u8 {
    let Some(mask) = layer.mask.as_ref() else {
        return 255;
    };
    if mask.disabled {
        return 255;
    }
    let Some(data) = mask.data.as_ref() else {
        return 255;
    };
    let mx = x - mask.rect.left;
    let my = y - mask.rect.top;
    let (mw, mh) = (mask.rect.width(), mask.rect.height());
    if mw <= 0 || mh <= 0 || mx < 0 || my < 0 || mx >= mw || my >= mh {
        return mask.default_color;
    }
    data.get(my as usize * mw as usize + mx as usize)
        .copied()
        .unwrap_or(mask.default_color)
}

/// The active layer sampled at proxy scale, as a straight-alpha RGBA buffer:
/// nearest-neighbour over the layer's own bounds, with its raster mask, opacity
/// and fill baked into the alpha. `None` for an empty or malformed layer.
///
/// ponytail: vector masks, blend modes and Blend If are not applied; the pane
/// is a preview, and the canvas preview next to it is exact.
fn proxy_layer_buffer(layer: &Layer, scale: f64) -> Option<PixelBuffer> {
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    let pw = ((f64::from(width) * scale).round() as u32).max(1);
    let ph = ((f64::from(height) * scale).round() as u32).max(1);
    let plane = (width * height) as usize;
    let channel = |id: i16| layer.channels.iter().find(|c| c.id == id).map(|c| &c.data);

    let c0 = channel(0)?;
    if c0.len() != plane {
        return None;
    }
    let (c1, c2) = if channel(1).is_none() && channel(2).is_none() {
        (c0, c0)
    } else {
        let (c1, c2) = (channel(1)?, channel(2)?);
        if c1.len() != plane || c2.len() != plane {
            return None;
        }
        (c1, c2)
    };
    let alpha = match channel(-1) {
        Some(a) if a.len() == plane => Some(a),
        Some(_) => return None,
        None => None,
    };
    let opacity = u32::from(layer.opacity) * u32::from(layer.fill) / 255;

    let mut out = PixelBuffer::new(pw, ph, 4);
    let out_plane = (pw * ph) as usize;
    for py in 0..ph {
        let sy = (py as u64 * u64::from(height) / u64::from(ph)).min(u64::from(height) - 1);
        for px in 0..pw {
            let sx = (px as u64 * u64::from(width) / u64::from(pw)).min(u64::from(width) - 1);
            let si = sy as usize * width as usize + sx as usize;
            let a = alpha.map_or(255, |a| a[si]);
            let mask = proxy_mask(
                layer,
                sx as i32 + layer.rect.left,
                sy as i32 + layer.rect.top,
            );
            let o = py as usize * pw as usize + px as usize;
            out.data[o] = c0[si];
            out.data[out_plane + o] = c1[si];
            out.data[2 * out_plane + o] = c2[si];
            out.data[3 * out_plane + o] =
                (((a as u32 * mask as u32 + 127) / 255) * opacity / 255).min(255) as u8;
        }
    }
    Some(out)
}

/// The filtered active layer positioned in a `max_edge`-bounded document-frame
/// proxy: the layer's own bounds are filtered — the same geometry the canvas
/// preview and the commit use — then blitted at the layer's scaled rect,
/// transparent elsewhere. The unfiltered layer is returned when the filter is
/// refused.
fn proxy_layer_frame(
    layer: &Layer,
    doc_width: u32,
    doc_height: u32,
    max_edge: u32,
    kind: &str,
    params: &[f64],
) -> QImage {
    proxy_layer_frame_buffer(layer, doc_width, doc_height, max_edge, kind, params)
        .map_or_else(QImage::default, |frame| buffer_to_image(&frame))
}

fn proxy_layer_frame_buffer(
    layer: &Layer,
    doc_width: u32,
    doc_height: u32,
    max_edge: u32,
    kind: &str,
    params: &[f64],
) -> Option<PixelBuffer> {
    let scale = f64::from(max_edge) / f64::from(doc_width.max(doc_height).max(1));
    let mut buffer = proxy_layer_buffer(layer, scale)?;
    if let Some(filter) = filter_from_kind_params(kind, params) {
        let _ = pictura_filters::apply(&filter, &mut buffer);
    }

    let (frame_w, frame_h) = proxy_dims(doc_width, doc_height, max_edge);
    let x = (f64::from(layer.rect.left) * scale).round() as i64;
    let y = (f64::from(layer.rect.top) * scale).round() as i64;
    let (lw, lh) = (buffer.width as usize, buffer.height as usize);
    let lplane = lw * lh;
    let mut frame = PixelBuffer::new(frame_w, frame_h, 4);
    let fplane = (frame_w * frame_h) as usize;
    for row in 0..lh {
        let fy = y + row as i64;
        if fy < 0 || fy >= i64::from(frame_h) {
            continue;
        }
        for col in 0..lw {
            let fx = x + col as i64;
            if fx < 0 || fx >= i64::from(frame_w) {
                continue;
            }
            let dst = fy as usize * frame_w as usize + fx as usize;
            let src = row * lw + col;
            for c in 0..4 {
                frame.data[c * fplane + dst] = buffer.data[c * lplane + src];
            }
        }
    }
    Some(frame)
}

fn filter_proxy_base(mut view: Pin<&mut PictureView>, max_edge: i32) -> QImage {
    let Ok(max_edge) = u32::try_from(max_edge) else {
        return QImage::default();
    };
    if max_edge == 0 {
        return QImage::default();
    }
    let mut rust = view.as_mut().rust_mut();
    let gpu_compute = rust.gpu_compute;
    let active = rust.layer_target.single().map(str::to_string);
    let Some(doc) = rust.session.doc.as_mut() else {
        return QImage::default();
    };
    let Some(active_path) = active.as_deref() else {
        return QImage::default();
    };
    if active_proxy_layer(doc, Some(active_path)).is_none() {
        return QImage::default();
    }
    let Some(base) = hidden_layer_buffer(doc, active_path, gpu_compute) else {
        return QImage::default();
    };
    let (width, height) = proxy_dims(doc.width, doc.height, max_edge);
    match pictura_ops::resize(&base, width, height, pictura_ops::Resample::Bilinear) {
        Ok(scaled) => buffer_to_image(&pictura_codec::buffer_to_srgb(doc, &scaled)),
        Err(_) => QImage::default(),
    }
}

fn filter_proxy_layer(
    view: &PictureView,
    kind: &QString,
    params: &QList<f64>,
    max_edge: i32,
) -> QImage {
    let Ok(max_edge) = u32::try_from(max_edge) else {
        return QImage::default();
    };
    if max_edge == 0 {
        return QImage::default();
    }
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return QImage::default();
    };
    let Some(active) = rust.layer_target.single() else {
        return QImage::default();
    };
    let Some(resolved) = active_proxy_layer(doc, Some(active)) else {
        return QImage::default();
    };
    // An open preview has already re-filtered the live layer; the proxy must
    // start from its pre-preview snapshot so the two never compound.
    let original = rust
        .filter_preview
        .as_ref()
        .filter(|p| p.layer_path.as_str() == active)
        .map(|p| p.baseline.original(resolved));
    let layer = original.as_deref().unwrap_or(resolved);
    let params: Vec<f64> = params.into_iter().copied().collect();
    proxy_layer_frame(
        layer,
        doc.width,
        doc.height,
        max_edge,
        &kind.to_string(),
        &params,
    )
}

fn filter_canvas_proxy(
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

fn filter_canvas_proxy_release(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().canvas_proxy = None;
}

fn filter_kind_positional(kind: &QString) -> bool {
    filter_preview_needs_whole_layer(&kind.to_string())
}

fn filter_displace_map_load(path: &QString) -> i32 {
    match displace_maps::load(&path.to_string()) {
        Ok(id) => id as i32,
        Err(reason) => {
            eprintln!("pictura: displacement map refused: {reason}");
            0
        }
    }
}

fn filter_lens_blur_depth(view: &PictureView, source: i32) -> i32 {
    let rust = view.rust();
    match lens_blur_depths::materialize(rust, source) {
        Some(map) => lens_blur_depths::register(map) as i32,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_layer_or_an_empty_selected_area_has_nothing_to_filter() {
        use crate::cxxqt_object::tests::pixel_layer;
        use pictura_select::Selection;

        let mut layer = pixel_layer("layer", 4, 4, (10, 20, 30));
        assert!(!selected_area_empty(&layer, None));
        let mut no_alpha = layer.clone();
        no_alpha.channels.retain(|c| c.id != -1);
        assert!(!selected_area_empty(&no_alpha, None), "an opaque layer");

        // One opaque pixel at (3, 1); the rest transparent.
        let mut alpha = vec![0u8; 16];
        alpha[4 + 3] = 255;
        layer.channels.last_mut().unwrap().data = alpha.into();
        assert!(!selected_area_empty(&layer, None));
        let mut selection = Selection {
            width: 4,
            height: 4,
            data: vec![0; 16],
        };
        selection.data[0] = 255;
        assert!(
            selected_area_empty(&layer, Some(&selection)),
            "selection misses it"
        );
        selection.data[4 + 3] = 1;
        assert!(!selected_area_empty(&layer, Some(&selection)));

        layer.channels.last_mut().unwrap().data = vec![0u8; 16].into();
        assert!(selected_area_empty(&layer, None), "fully transparent");
        let mut sized_out = pixel_layer("new", 0, 0, (0, 0, 0));
        assert!(
            selected_area_empty(&sized_out, None),
            "a zero-size new layer"
        );
        sized_out.channels.clear();
        assert!(selected_area_empty(&sized_out, None));
    }

    #[test]
    fn a_stack_takes_each_kinds_slots_in_turn() {
        let mut kinds = QStringList::default();
        kinds.append(QString::from("cutout"));
        kinds.append(QString::from("palette-knife"));
        let mut params = QList::<f64>::default();
        for v in [4.0, 4.0, 2.0, 25.0, 3.0, 0.0] {
            params.append(v);
        }
        let stack = resolve_stack(&kinds, &params).expect("stack resolves");
        assert_eq!(stack.len(), 2);
        assert_eq!(
            stack[1],
            pictura_filters::Filter::PaletteKnife {
                stroke_size: 25,
                stroke_detail: 3,
                softness: 0,
            }
        );
        // One slot short, or one over, is refused rather than misread.
        params.remove(5);
        assert!(resolve_stack(&kinds, &params).is_none());
        params.append(0.0);
        params.append(0.0);
        assert!(resolve_stack(&kinds, &params).is_none());
    }

    #[test]
    fn rgba_round_trips_through_a_planar_buffer() {
        let rgba = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let buffer = rgba_to_buffer(&rgba, 2, 1).expect("buffer");
        assert_eq!(buffer.data[0], 1);
        assert_eq!(buffer.data[1], 5);
        assert_eq!(buffer.data[2 * 2 + 1], 7);
        assert_eq!(buffer.data[3 * 2], 4);
        assert!(rgba_to_buffer(&rgba, 3, 1).is_none());
    }

    #[test]
    fn the_proxy_base_hides_the_active_layer_and_restores_it() {
        use crate::cxxqt_object::tests::pixel_layer;
        use pictura_core::{BitDepth, ColorMode};

        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("bottom", 4, 4, (255, 0, 0)),
            pixel_layer("top", 4, 4, (0, 0, 255)),
        ]);
        doc.composite = pictura_render::composite_rgba(&doc);

        let base = hidden_layer_buffer(&mut doc, "1", false).expect("base");
        assert!(doc.layers[1].visible, "hiding must be undone");
        let plane = 16;
        assert_eq!(base.data[0], 255, "the red bottom layer");
        assert_eq!(base.data[2 * plane], 0, "the blue top layer is still in");
        assert_eq!(base.data[3 * plane], 255);
    }

    #[test]
    fn the_proxy_layer_filters_and_positions_only_the_active_layer() {
        use crate::cxxqt_object::tests::pixel_layer;
        use pictura_core::{BitDepth, ColorMode, PsdRect};

        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        let mut top = pixel_layer("top", 2, 2, (0, 0, 255));
        top.rect = PsdRect {
            top: 2,
            left: 2,
            bottom: 4,
            right: 4,
        };
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("bottom", 8, 8, (255, 0, 0)),
            top,
        ]);

        let frame = proxy_layer_frame_buffer(&doc.layers[1], 8, 8, 8, "gaussian-blur", &[1.0])
            .expect("frame");
        let plane = 64;
        let at = |x: usize, y: usize| y * 8 + x;
        // Inside the layer's rect the uniform blue survives its blur...
        assert_eq!(frame.data[at(2, 2)], 0);
        assert_eq!(frame.data[plane + at(2, 2)], 0);
        assert_eq!(frame.data[2 * plane + at(2, 2)], 255);
        assert_eq!(frame.data[3 * plane + at(2, 2)], 255);
        // ...and everywhere else the frame is transparent: the bottom layer
        // and the unfiltered position are not drawn.
        assert_eq!(frame.data[3 * plane + at(0, 0)], 0);
        assert_eq!(frame.data[3 * plane + at(5, 5)], 0);
    }

    #[test]
    fn the_proxy_layer_bakes_opacity_and_mask_into_the_alpha() {
        use crate::cxxqt_object::tests::pixel_layer;

        let mut layer = pixel_layer("top", 2, 2, (0, 0, 255));
        layer.opacity = 128;
        let buffer = proxy_layer_buffer(&layer, 1.0).expect("layer buffer");
        assert_eq!(buffer.data[3 * 4], 128, "opacity baked into the alpha");
    }

    #[test]
    fn a_nested_active_layer_resolves_for_the_proxy() {
        use crate::cxxqt_object::tests::pixel_layer;
        use pictura_core::{BitDepth, ColorMode};

        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        let mut group = pixel_layer("group", 4, 4, (0, 0, 0));
        group.is_group = true;
        group.channels.clear();
        group.children = pictura_core::LayerStack::from_layers(vec![
            pixel_layer("nested-red", 4, 4, (255, 0, 0)),
            pixel_layer("nested-blue", 4, 4, (0, 0, 255)),
        ]);
        doc.layers = pictura_core::LayerStack::from_layers(vec![group]);
        doc.composite = pictura_render::composite_rgba(&doc);

        // The group itself is not a proxy target; the nested leaf is.
        assert!(active_proxy_layer(&doc, Some("0")).is_none());
        assert!(active_proxy_layer(&doc, Some("0/1")).is_some());

        let base = hidden_layer_buffer(&mut doc, "0/1", false).expect("nested base");
        let plane = 16;
        assert_eq!(base.data[0], 255, "the red nested layer survives");
        assert_eq!(base.data[2 * plane], 0, "the blue nested layer is hidden");
        assert!(doc.layers[0].children[1].visible, "visibility restored");
    }
}
