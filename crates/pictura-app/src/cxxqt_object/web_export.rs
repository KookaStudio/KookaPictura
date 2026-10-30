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
    /// 0–100 %; `transparency`; the `matte` (`0xAARRGGBB`).
    #[namespace = "pictura"]
    struct WebPalette {
        reduction: i32,
        colors: i32,
        dither: i32,
        amount: i32,
        transparency: bool,
        matte: u32,
    }

    unsafe extern "C++" {
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
    }
}
