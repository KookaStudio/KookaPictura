//! The raster-export bridge: flatten the document composite and write it through
//! Qt's image writers, except WebP, which the engine encodes because Qt's WebP
//! writer is an optional plugin. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::export_tools::*;
use super::qobject::PictureView;
use super::save_format::{
    raster_preview_bytes, save_document_format, save_format_aliases, save_format_canonical,
    save_format_code, save_format_flags, save_format_id_for_source_name, save_format_id_for_suffix,
    set_save_gif_options, set_save_options, set_save_raster_options,
};
pub use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("encode_image.h");
        /// Encode packed RGBA8888 to `path` with the named Qt image writer; false without writing on any failure.
        fn encode_image_rgba(
            rgba: &[u8],
            width: i32,
            height: i32,
            path: &str,
            format: &str,
            quality: i32,
            scale: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
        ) -> bool;

        /// Rescale packed RGBA8888 by `scale` percent with a smooth filter and report the new size in `out_width`/`out_height`.
        fn scale_rgba(
            rgba: &[u8],
            width: i32,
            height: i32,
            scale: i32,
            out_width: &mut i32,
            out_height: &mut i32,
        ) -> Vec<u8>;

        /// Encode packed RGBA8888 to memory with the named Qt image writer and
        /// return the bytes; empty on any refusal.
        fn encode_image_bytes(
            rgba: &[u8],
            width: i32,
            height: i32,
            format: &str,
            quality: i32,
            scale: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
        ) -> Vec<u8>;
    }

    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The output format remembered from the import source (`"psd"` when native).
        fn output_format(view: &PictureView) -> QString;

        /// The active layer's opacity (0-255), or -1 for the Background or none.
        fn active_layer_opacity(view: &PictureView) -> i32;

        /// Flatten the document composite and write it to `path` through the Qt encode edge. Never changes the document, its path, dirty flag, or history; false without writing on refusal.
        fn export_image(
            view: &PictureView,
            path: &QString,
            format: &QString,
            quality: i32,
            scale: i32,
        ) -> bool;

        /// Encode packed RGBA8888 as a WebP container; `lossless` or a negative
        /// `quality` selects VP8L, otherwise lossy VP8 at `quality` (0-100).
        /// Empty on refusal.
        fn webp_encode(
            rgba: &[u8],
            width: i32,
            height: i32,
            quality: i32,
            lossless: bool,
        ) -> Vec<u8>;

        /// Flatten the document composite and write it as WebP. `lossless`
        /// selects VP8L, otherwise lossy VP8 at `quality` (0-100). Never
        /// changes the document, its path, dirty flag, or history; false
        /// without writing on refusal.
        fn export_webp(view: &PictureView, path: &QString, quality: i32, lossless: bool) -> bool;

        /// Store the format options the next raster save of this document uses.
        fn set_save_raster_options(
            view: Pin<&mut PictureView>,
            quality: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
        );

        /// Store the typed Save As kind and JSON options for the engine writers.
        fn set_save_options(view: Pin<&mut PictureView>, kind: i32, json: &str);

        /// The catalog ordinal a typed suffix names, or -1. Alias-only.
        fn save_format_id_for_suffix(suffix: &str) -> i32;

        /// The catalog ordinal a remembered source name or suffix names, or -1.
        fn save_format_id_for_source_name(name: &str) -> i32;

        /// The policy flags for a catalog ordinal; 0 when the id is unknown.
        fn save_format_flags(id: i32) -> u32;

        /// The options wire code for a catalog ordinal; 0 when it opens no dialog.
        fn save_format_code(id: i32) -> i32;

        /// The canonical extension for a catalog ordinal; empty when unknown.
        fn save_format_canonical(id: i32) -> QString;

        /// The lowercase aliases for a catalog ordinal as one space-separated string.
        fn save_format_aliases(id: i32) -> QString;

        /// Save `path` with a shell-resolved catalog ordinal; -1 resolves from the path.
        fn save_document_format(
            view: Pin<&mut PictureView>,
            path: &QString,
            format_id: i32,
            copy: bool,
        ) -> bool;

        /// Store the palette settings the next GIF save of this document uses.
        fn set_save_gif_options(
            view: Pin<&mut PictureView>,
            palette: i32,
            colors: i32,
            dither: i32,
            amount: i32,
            interlaced: bool,
        );

        /// The composite encoded at the given settings, for the JPEG dialog's
        /// size estimate and preview; empty when there is no document.
        fn raster_preview_bytes(
            view: &PictureView,
            format: &QString,
            quality: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
        ) -> Vec<u8>;

        /// The document's Work Path and saved paths as Illustrator-8 EPS bytes; empty without a document.
        fn illustrator_paths_eps(view: &PictureView) -> Vec<u8>;

        /// The document's Work Path and saved paths as PDF-based `.ai` bytes; empty without a document.
        fn illustrator_paths_pdf(view: &PictureView) -> Vec<u8>;

        /// Whether the document has at least one path to export.
        fn illustrator_has_paths(view: &PictureView) -> bool;

        /// The Zoomify plan flattened as `[base_w, base_h, tiers, tiles]`, then per tier `[level, w, h, tiles]`, then per tile `[x, y, left, top, w, h]`.
        fn zoomify_plan_flat(base_width: i32, base_height: i32) -> Vec<i32>;

        /// The `TileGroup` index for a tile ordinal.
        fn zoomify_group_for(ordinal: i32) -> i32;

        /// The `ImageProperties.xml` text for a base size; empty for a zero size.
        fn zoomify_properties(base_width: i32, base_height: i32) -> QString;

        /// The self-contained viewer HTML for a base size; empty for a zero size.
        fn zoomify_html(base_width: i32, base_height: i32) -> QString;

        /// One line per export target: `layer path<TAB>base file name` (no suffix). Empty without a document.
        fn export_layer_targets(
            view: &PictureView,
            visible_only: bool,
            prefix: &QString,
        ) -> QString;

        /// Write one layer to `path` as `format` (PSD/JPEG/TIFF/BMP/PDF), alone on a document-sized canvas; false without writing on any failure.
        fn export_layer_file(
            view: &PictureView,
            layer_path: &QString,
            format: &QString,
            include_icc: bool,
            path: &QString,
        ) -> bool;

        /// Hold each export unit for `micros` microseconds; the tests' deterministic cancel seam.
        fn export_tools_set_delay(view: &PictureView, micros: i32);
    }
}
