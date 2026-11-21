//! File > Save for Web & Devices: the flattened sRGB image, palette reduction,
//! and the GIF and WBMP encoders (`pictura_codec::web`). Free functions (their
//! own bridge). JPEG, PNG-8, and PNG-24 are written by the dialog through Qt.

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    /// Palette settings: `reduction` 0 Perceptual / 1 Selective / 2 Adaptive /
    /// 3 Restrictive (Web) / 4 Black and White / 5 Grayscale; `colors` 2–256;
    /// `dither` 0 None / 1 Diffusion / 2 Pattern / 3 Noise and its `amount`
    /// 0–100 %; `transparency`; `mapped_transparency` (a custom table's clear
    /// entry also accepts opaque pixels of its colour); the `matte`
    /// (`0xAARRGGBB`).
    #[namespace = "pictura"]
    struct WebPalette {
        reduction: i32,
        colors: i32,
        dither: i32,
        amount: i32,
        transparency: bool,
        mapped_transparency: bool,
        matte: u32,
    }

    #[namespace = "pictura"]
    struct WebSliceCell {
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    #[namespace = "pictura"]
    struct WebMetadata {
        exif: Vec<u8>,
        iptc: Vec<u8>,
        xmp: Vec<u8>,
        history_xmp: Vec<u8>,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The flattened document as sRGB RGBA8888, `document_width` x `document_height`; empty without a document.
        fn web_flattened(view: &PictureView) -> Vec<u8>;

        /// Top-level layer `index` rendered alone on transparency and cropped to its bounds: a little-endian `u32` width, a `u32` height, then RGBA8888; empty without that layer.
        fn layer_flattened(view: &PictureView, index: i32) -> Vec<u8>;

        /// Reduce RGBA8888 `width` x `height` to a palette: `[n_hi, n_lo]`, then `n` RGBA entries (alpha 0 marks the transparent entry), then one index per pixel.
        fn web_quantize(rgba: &[u8], width: i32, height: i32, palette: &WebPalette) -> Vec<u8>;

        /// Encode a `web_quantize` result as a GIF89a file, `interlaced` or not.
        fn web_encode_gif(indexed: &[u8], width: i32, height: i32, interlaced: bool) -> Vec<u8>;

        /// Encode RGBA8888 as a black-and-white WBMP with `dither` (as `WebPalette::dither`) at `amount` %.
        fn web_encode_wbmp(
            rgba: &[u8],
            width: i32,
            height: i32,
            dither: i32,
            amount: i32,
        ) -> Vec<u8>;

        /// Reduce RGBA8888 to `palette` plus packed opaque RGB triples `locked`.
        fn web_quantize_locked(
            rgba: &[u8],
            width: i32,
            height: i32,
            palette: &WebPalette,
            locked: &[u8],
        ) -> Vec<u8>;

        /// Reduce RGBA8888 to a caller-supplied table of packed RGBA entries.
        fn web_quantize_custom(
            rgba: &[u8],
            width: i32,
            height: i32,
            palette: &WebPalette,
            table: &[u8],
        ) -> Vec<u8>;

        /// Sort a packed table: 0 hue, 1 luminance, 2 popularity.
        fn web_palette_sort(packed: &[u8], width: i32, height: i32, sort: i32) -> Vec<u8>;

        /// Delete packed table entry `index`, remapping affected pixels.
        fn web_palette_delete(packed: &[u8], width: i32, height: i32, index: i32) -> Vec<u8>;

        /// Parse a `.act` into `[count_hi, count_lo, RGBA entries..., transparent+1]`.
        fn web_act_parse(bytes: &[u8]) -> Vec<u8>;

        /// Encode the same packed table as a `.act`.
        fn web_act_encode(packed: &[u8]) -> Vec<u8>;

        /// The document's raw EXIF/IPTC/XMP blocks, empty when absent.
        fn web_metadata(view: &PictureView) -> WebMetadata;

        /// Filter the raw blocks by `level` (0 None, 1 Copyright, 2 Copyright and
        /// Contact, 3 All Except Camera, 4 All) and embed them in `bytes`.
        fn web_embed_metadata(
            bytes: &[u8],
            suffix: &QString,
            raw: &WebMetadata,
            level: i32,
        ) -> Vec<u8>;

        fn web_history_omitted(suffix: &QString, raw: &WebMetadata, level: i32) -> bool;

        /// All disjoint export slices as repeated `[x, y, width, height, number, userIndex]`.
        fn web_export_slices(view: &PictureView) -> Vec<i32>;

        /// Emit the Save for Web slice HTML/CSS for `cells` (`layout` 0 table, 1 CSS).
        fn web_slice_html(
            cells: Vec<WebSliceCell>,
            canvas_width: i32,
            canvas_height: i32,
            title: &QString,
            image_dir: &QString,
            layout: i32,
        ) -> QString;
    }
}
