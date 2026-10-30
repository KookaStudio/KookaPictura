//! Image > Adjustments > Replace Color (#164): the dialog's sample-driven HSL
//! shift, previewed on the canvas through the Filter preview machinery and
//! committed as one "Replace Color" state. Free functions over a
//! [`PictureView`] (their own bridge), including the dialog's selection-mask
//! thumbnail.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The Replace Color selection mask as a greyscale `QImage` fitted into
        /// a `size` box (white fully selected). Null when there is no document.
        fn image_replace_color_mask(
            view: &PictureView,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            size: i32,
        ) -> QImage;

        /// Preview Replace Color on the active layer over the document rect
        /// `(x, y, w, h)` (the whole layer when `w` is not positive), re-applied
        /// from the pre-preview pixels; no history. False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_replace_color_preview(
            view: Pin<&mut PictureView>,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            hue: f64,
            saturation: f64,
            lightness: f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply Replace Color to the whole active layer as one "Replace Color"
        /// state (an open preview is replaced). False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_replace_color_apply(
            view: Pin<&mut PictureView>,
            samples: &QString,
            fuzziness: f64,
            localized: bool,
            hue: f64,
            saturation: f64,
            lightness: f64,
        ) -> bool;

        /// The Result swatch: `sample` (`0xRRGGBB`) shifted by `hue` degrees
        /// and `saturation` / `lightness` percent, as `0xRRGGBB`.
        fn image_replace_color_result(
            sample: u32,
            hue: f64,
            saturation: f64,
            lightness: f64,
        ) -> u32;

        /// The rounded `[hue, saturation, lightness]` that take `sample`
        /// closest to `result` (both `0xRRGGBB`).
        fn image_replace_color_shift_for(sample: u32, result: u32) -> Vec<i32>;
    }
}
