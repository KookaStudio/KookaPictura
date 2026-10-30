//! Image ▸ Image Size / Canvas Size. Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//! Ported from photorust's `showImageSize` / `showCanvasSize`.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use imp::*;

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
        /// The document's resolution in pixels per inch (CS6's 72 when the
        /// file names none).
        fn document_ppi(view: &PictureView) -> f64;

        /// Whether the document shows its resolution in pixels/cm.
        fn document_ppi_per_cm(view: &PictureView) -> bool;

        /// Image ▸ Image Size: resample to `width` × `height` with `resample`
        /// (`"nearest"`, `"bilinear"`, `"bicubic"`; empty keeps the pixels) and
        /// set the resolution to `ppi` (shown per cm when `per_cm`). One
        /// "Image Size" state; false when nothing changes or the size is
        /// refused.
        fn image_size_apply(
            view: Pin<&mut PictureView>,
            resample: &QString,
            width: i32,
            height: i32,
            ppi: f64,
            per_cm: bool,
        ) -> bool;

        /// Image ▸ Canvas Size: place the document on a `width` × `height`
        /// canvas at `anchor` (`"top-left"` … `"bottom-right"`), filling the
        /// Background layer's new area with `fill` (`0xRRGGBB`). One
        /// "Canvas Size" state; false when nothing changes or it is refused.
        fn canvas_size_apply(
            view: Pin<&mut PictureView>,
            anchor: &QString,
            width: i32,
            height: i32,
            fill: u32,
        ) -> bool;
    }
}
