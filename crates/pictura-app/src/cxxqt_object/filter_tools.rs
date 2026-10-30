//! The Filter-menu bridges: apply a filter with runtime parameters, render a
//! non-committing preview, keep the last filter, and the Filter Gallery's stack
//! preview/commit and thumbnails. Free functions over a
//! [`PictureView`] (their own bridge, so the `PictureView` declaration list does
//! not grow). Ported from perfecto25/photorust's filter commit path.
//!
//! Source: https://github.com/perfecto25/photorust
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
pub(super) use canvas_proxy::canvas_proxy_buffer;
pub(super) use canvas_proxy::resolve_stack;
pub(super) use canvas_proxy::scaled_stack_params;
pub(super) use depth_gate::depth_refusal;
pub(super) use depth_gate::document_supported;
use depth_gate::filter_document_takes;
pub(super) use depth_gate::lens_supported;
pub(crate) use imp::*;
pub(super) use vanishing_point_file::saved_planes;
pub(super) use vanishing_point_file::store_planes;

pub(super) mod adaptive_wide_angle;
mod canvas_proxy;
mod depth_gate;
pub(super) mod displace_maps;
mod imp;
pub(super) mod jobs;
pub(super) mod lens_blur_depths;
pub(super) mod lens_correction;
mod lens_profiles;
pub(super) mod liquify;
pub(super) mod smart_apply;
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

    /// The result of reading the active pixel layer's in-tree watermark
    /// (`FILT-105`). `kind` is the watermark type's label, empty when none was
    /// found.
    #[namespace = "pictura"]
    struct WatermarkRow {
        found: bool,
        kind: QString,
        creator_id: i32,
        id: i32,
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
        /// restrict them; every other kind takes any), and, on a Smart
        /// Object, whether `kind` can be a Smart Filter.
        fn filter_document_takes(view: &PictureView, kind: &QString) -> bool;

        /// Whether the edit target is a Smart Object's pixels (no layer mask
        /// targeted), where Image > Adjustments do not apply.
        fn filter_target_smart(view: &PictureView) -> bool;

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

        /// Read the active pixel layer's in-tree watermark (`FILT-105`), if any.
        /// This is the project's own approximate carrier, not Digimarc Barcode.
        fn read_watermark(view: &PictureView) -> WatermarkRow;

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
