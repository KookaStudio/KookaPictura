use super::*;

use super::super::picture::picture_rgba;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

/// The typed Save As kind and options accepted by the dialogs.
#[derive(Clone, Debug, Default)]
pub(in crate::cxxqt_object) struct SaveSettings {
    pub kind: i32,
    pub options: pictura_codec::SaveOptions,
}

fn int(value: &serde_json::Value, key: &str, default: i32) -> i32 {
    value
        .get(key)
        .and_then(|v| v.as_i64())
        .map_or(default, |v| v as i32)
}

fn boolean(value: &serde_json::Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn parse_save_options(kind: i32, json: &str) -> SaveSettings {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json) else {
        return SaveSettings {
            kind,
            ..Default::default()
        };
    };
    let mut options = pictura_codec::SaveOptions::default().with_defaults();
    let png = root.get("png").cloned().unwrap_or_default();
    options.png_compression = (int(&png, "compression", 0) as u32 * 9 / 100) as u8;
    options.png_interlace = int(&png, "interlace", 0) != 0;
    let bmp = root.get("bmp").cloned().unwrap_or_default();
    options.bmp_os2 = int(&bmp, "format", 0) == 1;
    options.bmp_depth = int(&bmp, "depth", 24) as u8;
    options.bmp_rle = boolean(&bmp, "rle", false);
    options.bmp_flip = boolean(&bmp, "flip", false);
    options.bmp_advanced = int(&bmp, "advanced", 0) as u8;
    let tiff = root.get("tiff").cloned().unwrap_or_default();
    options.tiff_compression = int(&tiff, "compression", 2) as u8;
    options.tiff_jpeg_quality = int(&tiff, "jpegQuality", 10) as u8;
    options.tiff_pixel_order = int(&tiff, "pixelOrder", 0) != 0;
    options.tiff_byte_order_mac = int(&tiff, "byteOrder", 0) != 0;
    options.tiff_layer_compression = int(&tiff, "layerCompression", 0) as u8;
    options.tiff_pyramid = boolean(&tiff, "pyramid", false);
    options.tiff_transparency = boolean(&tiff, "transparency", true);
    let gif = root.get("gif").cloned().unwrap_or_default();
    options.gif_interlaced = int(&gif, "rowOrder", 0) != 0;
    let tga = root.get("tga").cloned().unwrap_or_default();
    options.tga_depth = int(&tga, "depth", 24) as u8;
    options.tga_rle = boolean(&tga, "rle", false);
    let raw = root.get("raw").cloned().unwrap_or_default();
    options.raw_header = int(&raw, "header", 0).max(0) as u32;
    options.raw_interleaved = int(&raw, "interleave", 0) == 0;
    let cineon = root.get("cineon").cloned().unwrap_or_default();
    options.cineon_bits = int(&cineon, "bits", 10) as u8;
    let eps = root.get("eps").cloned().unwrap_or_default();
    options.eps_preview = int(&eps, "preview", 2) as u8;
    options.eps_encoding = int(&eps, "encoding", 2) as u8;
    options.eps_jpeg_quality = int(&eps, "jpegQuality", 10) as u8;
    options.eps_halftone = boolean(&eps, "halftone", false);
    options.eps_transfer = boolean(&eps, "transfer", false);
    options.eps_transparent_whites = boolean(&eps, "transparentWhites", false);
    options.eps_ps_color_management = boolean(&eps, "psColorManagement", false);
    options.eps_vector_data = boolean(&eps, "vectorData", false);
    options.eps_interpolation = boolean(&eps, "interpolation", true);
    let dcs = root.get("dcs").cloned().unwrap_or_default();
    options.dcs_version = int(&dcs, "version", 2) as u8;
    options.dcs_preview = int(&dcs, "preview", 2) as u8;
    SaveSettings {
        kind,
        options: options.with_defaults(),
    }
}

pub fn set_save_options(mut view: Pin<&mut PictureView>, kind: i32, json: &str) {
    view.as_mut().rust_mut().save_options = Some(parse_save_options(kind, json));
}

/// The options the Qt-boundary encoder honors for the next raster save. `-1`
/// means "use the format default".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct SaveRasterOptions {
    /// JPEG/TIFF quality on Qt's 0-100 scale, already mapped from CS6's 0-12.
    pub(in crate::cxxqt_object) quality: i32,
    /// PNG zlib level (0-9) or the Qt TIFF compression enum (0/1/5).
    pub(in crate::cxxqt_object) compression: i32,
    pub(in crate::cxxqt_object) optimized: bool,
    pub(in crate::cxxqt_object) progressive: bool,
    /// A matte colour as `0xRRGGBB`, or -1 to keep alpha (and flatten JPEG/BMP
    /// onto opaque white).
    pub(in crate::cxxqt_object) matte: i32,
}

impl Default for SaveRasterOptions {
    fn default() -> Self {
        Self {
            quality: -1,
            compression: -1,
            optimized: false,
            progressive: false,
            matte: -1,
        }
    }
}

pub fn set_save_raster_options(
    mut view: Pin<&mut PictureView>,
    quality: i32,
    compression: i32,
    optimized: bool,
    progressive: bool,
    matte: i32,
) {
    view.as_mut().rust_mut().save_raster_options = Some(SaveRasterOptions {
        quality,
        compression,
        optimized,
        progressive,
        matte,
    });
}

pub fn raster_preview_bytes(
    view: &PictureView,
    format: &QString,
    quality: i32,
    compression: i32,
    optimized: bool,
    progressive: bool,
    matte: i32,
) -> Vec<u8> {
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let rgba = picture_rgba(doc, rust, false);
    let format = format.to_string();
    super::super::export::ffi::encode_image_bytes(
        &rgba,
        doc.width as i32,
        doc.height as i32,
        &format,
        quality,
        100,
        compression,
        optimized,
        progressive,
        matte,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_are_unset() {
        let options = SaveRasterOptions::default();
        assert_eq!(options.quality, -1);
        assert_eq!(options.compression, -1);
        assert!(!options.optimized);
        assert!(!options.progressive);
        assert_eq!(options.matte, -1);
    }

    #[test]
    fn typed_save_options_parse_the_dialog_json() {
        let json = r#"{"png":{"compression":100,"interlace":1},"bmp":{"depth":8,"rle":true},"tiff":{"compression":3,"pixelOrder":1},"gif":{"rowOrder":1},"eps":{"preview":1,"encoding":0},"dcs":{"version":1},"raw":{"header":16,"interleave":1},"tga":{"depth":32,"rle":true},"cineon":{"bits":10}}"#;
        let settings = parse_save_options(10, json);
        assert_eq!(settings.options.png_compression, 9);
        assert!(settings.options.png_interlace);
        assert_eq!(settings.options.bmp_depth, 8);
        assert_eq!(settings.options.tiff_compression, 3);
        assert!(settings.options.gif_interlaced);
        assert_eq!(settings.options.eps_preview, 1);
        assert_eq!(settings.options.dcs_version, 1);
        assert_eq!(settings.options.raw_header, 16);
        assert_eq!(settings.options.tga_depth, 32);
    }
}
