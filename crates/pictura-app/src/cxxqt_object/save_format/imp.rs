use super::*;

use super::super::helpers_composite::buffer_to_rgba_bytes;
use super::super::picture::picture;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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
    let buffer = picture(doc, rust);
    let rgba = buffer_to_rgba_bytes(&buffer);
    let format = format.to_string();
    ffi::encode_image_bytes(
        &rgba,
        buffer.width as i32,
        buffer.height as i32,
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
}
