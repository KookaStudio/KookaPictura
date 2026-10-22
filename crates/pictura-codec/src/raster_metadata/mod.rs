//! Raster container metadata: preserve the document's EXIF, IPTC-IIM, and XMP
//! resources when Save As writes a JPEG or TIFF. PNG/BMP/WebP keep their
//! existing drop behaviour; this module only rewrites the two containers the
//! File Info oracle checks.

mod jpeg;
mod tiff;

use pictura_core::Document;

use crate::error::PsdError;
use crate::image_resources::{
    decode_image_resources, EXIF_DATA_1, EXIF_DATA_3, IPTC_NAA, XMP_METADATA,
};

/// The raw metadata blocks gathered from a document's image resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RasterMetadata {
    pub exif: Option<Vec<u8>>,
    pub iptc: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
}

impl RasterMetadata {
    pub fn is_empty(&self) -> bool {
        self.exif.is_none() && self.iptc.is_none() && self.xmp.is_none()
    }
}

/// Gather the first EXIF (1058/1059), IPTC-IIM (1028), and XMP (1060) resource
/// from `document`, in its preserved image-resource section.
pub fn raster_metadata(document: &Document) -> RasterMetadata {
    let mut metadata = RasterMetadata::default();
    for resource in decode_image_resources(document) {
        match resource.id {
            EXIF_DATA_1 | EXIF_DATA_3 if metadata.exif.is_none() && !resource.data.is_empty() => {
                metadata.exif = Some(resource.data);
            }
            IPTC_NAA if metadata.iptc.is_none() && !resource.data.is_empty() => {
                metadata.iptc = Some(resource.data);
            }
            XMP_METADATA if metadata.xmp.is_none() && !resource.data.is_empty() => {
                metadata.xmp = Some(resource.data);
            }
            _ => {}
        }
    }
    metadata
}

/// Embed `metadata` into an encoded JPEG/TIFF. Other formats and an empty
/// metadata set return the bytes unchanged.
pub fn embed_raster_metadata(
    bytes: &[u8],
    format: &str,
    metadata: &RasterMetadata,
) -> Result<Vec<u8>, PsdError> {
    if metadata.is_empty() {
        return Ok(bytes.to_vec());
    }
    if format.eq_ignore_ascii_case("jpg")
        || format.eq_ignore_ascii_case("jpeg")
        || format.eq_ignore_ascii_case("jpe")
    {
        jpeg::embed(bytes, metadata)
    } else if format.eq_ignore_ascii_case("tif") || format.eq_ignore_ascii_case("tiff") {
        tiff::embed(bytes, metadata)
    } else {
        Ok(bytes.to_vec())
    }
}
