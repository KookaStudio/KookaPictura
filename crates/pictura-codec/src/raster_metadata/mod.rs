//! Raster container metadata: preserve the document's EXIF, IPTC-IIM, XMP and
//! Vanishing Point resources when Save As writes a JPEG or TIFF, and read the
//! Photoshop resource block back when one is opened. PNG/BMP/WebP keep their
//! existing drop behaviour; this module only rewrites the two containers the
//! File Info oracle checks.

mod jpeg;
mod tiff;

use pictura_core::Document;

use crate::error::PsdError;
use crate::image_resources::{
    decode_image_resources, decode_section, encode_image_resources, frame_image_resource,
    vanishing_point_data, EXIF_DATA_1, EXIF_DATA_3, ICC_PROFILE, IPTC_NAA, VANISHING_POINT,
    VANISHING_POINT_NAME, XMP_METADATA,
};

/// The raw metadata blocks gathered from a document's image resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RasterMetadata {
    pub exif: Option<Vec<u8>>,
    pub iptc: Option<Vec<u8>>,
    pub xmp: Option<Vec<u8>>,
    /// The planes resource's data, for the Photoshop resource block.
    pub vanishing_point: Option<Vec<u8>>,
}

impl RasterMetadata {
    pub fn is_empty(&self) -> bool {
        self.exif.is_none()
            && self.iptc.is_none()
            && self.xmp.is_none()
            && self.vanishing_point.is_none()
    }

    /// The image-resource section a JPEG's APP13 or a TIFF's Photoshop tag
    /// carries: IPTC-IIM, then the planes. `None` when it holds neither.
    fn photoshop_block(&self) -> Option<Vec<u8>> {
        let mut resources = Vec::new();
        if let Some(iptc) = &self.iptc {
            resources.push(frame_image_resource(IPTC_NAA, "", iptc));
        }
        if let Some(planes) = &self.vanishing_point {
            resources.push(frame_image_resource(
                VANISHING_POINT,
                VANISHING_POINT_NAME,
                planes,
            ));
        }
        (!resources.is_empty()).then(|| encode_image_resources(&resources))
    }
}

/// Gather the first EXIF (1058/1059), IPTC-IIM (1028), and XMP (1060) resource
/// and the planes resource from `document`, in its preserved image-resource
/// section.
pub fn raster_metadata(document: &Document) -> RasterMetadata {
    let mut metadata = RasterMetadata {
        vanishing_point: vanishing_point_data(document),
        ..RasterMetadata::default()
    };
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

/// The Photoshop image-resource section a JPEG (APP13) or TIFF (tag 34377)
/// carries, for a document opened from it; `None` for another format or a
/// file without one. An ICC profile block is left out: the opened pixels
/// are not converted from it, so it would mis-tag them. Blocks the parser
/// cannot frame stay byte-for-byte at the end.
pub fn raster_image_resources(bytes: &[u8]) -> Option<Vec<u8>> {
    let section = if bytes.starts_with(&[0xFF, 0xD8]) {
        jpeg::photoshop_block(bytes)?
    } else {
        tiff::photoshop_block(bytes)?
    };
    let (resources, consumed) = decode_section(&section);
    if !resources.iter().any(|r| r.id == ICC_PROFILE) {
        return Some(section);
    }
    let kept: Vec<_> = resources
        .into_iter()
        .filter(|r| r.id != ICC_PROFILE)
        .collect();
    let mut out = encode_image_resources(&kept);
    out.extend_from_slice(&section[consumed..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_open_reads_the_block_without_its_profile() {
        let planes = frame_image_resource(VANISHING_POINT, VANISHING_POINT_NAME, b"planes");
        let icc = frame_image_resource(ICC_PROFILE, "", b"profile");
        let iptc = frame_image_resource(IPTC_NAA, "", b"iptc");
        let block = encode_image_resources(&[icc, planes.clone(), iptc.clone()]);
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xED];
        jpeg.extend_from_slice(&((block.len() + 16) as u16).to_be_bytes());
        jpeg.extend_from_slice(b"Photoshop 3.0\0");
        jpeg.extend_from_slice(&block);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        assert_eq!(
            raster_image_resources(&jpeg),
            Some(encode_image_resources(&[planes.clone(), iptc.clone()]))
        );

        let metadata = RasterMetadata {
            iptc: Some(b"iptc".to_vec()),
            vanishing_point: Some(b"planes".to_vec()),
            ..RasterMetadata::default()
        };
        let mut tiff = vec![b'I', b'I', 42, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        tiff = embed_raster_metadata(&tiff, "tif", &metadata).expect("embed");
        assert_eq!(
            raster_image_resources(&tiff),
            Some(encode_image_resources(&[iptc, planes]))
        );
        assert_eq!(raster_image_resources(b"\x89PNG\r\n\x1a\n"), None);
    }
}
