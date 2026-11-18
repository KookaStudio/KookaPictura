use super::*;

use super::super::helpers::safe_io::reject_protected_destination;
use super::super::picture::picture_rgba;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

/// Encode packed RGBA8888 as a WebP container; empty on refusal.
///
/// The Export As dialog previews and writes per-entry bytes in memory, so this
/// exposes the engine encoder directly rather than the path-based save edge.
/// `lossless` or a negative `quality` selects the lossless VP8L encoder; any
/// other quality selects lossy VP8 on the 0-100 scale.
pub fn webp_encode(rgba: &[u8], width: i32, height: i32, quality: i32, lossless: bool) -> Vec<u8> {
    if width <= 0 || height <= 0 {
        return Vec::new();
    }
    let encoded = if lossless || quality < 0 {
        pictura_codec::encode_webp(rgba, width as u32, height as u32)
    } else {
        pictura_codec::encode_webp_lossy(
            rgba,
            width as u32,
            height as u32,
            quality.clamp(0, 100) as u8,
        )
    };
    encoded.unwrap_or_default()
}

/// Flatten the composite and write it as WebP: lossless VP8L when `lossless`
/// or `quality` is negative, otherwise lossy VP8. Writes atomically.
pub fn export_webp(view: &PictureView, path: &QString, quality: i32, lossless: bool) -> bool {
    let path = path.to_string();
    if reject_protected_destination(&path).is_err() {
        return false;
    }
    let rust = view.rust();
    if rust.session.read_only() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let rgba = picture_rgba(doc, rust, true);
    let encoded = if lossless || quality < 0 {
        pictura_codec::encode_webp(&rgba, doc.width, doc.height)
    } else {
        pictura_codec::encode_webp_lossy(&rgba, doc.width, doc.height, quality.clamp(0, 100) as u8)
    };
    let Some(bytes) = encoded else {
        return false;
    };
    super::super::helpers::safe_io::write_atomically(&path, &bytes, false).is_ok()
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
    if reject_protected_destination(&path).is_err() {
        return false;
    }
    let format = format.to_string();
    let rust = view.rust();
    if rust.session.read_only() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    let rgba = picture_rgba(doc, rust, true);
    encode_raster(
        &rgba, doc.width, doc.height, &path, &format, quality, scale, -1, false, false, -1,
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
    if reject_protected_destination(path).is_err() || reject_protected_destination(&tmp).is_err() {
        return false;
    }
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
        if reject_protected_destination(&tmp).is_ok() {
            let _ = std::fs::remove_file(&tmp);
        }
        return false;
    }
    if inject_and_rename(&tmp, path, format, metadata).is_err() {
        if reject_protected_destination(&tmp).is_ok() {
            let _ = std::fs::remove_file(&tmp);
        }
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
    reject_protected_destination(path)?;
    reject_protected_destination(tmp)?;
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
