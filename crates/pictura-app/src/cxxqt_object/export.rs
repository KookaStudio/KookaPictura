//! The raster-export bridge: flatten the document composite and write it through
//! Qt's image writers, except WebP, which the engine encodes because Qt's WebP
//! writer is an optional plugin. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers_composite::{buffer_to_rgba_bytes, current_buffer};
use super::qobject::PictureView;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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

/// Encode packed RGBA8888 as a lossless WebP container; empty on refusal.
///
/// The Export As dialog previews and writes per-entry bytes in memory, so this
/// exposes the engine encoder directly rather than the path-based save edge.
pub fn webp_encode(rgba: &[u8], width: i32, height: i32) -> Vec<u8> {
    if width <= 0 || height <= 0 {
        return Vec::new();
    }
    pictura_codec::encode_webp(rgba, width as u32, height as u32).unwrap_or_default()
}

pub fn output_format(view: &PictureView) -> QString {
    QString::from(view.rust().source_format.as_str())
}

pub fn export_image(
    view: &PictureView,
    path: &QString,
    format: &QString,
    quality: i32,
    scale: i32,
) -> bool {
    let path = path.to_string();
    let format = format.to_string();
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let buffer = current_buffer(doc, rust.gpu_compute);
    let srgb = pictura_codec::buffer_to_srgb(doc, &buffer);
    let rgba = buffer_to_rgba_bytes(&srgb);
    encode_raster(
        &rgba,
        srgb.width,
        srgb.height,
        &path,
        &format,
        quality,
        scale,
    )
}

/// Encode packed RGBA8888 to `path` in `format` at `scale` percent: `WEBP`
/// through the engine's native codec, every other format through the Qt
/// boundary. False without writing on any failure.
pub(super) fn encode_raster(
    rgba: &[u8],
    width: u32,
    height: u32,
    path: &str,
    format: &str,
    quality: i32,
    scale: i32,
) -> bool {
    if format.eq_ignore_ascii_case("WEBP") {
        encode_webp_rgba(rgba, width, height, path, scale)
    } else {
        ffi::encode_image_rgba(
            rgba,
            width as i32,
            height as i32,
            path,
            format,
            quality,
            scale,
        )
    }
}

/// Rescale through the boundary (so the percent-scale contract matches the Qt
/// formats), encode losslessly in the engine, and write temp-file + rename.
fn encode_webp_rgba(rgba: &[u8], width: u32, height: u32, path: &str, scale: i32) -> bool {
    let mut out_width = width as i32;
    let mut out_height = height as i32;
    let scaled = ffi::scale_rgba(
        rgba,
        width as i32,
        height as i32,
        scale,
        &mut out_width,
        &mut out_height,
    );
    if out_width <= 0 || out_height <= 0 {
        return false;
    }
    let Some(bytes) = pictura_codec::encode_webp(&scaled, out_width as u32, out_height as u32)
    else {
        return false;
    };
    super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok()
}
