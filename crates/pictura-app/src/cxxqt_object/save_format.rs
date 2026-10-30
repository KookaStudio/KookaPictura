//! The per-format raster save options bridge (`interop/save-format-options`):
//! store the settings the per-format dialog accepted on the view, read them
//! back in `save`, and encode the composite to bytes for the JPEG dialog's size
//! estimate and preview. Free functions over a [`PictureView`] (their own
//! bridge, so the central qobject list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("encode_image.h");
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

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Store the format options the next raster save of this document uses.
        fn set_save_raster_options(
            view: Pin<&mut PictureView>,
            quality: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
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
    }
}
