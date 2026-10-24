//! Image-resource section parser.
//!
//! The section is preserved verbatim by the reader and re-emitted byte-for-byte
//! by the writer; this module derives a typed view (id, name, data) so callers
//! can reach the embedded ICC profile, XMP packet, EXIF, and IPTC without
//! re-parsing the block framing. The raw bytes stay the source of truth.
//!
//! ponytail: an unrecognized signature ends parsing (the accepted set is the
//! psd-tools one); add a signature when a real file needs it.

use pictura_core::Document;

use crate::common::Reader;

/// The ICC profile resource id.
pub const ICC_PROFILE: u16 = 1039;
/// The XMP metadata resource id.
pub const XMP_METADATA: u16 = 1060;
/// The first EXIF resource id.
pub const EXIF_DATA_1: u16 = 1058;
/// The third EXIF resource id.
pub const EXIF_DATA_3: u16 = 1059;
/// The IPTC-NAA resource id.
pub const IPTC_NAA: u16 = 1028;
/// The ResolutionInfo resource id.
pub const RESOLUTION_INFO: u16 = 1005;

/// The project's Vanishing Point planes resource id, in Photoshop's plug-in
/// range. A block counts as ours only with [`VANISHING_POINT_NAME`] too, so
/// another plug-in's block at this id is left alone.
// ponytail: CS6's own plane block is undocumented; this one is project-owned
// and Photoshop does not read it.
pub const VANISHING_POINT: u16 = 4900;
/// The Pascal name that marks the planes resource.
pub const VANISHING_POINT_NAME: &str = "Kooka Vanishing Point";

fn is_vanishing_point(resource: &ImageResource) -> bool {
    resource.id == VANISHING_POINT && resource.name == VANISHING_POINT_NAME
}

/// The data of the document's planes resource, if it has one.
pub fn vanishing_point_data(document: &Document) -> Option<Vec<u8>> {
    decode_image_resources(document)
        .into_iter()
        .find(is_vanishing_point)
        .map(|r| r.data)
}

/// Replace the document's planes resource with `data`, append one, or with
/// `None` remove it. Every other block and any unparsed trailing bytes are
/// kept as they were.
pub fn set_vanishing_point_data(document: &mut Document, data: Option<&[u8]>) {
    let (mut resources, consumed) = decode_image_resources_with_len(document);
    let at = resources.iter().position(is_vanishing_point);
    match (at, data) {
        (Some(i), Some(data)) => {
            resources[i] = frame_image_resource(VANISHING_POINT, VANISHING_POINT_NAME, data)
        }
        (Some(i), None) => {
            resources.remove(i);
        }
        (None, Some(data)) => resources.push(frame_image_resource(
            VANISHING_POINT,
            VANISHING_POINT_NAME,
            data,
        )),
        (None, None) => return,
    }
    let mut section = encode_image_resources(&resources);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section;
}

/// The document's horizontal resolution from its ResolutionInfo resource.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolution {
    /// Pixels per inch; the file stores ppi whatever the display unit.
    pub ppi: f64,
    /// The unit the user picked: true for pixels/cm, false for pixels/inch.
    pub per_cm: bool,
}

/// The document's resolution, or `None` when the ResolutionInfo resource is
/// absent or malformed (callers fall back to CS6's 72 ppi).
pub fn document_resolution(document: &Document) -> Option<Resolution> {
    let resource = decode_image_resources(document)
        .into_iter()
        .find(|r| r.id == RESOLUTION_INFO)?;
    let data = resource.data.get(..6)?;
    // hRes is 16.16 fixed point, then the hResUnit display unit.
    let fixed = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let ppi = f64::from(fixed) / 65536.0;
    (ppi > 0.0).then_some(Resolution {
        ppi,
        per_cm: u16::from_be_bytes([data[4], data[5]]) == 2,
    })
}

/// Write `resolution` into the document's ResolutionInfo resource (both
/// axes), replacing it or appending one. The width/height display units an
/// existing resource names are kept; a new one shows inches, or centimeters
/// for a pixels/cm resolution. Unparsed trailing bytes are preserved.
pub fn set_document_resolution(document: &mut Document, resolution: Resolution) {
    let (mut resources, consumed) = decode_image_resources_with_len(document);
    let unit: u16 = if resolution.per_cm { 2 } else { 1 };
    let existing = resources
        .iter()
        .find(|r| r.id == RESOLUTION_INFO)
        .filter(|r| r.data.len() >= 16);
    let size_unit =
        |at: usize| existing.map_or(unit, |r| u16::from_be_bytes([r.data[at], r.data[at + 1]]));
    let (width_unit, height_unit) = (size_unit(6), size_unit(14));
    let fixed = ((resolution.ppi * 65536.0)
        .round()
        .clamp(1.0, u32::MAX as f64) as u32)
        .to_be_bytes();
    let mut data = Vec::with_capacity(16);
    for size in [width_unit, height_unit] {
        data.extend_from_slice(&fixed);
        data.extend_from_slice(&unit.to_be_bytes());
        data.extend_from_slice(&size.to_be_bytes());
    }
    let block = frame_image_resource(RESOLUTION_INFO, "", &data);
    match resources.iter_mut().find(|r| r.id == RESOLUTION_INFO) {
        Some(slot) => *slot = block,
        None => resources.push(block),
    }
    let mut section = encode_image_resources(&resources);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section;
}

/// One image-resource block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageResource {
    pub id: u16,
    pub name: String,
    pub data: Vec<u8>,
    /// The exact on-disk bytes of the whole block (signature through padded
    /// data), so the section can be re-emitted or rewritten byte-for-byte.
    pub raw: Vec<u8>,
}

/// Signatures psd-tools accepts for an image-resource block; each uses the same
/// `signature id name size data` framing.
const SIGNATURES: [[u8; 4]; 6] = [*b"8BIM", *b"8B64", *b"MeSa", *b"AgHg", *b"PHUT", *b"DCSR"];

/// Parse the document's preserved image-resource section into typed records in
/// stored order. A truncated block or an unrecognized signature ends parsing and
/// the records decoded so far are returned; never panics.
pub fn decode_image_resources(document: &Document) -> Vec<ImageResource> {
    decode_image_resources_with_len(document).0
}

/// As [`decode_image_resources`], plus the number of section bytes the parsed
/// records consumed. The remainder is unparsed (an unknown signature or a
/// truncated block) and must be preserved byte-for-byte when the section is
/// rewritten.
pub fn decode_image_resources_with_len(document: &Document) -> (Vec<ImageResource>, usize) {
    decode_section(&document.image_resources)
}

/// [`decode_image_resources_with_len`] over a bare image-resource section.
pub(crate) fn decode_section(section: &[u8]) -> (Vec<ImageResource>, usize) {
    let mut reader = Reader::new(section);
    let mut out = Vec::new();
    let mut consumed = 0;
    while reader.remaining() >= 4 {
        let start = reader.pos;
        let Ok(signature) = reader.take(4) else {
            break;
        };
        if !SIGNATURES.iter().any(|s| s.as_slice() == signature) {
            break;
        }
        let Ok(id) = reader.u16() else {
            break;
        };
        let Ok(name_len) = reader.u8() else {
            break;
        };
        let name_len = name_len as usize;
        let Ok(raw_name) = reader.take(name_len) else {
            break;
        };
        if !(name_len + 1).is_multiple_of(2) && reader.take(1).is_err() {
            break;
        }
        let name = String::from_utf8_lossy(raw_name)
            .trim_end_matches('\0')
            .to_string();
        let Ok(size) = reader.u32() else {
            break;
        };
        let size = size as usize;
        let Ok(data) = reader.take(size) else {
            break;
        };
        if !size.is_multiple_of(2) && reader.take(1).is_err() {
            break;
        }
        out.push(ImageResource {
            id,
            name,
            data: data.to_vec(),
            raw: reader.data[start..reader.pos].to_vec(),
        });
        consumed = reader.pos;
    }
    (out, consumed)
}

/// Re-serialize typed records into an image-resource section by concatenating
/// each record's raw block bytes. `encode_image_resources(&decode_image_resources(doc))`
/// reproduces `doc.image_resources`.
pub fn encode_image_resources(resources: &[ImageResource]) -> Vec<u8> {
    let mut out = Vec::new();
    for resource in resources {
        out.extend_from_slice(&resource.raw);
    }
    out
}

/// Frame a single block from typed fields — the inverse of the parser — so a
/// resource can be replaced or appended and re-emitted with
/// [`encode_image_resources`]. The signature is always `8BIM`.
pub fn frame_image_resource(id: u16, name: &str, data: &[u8]) -> ImageResource {
    // ponytail: a Pascal name length is a u8, so a longer name is truncated; the
    // stored `name` is the truncated form the parser will read back.
    let full = name.as_bytes();
    let name_bytes = &full[..full.len().min(u8::MAX as usize)];
    let mut raw = Vec::with_capacity(14 + name_bytes.len() + data.len());
    raw.extend_from_slice(b"8BIM");
    raw.extend_from_slice(&id.to_be_bytes());
    raw.push(name_bytes.len() as u8);
    raw.extend_from_slice(name_bytes);
    if !(name_bytes.len() + 1).is_multiple_of(2) {
        raw.push(0);
    }
    raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
    raw.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        raw.push(0);
    }
    ImageResource {
        id,
        name: String::from_utf8_lossy(name_bytes).into_owned(),
        data: data.to_vec(),
        raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    fn resource(signature: &[u8; 4], id: u16, name: &str, data: &[u8]) -> Vec<u8> {
        let mut out = signature.to_vec();
        out.extend_from_slice(&id.to_be_bytes());
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
        if !(name.len() + 1).is_multiple_of(2) {
            out.push(0);
        }
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            out.push(0);
        }
        out
    }

    fn document_with(section: Vec<u8>) -> Document {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources = section;
        doc
    }

    #[test]
    fn empty_section_is_empty() {
        assert_eq!(
            decode_image_resources(&document_with(Vec::new())),
            Vec::new()
        );
    }

    #[test]
    fn decodes_resources_in_order_with_odd_padding() {
        let first = resource(b"8BIM", ICC_PROFILE, "ICC", b"profile-bytes");
        let mut section = first.clone();
        section.extend(resource(b"8BIM", XMP_METADATA, "xmp", b"<x/>"));
        let decoded = decode_image_resources(&document_with(section.clone()));
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].id, ICC_PROFILE);
        assert_eq!(decoded[0].name, "ICC");
        assert_eq!(decoded[0].data, b"profile-bytes");
        assert_eq!(decoded[0].raw, first, "the raw block is retained");
        assert_eq!(decoded[1].id, XMP_METADATA);
        assert_eq!(decoded[1].data, b"<x/>");
        assert_eq!(
            encode_image_resources(&decoded),
            section,
            "decode then encode reproduces the section"
        );
    }

    #[test]
    fn accepts_alternate_signatures() {
        let section = resource(b"MeSa", IPTC_NAA, "iptc", b"data");
        let decoded = decode_image_resources(&document_with(section));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].id, IPTC_NAA);
    }

    #[test]
    fn truncation_returns_prior_records_and_never_panics() {
        let mut section = resource(b"8BIM", EXIF_DATA_1, "exif", b"abc");
        section.extend_from_slice(b"8BIM"); // header cut off mid-block
        let decoded = decode_image_resources(&document_with(section));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].id, EXIF_DATA_1);

        // Every prefix of a valid section must be panic-free.
        let full = resource(b"8BIM", ICC_PROFILE, "ICC", b"profile");
        for cut in 0..full.len() {
            let _ = decode_image_resources(&document_with(full[..cut].to_vec()));
        }
    }

    #[test]
    fn unrecognized_signature_stops() {
        let mut section = resource(b"8BIM", ICC_PROFILE, "ICC", b"a");
        section.extend_from_slice(b"nope");
        let decoded = decode_image_resources(&document_with(section));
        assert_eq!(decoded.len(), 1);
    }

    #[test]
    fn reads_resolution_info() {
        let mut data = (300u32 << 16).to_be_bytes().to_vec();
        data.extend_from_slice(&2u16.to_be_bytes());
        data.extend_from_slice(&[0; 10]);
        let section = resource(b"8BIM", RESOLUTION_INFO, "", &data);
        assert_eq!(
            document_resolution(&document_with(section)),
            Some(Resolution {
                ppi: 300.0,
                per_cm: true
            })
        );
        assert_eq!(document_resolution(&document_with(Vec::new())), None);
        let short = resource(b"8BIM", RESOLUTION_INFO, "", &[0, 72]);
        assert_eq!(document_resolution(&document_with(short)), None);
    }

    #[test]
    fn writes_resolution_info_and_keeps_the_size_units() {
        let mut doc = document_with(resource(b"8BIM", ICC_PROFILE, "ICC", b"a"));
        set_document_resolution(
            &mut doc,
            Resolution {
                ppi: 300.0,
                per_cm: false,
            },
        );
        assert_eq!(
            document_resolution(&doc),
            Some(Resolution {
                ppi: 300.0,
                per_cm: false
            })
        );
        assert_eq!(
            decode_image_resources(&doc).len(),
            2,
            "the ICC block is kept"
        );
        // Replacing keeps the existing width/height display units (points).
        let mut data = (72u32 << 16).to_be_bytes().to_vec();
        data.extend_from_slice(&[0, 1, 0, 3]);
        data.extend_from_slice(&(72u32 << 16).to_be_bytes());
        data.extend_from_slice(&[0, 1, 0, 3]);
        let mut doc = document_with(resource(b"8BIM", RESOLUTION_INFO, "", &data));
        let wanted = Resolution {
            ppi: 118.11,
            per_cm: true,
        };
        set_document_resolution(&mut doc, wanted);
        let read = document_resolution(&doc).unwrap();
        assert!((read.ppi - 118.11).abs() < 1e-4 && read.per_cm);
        let block = decode_image_resources(&doc).remove(0);
        assert_eq!(block.data.len(), 16);
        assert_eq!(&block.data[6..8], &[0, 3]);
        assert_eq!(&block.data[14..16], &[0, 3]);
    }

    #[test]
    fn framed_block_round_trips() {
        let framed = frame_image_resource(IPTC_NAA, "", b"odd");
        let expected = resource(b"8BIM", IPTC_NAA, "", b"odd");
        assert_eq!(framed.raw, expected, "framing matches the parser's layout");
        let decoded = decode_image_resources(&document_with(framed.raw.clone()));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].id, IPTC_NAA);
        assert_eq!(decoded[0].data, b"odd");
        assert_eq!(encode_image_resources(&[framed]), expected);
    }

    #[test]
    fn a_long_name_is_truncated_to_the_length_byte() {
        let long = "n".repeat(300);
        let framed = frame_image_resource(IPTC_NAA, &long, b"x");
        assert_eq!(framed.name, "n".repeat(255));
        let decoded = decode_image_resources(&document_with(framed.raw));
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].name, "n".repeat(255));
        assert_eq!(decoded[0].data, b"x");
    }

    #[test]
    fn the_planes_resource_is_replaced_in_place_and_others_are_kept() {
        let other = resource(b"8BIM", VANISHING_POINT, "Another Plug-in", b"theirs");
        let xmp = resource(b"8BIM", XMP_METADATA, "", b"<x/>");
        let tail = b"MXYZ unparsed".to_vec();
        let mut section = [other.clone(), xmp.clone()].concat();
        section.extend_from_slice(&tail);
        let mut doc = document_with(section.clone());
        assert_eq!(
            vanishing_point_data(&doc),
            None,
            "another plug-in's block is not ours"
        );
        set_vanishing_point_data(&mut doc, None);
        assert_eq!(
            doc.image_resources, section,
            "removing nothing changes nothing"
        );
        set_vanishing_point_data(&mut doc, Some(b"one"));
        assert_eq!(vanishing_point_data(&doc).as_deref(), Some(&b"one"[..]));
        set_vanishing_point_data(&mut doc, Some(b"second"));
        let ours = resource(b"8BIM", VANISHING_POINT, VANISHING_POINT_NAME, b"second");
        let mut expected = [other.clone(), xmp.clone(), ours].concat();
        expected.extend_from_slice(&tail);
        assert_eq!(
            doc.image_resources, expected,
            "replaced where it was, the rest verbatim"
        );
        set_vanishing_point_data(&mut doc, None);
        assert_eq!(doc.image_resources, section);
    }
}
