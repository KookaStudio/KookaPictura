use super::*;

use super::super::picture::picture_rgba;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{ColorReduction, Dither, PaletteOptions};

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
    let pict = root.get("pict").cloned().unwrap_or_default();
    options.pict_resolution = int(&pict, "resolution", 32) as u8;
    options.pict_compression = int(&pict, "compression", 0) as u8;
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

/// The palette settings the engine GIF encoder uses for the next GIF save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct SaveGifOptions {
    /// 0 Exact, 1 Web, 2 Local Perceptual, 3 Local Selective, 4 Local Adaptive.
    pub(in crate::cxxqt_object) palette: i32,
    pub(in crate::cxxqt_object) colors: i32,
    /// 0 None, 1 Diffusion, 2 Pattern, 3 Noise.
    pub(in crate::cxxqt_object) dither: i32,
    pub(in crate::cxxqt_object) amount: i32,
    pub(in crate::cxxqt_object) interlaced: bool,
}

impl Default for SaveGifOptions {
    fn default() -> Self {
        Self {
            palette: 2,
            colors: 256,
            dither: 1,
            amount: 75,
            interlaced: false,
        }
    }
}

impl SaveGifOptions {
    /// The engine palette options for these dialog values. The GIF save and
    /// the GIF save preview both use this mapping so they cannot drift.
    pub(in crate::cxxqt_object) fn palette_options(self) -> PaletteOptions {
        let reduction = match self.palette {
            0 => ColorReduction::Adaptive,
            1 => ColorReduction::Restrictive,
            2 => ColorReduction::Perceptual,
            3 => ColorReduction::Selective,
            _ => ColorReduction::Adaptive,
        };
        let colors = match self.palette {
            0 => 256,
            1 => 216,
            _ => self.colors,
        };
        let dither = match (self.palette, self.dither) {
            (0, _) => Dither::None,
            (_, 1) => Dither::Diffusion,
            (_, 2) => Dither::Pattern,
            (_, 3) => Dither::Noise,
            _ => Dither::None,
        };
        PaletteOptions {
            reduction,
            colors: colors.clamp(2, 256) as u16,
            dither,
            amount: self.amount.clamp(0, 100) as u8,
            transparency: false,
            mapped_transparency: false,
            matte: [255, 255, 255],
        }
    }
}

pub fn set_save_gif_options(
    mut view: Pin<&mut PictureView>,
    palette: i32,
    colors: i32,
    dither: i32,
    amount: i32,
    interlaced: bool,
) {
    view.as_mut().rust_mut().save_gif_options = Some(SaveGifOptions {
        palette,
        colors,
        dither,
        amount,
        interlaced,
    });
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

/// The catalog ordinal a typed suffix names, or -1. Alias-only, so `.dcs` stays
/// unknown.
pub fn save_format_id_for_suffix(suffix: &str) -> i32 {
    pictura_codec::SaveFormat::from_suffix(suffix).map_or(-1, pictura_codec::SaveFormat::id)
}

/// The catalog ordinal a remembered source name or suffix names, or -1. Unlike
/// the suffix lookup, this accepts identity names, so `"dcs"` finds DCS.
pub fn save_format_id_for_source_name(name: &str) -> i32 {
    pictura_codec::SaveFormat::from_source_name(name).map_or(-1, pictura_codec::SaveFormat::id)
}

/// The policy flags for a catalog ordinal; 0 when the id is unknown.
pub fn save_format_flags(id: i32) -> u32 {
    pictura_codec::SaveFormat::from_id(id).map_or(0, pictura_codec::SaveFormat::flags)
}

/// The options wire code for a catalog ordinal; 0 when it opens no dialog.
pub fn save_format_code(id: i32) -> i32 {
    pictura_codec::SaveFormat::from_id(id).map_or(0, pictura_codec::SaveFormat::code)
}

/// The canonical extension for a catalog ordinal; empty when unknown.
pub fn save_format_canonical(id: i32) -> QString {
    pictura_codec::SaveFormat::from_id(id).map_or(QString::from(""), |format| {
        QString::from(format.canonical())
    })
}

/// The lowercase aliases for a catalog ordinal as one space-separated string.
pub fn save_format_aliases(id: i32) -> QString {
    pictura_codec::SaveFormat::from_id(id).map_or(QString::from(""), |format| {
        QString::from(format.aliases_text().as_str())
    })
}

/// Save `path` with a format the shell resolved. `format_id` is a catalog
/// ordinal; `-1` resolves from the path. `copy` skips the path and modified
/// bookkeeping.
pub fn save_document_format(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    format_id: i32,
    copy: bool,
) -> bool {
    view.as_mut().save_as_format(path, format_id, copy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_codec::{SaveFormat, FLAG_KEEPS_LAYERS};

    #[test]
    fn save_format_queries_match_the_catalog() {
        assert_eq!(save_format_id_for_suffix("PNG"), SaveFormat::Png.id());
        assert_eq!(save_format_id_for_suffix("dcs"), -1);
        assert_eq!(save_format_id_for_source_name("dcs"), SaveFormat::Dcs.id());
        assert_eq!(
            save_format_flags(SaveFormat::Tiff.id()) & FLAG_KEEPS_LAYERS,
            FLAG_KEEPS_LAYERS
        );
        assert_eq!(save_format_code(SaveFormat::Jpeg.id()), 1);
        assert_eq!(
            save_format_canonical(SaveFormat::Tiff.id()).to_string(),
            ".tif"
        );
        assert_eq!(
            save_format_aliases(SaveFormat::Jpeg.id()).to_string(),
            "jpg jpeg jpe"
        );
        assert_eq!(save_format_id_for_suffix("xyz"), -1);
        assert_eq!(save_format_id_for_source_name("xyz"), -1);
        assert_eq!(save_format_flags(-1), 0);
        assert_eq!(save_format_code(-1), 0);
    }

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
