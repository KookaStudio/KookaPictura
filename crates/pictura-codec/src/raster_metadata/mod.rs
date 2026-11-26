//! Raster container metadata: preserve the document's EXIF, IPTC-IIM, XMP and
//! Vanishing Point resources when Save As writes a JPEG or TIFF, and read the
//! Photoshop resource block back when one is opened. PNG/BMP/WebP keep their
//! existing drop behaviour; this module only rewrites the two containers the
//! File Info oracle checks.

mod jpeg;
mod tiff;

pub const JPEG_XMP_MAX_BYTES: usize = jpeg::XMP_MAX_BYTES;

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
    pub hdr_preview: Option<crate::image_resources::ImageResource>,
    pub tiff_resources: Vec<crate::image_resources::ImageResource>,
    pub xmp: Option<Vec<u8>>,
    /// The planes resource's data, for the Photoshop resource block.
    pub vanishing_point: Option<Vec<u8>>,
    /// An RGB working profile to embed. `None` writes no profile.
    pub icc: Option<Vec<u8>>,
}

impl RasterMetadata {
    pub fn is_empty(&self) -> bool {
        self.exif.is_none()
            && self.iptc.is_none()
            && self.hdr_preview.is_none()
            && self.tiff_resources.is_empty()
            && self.xmp.is_none()
            && self.vanishing_point.is_none()
            && self.icc.is_none()
    }

    /// The image-resource section a JPEG's APP13 or a TIFF's Photoshop tag
    /// carries: IPTC-IIM, then the planes. `None` when it holds neither.
    fn photoshop_block(&self, tiff: bool) -> Option<Vec<u8>> {
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
        if tiff {
            resources.extend(self.hdr_preview.clone());
            resources.extend(self.tiff_resources.iter().cloned());
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
        if crate::hdr_preview::is_preview(&resource) && metadata.hdr_preview.is_none() {
            metadata.hdr_preview = Some(resource);
            continue;
        }
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
            ICC_PROFILE | EXIF_DATA_1 | EXIF_DATA_3 | IPTC_NAA | XMP_METADATA | VANISHING_POINT => {
            }
            _ => metadata.tiff_resources.push(resource),
        }
    }
    // Only carry opaque Photoshop resources alongside explicit HDR preview settings.
    if metadata.hdr_preview.is_none() {
        metadata.tiff_resources.clear();
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
        let mut jpeg_metadata = metadata.clone();
        jpeg_metadata.hdr_preview = None;
        jpeg_metadata.tiff_resources.clear();
        if jpeg_metadata.is_empty() {
            Ok(bytes.to_vec())
        } else {
            jpeg::embed(bytes, &jpeg_metadata)
        }
    } else if format.eq_ignore_ascii_case("tif") || format.eq_ignore_ascii_case("tiff") {
        tiff::embed(bytes, metadata)
    } else {
        Ok(bytes.to_vec())
    }
}

/// For a streamed TIFF of `len` bytes whose IFD0 table is `ifd`: the bytes
/// [`embed_raster_metadata`] would append and the new IFD0 offset for the
/// header, or `None` when there is no metadata to embed (#1320).
pub(crate) fn embed_tiff_tail(
    len: u64,
    big: bool,
    ifd: &[u8],
    metadata: &RasterMetadata,
) -> Result<Option<(Vec<u8>, u32)>, PsdError> {
    if metadata.is_empty() {
        return Ok(None);
    }
    tiff::embed_tail(len, big, ifd, metadata).map(Some)
}

fn valid_xmp(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() <= crate::xmp::MAX_PACKET_BYTES
        && std::str::from_utf8(bytes).is_ok_and(crate::xmp::is_well_formed)
}

/// Effective photographic exposure, relative to ISO 100, from JPEG or TIFF.
pub fn raster_ev100(bytes: &[u8]) -> Option<f64> {
    let tiff = if bytes.starts_with(&[0xff, 0xd8]) {
        jpeg::resources(bytes)?.exif?
    } else {
        bytes
    };
    let exif = crate::parse_exif(tiff);
    let read = |tag| exif.get(tag)?.as_f64().filter(|&number| number > 0.0);
    let aperture = read(0x829d)?;
    let duration = read(0x829a)?;
    let iso = read(0x8827)?;
    let ev100 = (aperture * aperture / duration * (100.0 / iso)).log2();
    ev100.is_finite().then_some(ev100)
}

/// JPEG/TIFF Photoshop resources with the standard container XMP packet in
/// resource 1060. The imported pixels have their own profile policy, so an ICC
/// resource in the Photoshop block is left out. Unparsed bytes stay at the end.
pub fn raster_image_resources(bytes: &[u8]) -> Option<Vec<u8>> {
    let (section, xmp) = if bytes.starts_with(&[0xFF, 0xD8]) {
        let resources = jpeg::resources(bytes)?;
        (resources.photoshop, resources.xmp.map(<[u8]>::to_vec))
    } else {
        (tiff::photoshop_block(bytes), tiff::xmp_packet(bytes))
    };
    if section.is_none() && xmp.is_none() {
        return None;
    }
    let section = section.unwrap_or_default();
    let (resources, consumed) = decode_section(&section);
    if xmp.is_none() && !resources.iter().any(|r| r.id == ICC_PROFILE) {
        return Some(section);
    }
    let mut kept: Vec<_> = resources
        .into_iter()
        .filter(|r| r.id != ICC_PROFILE && (xmp.is_none() || r.id != XMP_METADATA))
        .collect();
    if let Some(packet) = xmp {
        kept.push(frame_image_resource(XMP_METADATA, "", &packet));
    }
    let mut out = encode_image_resources(&kept);
    out.extend_from_slice(&section[consumed..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_ev100_reads_shutter_aperture_and_iso() {
        let mut tiff = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&26u32.to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        for (tag, format, value) in [(0x829a_u16, 5u16, 68u32), (0x829d, 5, 76), (0x8827, 3, 200)] {
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&format.to_le_bytes());
            tiff.extend_from_slice(&1u32.to_le_bytes());
            tiff.extend_from_slice(&value.to_le_bytes());
        }
        tiff.extend_from_slice(&0u32.to_le_bytes());
        for number in [1u32, 8, 4, 1] {
            tiff.extend_from_slice(&number.to_le_bytes());
        }
        assert_eq!(raster_ev100(&tiff), Some(6.0));

        let jpeg = jpeg::embed(
            b"\xff\xd8\xff\xd9",
            &RasterMetadata { exif: Some(tiff.clone()), ..RasterMetadata::default() },
        ).unwrap();
        assert_eq!(raster_ev100(&jpeg), Some(6.0));
        assert_eq!(raster_ev100(&tiff[..68]), None);
    }

    #[test]
    fn standard_raster_xmp_round_trips() {
        let packet = b"<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description xmlns:p=\"http://ns.adobe.com/photoshop/1.0/\"><p:History>first edit</p:History></rdf:Description></rdf:RDF>";
        let metadata = RasterMetadata {
            xmp: Some(packet.to_vec()),
            ..RasterMetadata::default()
        };
        for (format, image) in [
            ("jpg", &b"\xff\xd8\xff\xd9"[..]),
            ("tif", &b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0"[..]),
            ("tif", &b"MM\0\x2a\0\0\0\x08\0\0\0\0\0\0"[..]),
        ] {
            let written = embed_raster_metadata(image, format, &metadata).unwrap();
            let section = raster_image_resources(&written).unwrap();
            let (resources, _) = decode_section(&section);
            assert_eq!(resources.len(), 1);
            assert_eq!(resources[0].id, XMP_METADATA);
            assert_eq!(resources[0].data.as_slice(), packet);
        }
    }

    #[test]
    fn tiff_preview_resource_round_trip() {
        use crate::hdr_preview::{get, set, PreviewOptions};

        let options = PreviewOptions::new(1, -2.5, 1.25).unwrap();
        let mut doc = Document::new(
            1,
            1,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::ThirtyTwo,
        );
        let iptc = frame_image_resource(IPTC_NAA, "", b"iptc");
        let opaque = frame_image_resource(4200, "opaque", b"preserve me");
        doc.image_resources = encode_image_resources(&[iptc.clone(), opaque.clone()]).into();
        set(&mut doc, options);
        let metadata = raster_metadata(&doc);
        let tiff = b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0";
        let saved = embed_raster_metadata(tiff, "tiff", &metadata).unwrap();
        doc.image_resources = raster_image_resources(&saved).unwrap().into();
        assert_eq!(get(&doc), options);
        assert!(decode_image_resources(&doc).contains(&iptc));
        assert!(decode_image_resources(&doc).contains(&opaque));
        let jpeg = embed_raster_metadata(&[0xff, 0xd8, 0xff, 0xd9], "jpeg", &metadata).unwrap();
        let jpeg_resources = raster_image_resources(&jpeg).unwrap_or_default();
        let (resources, _) = decode_section(&jpeg_resources);
        assert!(!resources.iter().any(crate::hdr_preview::is_preview));
        assert!(!resources.contains(&opaque));
    }

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
