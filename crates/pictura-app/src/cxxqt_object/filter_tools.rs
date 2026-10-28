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
use super::helpers::{active_layer_visible, active_mask_target, active_pixel_layer};
use super::helpers_composite::{
    buffer_to_image, patch_buffer_region, premultiplied_display_image, selection_to_mask,
};
use super::impl_filters::{
    apply_filter_active, apply_filter_active_region, apply_op_active_region, cancel_filter_preview,
    ActiveOp,
};
use super::qobject::PictureView;
use super::state::PictureViewRust;
use crate::cxxqt_object::edit::{structural, Edit};
pub(super) use canvas_proxy::{canvas_proxy_buffer, resolve_stack, scaled_stack_params};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QImage, QList, QString, QStringList};
use depth_gate::filter_document_takes;
pub(super) use depth_gate::{depth_refusal, document_supported, lens_supported};
use pictura_core::{layer_pixel_locked, Document, Layer, PixelBuffer};
pub(super) use vanishing_point_file::{saved_planes, store_planes};

pub(super) mod adaptive_wide_angle;
mod canvas_proxy;
mod depth_gate;
pub(super) mod displace_maps;
pub(super) mod jobs;
pub(super) mod lens_blur_depths;
pub(super) mod lens_correction;
pub(super) mod liquify;
pub(super) mod vanishing_point;
mod vanishing_point_file;
mod vanishing_point_retouch;

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
    struct FilterParamRow {
        control: i32,
        label: QString,
        minimum: f64,
        maximum: f64,
        value: f64,
        slot_count: i32,
        decimals: i32,
        suffix: QString,
        choices: QStringList,
        initial: QList_f64,
        measures_pixels: bool,
    }

    #[namespace = "pictura"]
    struct FilterDescriptorRow {
        kind: QString,
        path: QStringList,
        menu: bool,
        label: QString,
        params: Vec<FilterParamRow>,
        whole_layer: bool,
        dialog: i32,
        map_after_ok: bool,
        preview_pane: bool,
        stacked: bool,
        preview_below: bool,
        live_preview: bool,
        gallery_rank: i32,
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

        /// Whether the document's depth and mode are ones filter `kind`
        /// takes (Adaptive Wide Angle, Vanishing Point, and Lens Correction
        /// restrict them; every other kind takes any).
        fn filter_document_takes(view: &PictureView, kind: &QString) -> bool;

        /// Whether the active pixel layer has nothing to filter: no opaque
        /// pixel at all, or none inside the selection. False when there is no
        /// pixel layer (that is [`filter_target_ready`]'s refusal).
        fn filter_target_empty(view: Pin<&mut PictureView>) -> bool;

        /// Whether `kind` is a filter the parameterised mapper understands.
        fn filter_kind_supported(kind: &QString) -> bool;

        /// The number of parameter slots `kind` expects, or -1 for an unknown
        /// kind.
        fn filter_param_arity(kind: &QString) -> i32;

        /// The filter parameter descriptors, in the C++ table's order.
        fn filter_descriptors() -> Vec<FilterDescriptorRow>;

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
        /// RGBA8888 image, for a gallery thumbnail. `scale` is the image's
        /// document-to-thumbnail scale; pixel-distance parameters are scaled by
        /// it. A null image when the filter is refused.
        fn filter_thumbnail(
            rgba: &QByteArray,
            width: i32,
            height: i32,
            kind: &QString,
            params: &QList_f64,
            scale: f64,
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
    if !super::filter_map::is_filter_command(&kind_s) {
        return false;
    }
    if let Some(reason) = depth_refusal(&kind_s, view.rust().session.doc.as_ref()) {
        eprintln!("pictura: filter '{kind_s}' refused: {reason}");
        view.as_mut().rust_mut().filter_error = Some(reason);
        return false;
    }
    let params_v: Vec<f64> = params.into_iter().copied().collect();
    // Lens Correction on an embedded smart object attaches a smart filter
    // instead of filtering the proxy destructively.
    if kind_s == "lens-correction" {
        if let Some(path) = lens_correction_smart_target(view.rust()) {
            if apply_lens_correction_smart(view.as_mut(), &path, &params_v, "Filter") {
                view.as_mut().rust_mut().last_filter = Some((kind_s, params_v));
                return true;
            }
            return false;
        }
    }
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
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    view.as_mut().edit("Filter", |_| outcome);
    true
}

/// Whether the active layer is an embedded smart object a Lens Correction smart
/// filter can attach to, and its path.
fn lens_correction_smart_target(rust: &PictureViewRust) -> Option<String> {
    let doc = rust.session.doc.as_ref()?;
    let path = rust.layer_target.single()?.to_string();
    pictura_render::can_replace_smart_object_contents(doc, &path).then_some(path)
}

/// Attach a Lens Correction smart filter carrying `slots` to the smart object at
/// `path` and bake its proxy, recording one `label` history state. `false` (no
/// state) when the engine refuses.
fn apply_lens_correction_smart(
    mut view: Pin<&mut PictureView>,
    path: &str,
    slots: &[f64],
    label: &str,
) -> bool {
    let before = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, path));
    let applied = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        pictura_render::apply_lens_correction(doc, path, slots)
    };
    if !applied {
        return false;
    }
    let after = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, path));
    view.as_mut().edit(label, |_| structural(before, after));
    true
}

fn filter_preview(mut view: Pin<&mut PictureView>, kind: &QString, params: &QList<f64>) -> bool {
    let kind_s = kind.to_string();
    if !super::filter_map::is_filter_command(&kind_s) {
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
    super::filter_map::descriptor::descriptor(kind).is_some_and(|descriptor| descriptor.whole_layer)
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
    super::filter_map::is_filter_command(&kind.to_string())
}

fn filter_descriptors() -> Vec<ffi::FilterDescriptorRow> {
    super::filter_map::descriptor::all()
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

fn run_stack(
    mut view: Pin<&mut PictureView>,
    kinds: &QStringList,
    params: &QList<f64>,
    commit: bool,
    section: Option<pictura_core::PsdRect>,
) -> bool {
    let params_v: Vec<f64> = params.into_iter().copied().collect();
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

fn filter_stack_preview(
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
    let scaled = scaled_stack_params(kinds, params, scale);
    let filtered = match (resolve_stack(kinds, &scaled), rust.layer_target.single()) {
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
    scale: f64,
) -> QImage {
    if !super::filter_map::is_filter_command(&kind.to_string()) {
        return QImage::default();
    }
    let params: Vec<f64> = params.into_iter().copied().collect();
    let params =
        super::filter_map::descriptor::scale_pixel_params(&kind.to_string(), &params, scale);
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
    if !super::filter_map::is_filter_command(kind) {
        return None;
    }
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
        .session
        .live
        .filter_preview()
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
mod tests;
