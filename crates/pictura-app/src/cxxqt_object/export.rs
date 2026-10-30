//! The raster-export bridge: flatten the document composite and write it through
//! Qt's image writers, except WebP, which the engine encodes because Qt's WebP
//! writer is an optional plugin. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow).
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
        /// Encode packed RGBA8888 to `path` with the named Qt image writer; false without writing on any failure.
        fn encode_image_rgba(
            rgba: &[u8],
            width: i32,
            height: i32,
            path: &str,
            format: &str,
            quality: i32,
            scale: i32,
            compression: i32,
            optimized: bool,
            progressive: bool,
            matte: i32,
        ) -> bool;

        /// Rescale packed RGBA8888 by `scale` percent with a smooth filter and report the new size in `out_width`/`out_height`.
        fn scale_rgba(
            rgba: &[u8],
            width: i32,
            height: i32,
            scale: i32,
            out_width: &mut i32,
            out_height: &mut i32,
        ) -> Vec<u8>;
    }

    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The output format remembered from the import source (`"psd"` when native).
        fn output_format(view: &PictureView) -> QString;

        /// The active layer's opacity (0-255), or -1 for the Background or none.
        fn active_layer_opacity(view: &PictureView) -> i32;

        /// Flatten the document composite and write it to `path` through the Qt encode edge. Never changes the document, its path, dirty flag, or history; false without writing on refusal.
        fn export_image(
            view: &PictureView,
            path: &QString,
            format: &QString,
            quality: i32,
            scale: i32,
        ) -> bool;

        /// Encode packed RGBA8888 as a lossless WebP container; empty on refusal.
        fn webp_encode(rgba: &[u8], width: i32, height: i32) -> Vec<u8>;
    }
}
