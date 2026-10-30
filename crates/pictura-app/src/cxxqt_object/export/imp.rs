use super::*;

use super::super::helpers_composite::buffer_to_rgba_bytes;
use super::super::picture::picture;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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

pub fn active_layer_opacity(view: &PictureView) -> i32 {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return -1;
    };
    let Some(path) = rust.layer_target.anchor() else {
        return -1;
    };
    match pictura_render::resolve_path(doc, path) {
        Some(layer) if !layer.background => layer.opacity as i32,
        _ => -1,
    }
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
    let buffer = picture(doc, rust);
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
        -1,
        false,
        false,
        -1,
    )
}

/// Encode packed RGBA8888 to `path` in `format` at `scale` percent: `WEBP`
/// through the engine's native codec, every other format through the Qt
/// boundary. `compression`, `optimized`, `progressive`, and `matte` are the
/// per-format save options. False without writing on any failure.
#[allow(clippy::too_many_arguments)]
pub(in crate::cxxqt_object) fn encode_raster(
    rgba: &[u8],
    width: u32,
    height: u32,
    path: &str,
    format: &str,
    quality: i32,
    scale: i32,
    compression: i32,
    optimized: bool,
    progressive: bool,
    matte: i32,
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
            compression,
            optimized,
            progressive,
            matte,
        )
    }
}

/// Encode a raster to a temporary path with the per-format options, inject the
/// document metadata into JPEG/TIFF, and rename over `path`. A failed encode or
/// injection removes the temporary and leaves the previous file untouched.
pub(in crate::cxxqt_object) fn write_raster_with_metadata(
    rgba: &[u8],
    width: u32,
    height: u32,
    path: &str,
    format: &str,
    metadata: &pictura_codec::RasterMetadata,
    options: &super::super::save_format::SaveRasterOptions,
) -> bool {
    let quality = if options.quality > 0 {
        options.quality
    } else {
        90
    };
    let tmp = format!("{path}.tmp-metadata");
    if !encode_raster(
        rgba,
        width,
        height,
        &tmp,
        format,
        quality,
        100,
        options.compression,
        options.optimized,
        options.progressive,
        options.matte,
    ) {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    if inject_and_rename(&tmp, path, format, metadata).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    true
}

pub(super) fn inject_and_rename(
    tmp: &str,
    path: &str,
    format: &str,
    metadata: &pictura_codec::RasterMetadata,
) -> std::io::Result<()> {
    let bytes = std::fs::read(tmp)?;
    let bytes = pictura_codec::embed_raster_metadata(&bytes, format, metadata)
        .map_err(|_| std::io::Error::other("metadata injection"))?;
    std::fs::write(tmp, bytes)?;
    std::fs::rename(tmp, path)
}

/// Rescale through the boundary (so the percent-scale contract matches the Qt
/// formats), encode losslessly in the engine, and write temp-file + rename.
pub(super) fn encode_webp_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    path: &str,
    scale: i32,
) -> bool {
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
    super::super::helpers::safe_io::write_atomically(path, &bytes, false).is_ok()
}
