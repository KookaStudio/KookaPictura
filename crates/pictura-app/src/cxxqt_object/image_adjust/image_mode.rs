//! Image ▸ Mode: the color-mode and bit-depth conversions, the Indexed Color
//! dialog's live preview, and the menu's enabled/checked state. Free functions
//! over a [`PictureView`] (their own bridge). Ported from photorust's
//! `setColorMode` / `convertToIndexed` / `setBitDepth` bridge.
//!
//! Every conversion records one `Convert Mode` state (inferred CS6 label).
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
pub(crate) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The document's mode as the user sees it (`bitmap`, `grayscale`,
        /// `duotone`, `indexed`, `rgb`, `cmyk`, `lab`, `multichannel`); empty
        /// without a document.
        fn image_mode(view: &PictureView) -> QString;

        /// The document's bits per channel as the user sees it (1 for Bitmap);
        /// 0 without a document.
        fn image_depth_bits(view: &PictureView) -> i32;

        /// The document's resolution for the Properties panel, e.g. `"72
        /// pixels/inch"`; empty without a document.
        fn image_resolution(view: &PictureView) -> QString;

        /// Whether Image ▸ Mode ▸ `mode` is available (an `image_mode` name).
        fn image_mode_available(view: &PictureView, mode: &QString) -> bool;

        /// Whether Image ▸ Mode ▸ `bits` Bits/Channel is available.
        fn image_depth_available(view: &PictureView, bits: i32) -> bool;

        /// Convert to `grayscale`, `rgb`, `cmyk`, or `lab`. One "Convert Mode"
        /// state; false (nothing recorded) when the conversion is refused.
        fn convert_image_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool;

        /// Convert between 8, 16, and 32 bits per channel (a 32-bit source
        /// goes through `convert_depth`'s HDR toning instead). One "Convert
        /// Mode" state; false when refused.
        fn convert_image_depth(view: Pin<&mut PictureView>, bits: i32) -> bool;

        /// Whether the image has at most 256 colors (the Exact palette).
        fn indexed_exact_available(view: &PictureView) -> bool;

        /// Image ▸ Mode ▸ Indexed Color. `palette`: 0 Exact, 1 Web, 2 Local
        /// (Perceptual), 3 Local (Selective), 4 Local (Adaptive); `dither`: 0
        /// None, 1 Diffusion, 2 Pattern, 3 Noise; `amount` 0–100. Flattens.
        /// Ends any preview; one "Convert Mode" state; false when refused.
        fn convert_to_indexed(
            view: Pin<&mut PictureView>,
            palette: i32,
            colors: i32,
            dither: i32,
            amount: i32,
        ) -> bool;

        /// Show the Indexed Color conversion on the canvas without recording
        /// history; each call converts the pre-preview document again.
        fn preview_indexed(
            view: Pin<&mut PictureView>,
            palette: i32,
            colors: i32,
            dither: i32,
            amount: i32,
        ) -> bool;

        /// Restore the document from before `preview_indexed`; false when no
        /// preview is shown.
        fn cancel_mode_preview(view: Pin<&mut PictureView>) -> bool;

        /// Image ▸ Mode ▸ Bitmap from 8-bit Grayscale. `method`: 0 50%
        /// Threshold, 1 Pattern Dither, 2 Diffusion Dither. Flattens to a flat
        /// image; one "Convert Mode" state; false when refused.
        fn convert_to_bitmap(view: Pin<&mut PictureView>, method: i32) -> bool;
    }
}
