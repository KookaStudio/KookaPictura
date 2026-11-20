//! Select > Color Range: the dialog's preview mask, its image-window Selection
//! Preview, and its OK. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). `select` is
//! the dialog's Select index (0 Sampled Colors … 10 Shadows, 11 Out Of Gamut);
//! `samples` is `"x,y,r,g,b;…"` with document positions.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

pub(in crate::cxxqt_object) use imp::ColorRangePreviewState;
use imp::{
    color_range_apply, color_range_available, color_range_out_of_gamut_available,
    color_range_preview, color_range_preview_clear, color_range_preview_image,
    color_range_preview_set,
};

use super::qobject::PictureView;

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
        /// Whether Color Range can run: a document that is not 32 bits per channel.
        fn color_range_available(view: &PictureView) -> bool;

        /// Whether the Out Of Gamut mode is offered: RGB documents only.
        fn color_range_out_of_gamut_available(view: &PictureView) -> bool;

        /// The coverage mask (one byte a pixel, row-major) of the composite
        /// scaled to fit `max_side`, its size in `width` / `height`; empty when
        /// unavailable. The mask is intersected with a live selection.
        #[allow(clippy::too_many_arguments)]
        fn color_range_preview(
            view: &PictureView,
            select: i32,
            samples: &QString,
            fuzziness: i32,
            localized: bool,
            range: i32,
            invert: bool,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// The bounded composite for the dialog's Image display.
        fn color_range_preview_image(view: &PictureView, max_side: i32) -> QImage;

        /// Store the display-only image-window Selection Preview: settings plus
        /// the mode (0 None, 1 Grayscale, 2 Black Matte, 3 White Matte,
        /// 4 Quick Mask). The document and selection are untouched.
        #[allow(clippy::too_many_arguments)]
        fn color_range_preview_set(
            view: Pin<&mut PictureView>,
            select: i32,
            samples: &QString,
            fuzziness: i32,
            localized: bool,
            range: i32,
            invert: bool,
            mode: i32,
        ) -> bool;

        /// Clear the image-window preview; true when one was active.
        fn color_range_preview_clear(view: Pin<&mut PictureView>) -> bool;

        /// Select the range: a new selection, or refine (intersect) the current
        /// one, as CS6 does. One "Color Range" state; false when unavailable.
        #[allow(clippy::too_many_arguments)]
        fn color_range_apply(
            view: Pin<&mut PictureView>,
            select: i32,
            samples: &QString,
            fuzziness: i32,
            localized: bool,
            range: i32,
            invert: bool,
        ) -> bool;
    }
}
