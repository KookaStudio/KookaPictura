//! Select > Color Range: the dialog's preview mask and its OK. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). `select` is the dialog's Select index (0 Sampled
//! Colors … 9 Shadows); `rgb` the sampled colour as `0xRRGGBB`.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether Color Range can run: a document that is not 32 bits per channel.
        fn color_range_available(view: &PictureView) -> bool;

        /// The coverage mask (one byte a pixel, row-major) of the composite scaled to fit `max_side`, its size in `width` / `height`; empty when unavailable.
        fn color_range_preview(
            view: &PictureView,
            select: i32,
            rgb: u32,
            fuzziness: i32,
            invert: bool,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Select the range: a new selection, or refine (intersect) the current one, as CS6 does. One "Color Range" state; false when unavailable.
        fn color_range_apply(
            view: Pin<&mut PictureView>,
            select: i32,
            rgb: u32,
            fuzziness: i32,
            invert: bool,
        ) -> bool;
    }
}
