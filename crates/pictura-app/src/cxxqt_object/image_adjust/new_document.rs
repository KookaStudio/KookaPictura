//! File ▸ New: a document in any of the dialog's modes, depths, Background
//! Contents, and resolutions, opened with one "New" history state. The base
//! is [`PictureView::new_document`]'s 8-bit RGB / Grayscale white or
//! transparent layer, made the locked Background unless transparent; the rest is the Image ▸ Mode conversions and the
//! ResolutionInfo writer applied before the history starts. A free function
//! (its own bridge, so the `PictureView` declaration list does not grow).
//! Ported from photorust's NewDocumentDialog.
//!
//! [`PictureView::new_document`]: super::super::qobject::PictureView

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
    }
}
