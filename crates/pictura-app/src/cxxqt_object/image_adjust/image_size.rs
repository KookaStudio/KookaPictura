//! Image ▸ Image Size / Canvas Size. Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//! Ported from photorust's `showImageSize` / `showCanvasSize`.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use super::new_document::{create_document, document_budget_allows, document_resize_budget_allows};
pub(in crate::cxxqt_object) use imp::image_size_fits_budget;
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

        /// Whether Image Size can resample the document to `width` ×
        /// `height`: each side within the image budget, and the resize's
        /// working set within memory and its planes within memory plus the
        /// scratch disks (#1293). A refusal is recorded, so
        /// `edit_refused_for_scratch_full` tells the two apart.
        fn image_size_fits_budget(view: &PictureView, width: i32, height: i32) -> bool;

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

        /// A new `width` × `height` document in `mode` (`"bitmap"`,
        /// `"grayscale"`, `"rgb"`, `"cmyk"`, `"lab"`) at `bits` per channel
        /// (1 for Bitmap; else 8, 16, or 32), its layer filled with `fill`
        /// (`0xRRGGBB`) or left clear when `transparent`, at `ppi` pixels per
        /// inch (shown per cm when `per_cm`). One "New" state; false when
        /// refused (an unknown mode, a depth the mode cannot take, or a
        /// dimension below 1).
        #[allow(clippy::too_many_arguments)]
        fn create_document(
            view: Pin<&mut PictureView>,
            width: i32,
            height: i32,
            mode: &QString,
            bits: i32,
            fill: u32,
            transparent: bool,
            ppi: f64,
            per_cm: bool,
        ) -> bool;

        /// Whether a `width` × `height` document at `bits` per channel fits
        /// `ImageBudget`, so a caller can refuse before allocating: an
        /// allocation failure aborts the process.
        fn document_budget_allows(width: i32, height: i32, bits: i32) -> bool;

        /// Whether an immediate resize to `width` × `height` at `bits` per
        /// channel fits the memory available now (the resize gate, #1156).
        fn document_resize_budget_allows(width: i32, height: i32, bits: i32) -> bool;
    }
}
