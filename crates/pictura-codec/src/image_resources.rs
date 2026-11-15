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
/// The legacy alpha-channel names resource id (MacRoman Pascal strings).
pub const ALPHA_NAMES_PASCAL: u16 = 1006;
/// The Unicode alpha-channel names resource id.
pub const ALPHA_NAMES_UNICODE: u16 = 1045;
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
    document.image_resources = section.into();
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
    document.image_resources = section.into();
}

/// The stored extra-channel names in channel order. Resource 1045 (Unicode) is
/// preferred; resource 1006 (MacRoman Pascal strings) is the legacy fallback.
pub fn alpha_names(document: &Document) -> Vec<String> {
    let mut names = stored_alpha_names(document).unwrap_or_default();
    names.truncate(document.channels.len());
    names
}

fn stored_alpha_names(document: &Document) -> Option<Vec<String>> {
    let (resources, _) = decode_image_resources_with_len(document);
    let unicode = resources
        .iter()
        .find(|r| r.id == ALPHA_NAMES_UNICODE)
        .and_then(|r| decode_unicode_names(&r.data));
    let pascal = resources
        .iter()
        .find(|r| r.id == ALPHA_NAMES_PASCAL)
        .and_then(|r| decode_pascal_names(&r.data));
    unicode.or(pascal)
}

/// Replace the document's alpha-name resources with `names`, or remove them
/// when `names` is empty. Every other block and the unparsed tail are kept
/// byte-for-byte; unchanged names leave the section untouched.
pub fn set_alpha_names(document: &mut Document, names: &[String]) {
    let stored = stored_alpha_names(document);
    let unchanged = match &stored {
        Some(stored) => stored == names,
        None => names.is_empty(),
    };
    if unchanged {
        return;
    }
    let (resources, consumed) = decode_image_resources_with_len(document);
    let mut kept: Vec<ImageResource> = resources
        .into_iter()
        .filter(|r| r.id != ALPHA_NAMES_PASCAL && r.id != ALPHA_NAMES_UNICODE)
        .collect();
    if !names.is_empty() {
        kept.push(frame_image_resource(
            ALPHA_NAMES_UNICODE,
            "",
            &encode_unicode_names(names),
        ));
        kept.push(frame_image_resource(
            ALPHA_NAMES_PASCAL,
            "",
            &encode_pascal_names(names),
        ));
    }
    let mut section = encode_image_resources(&kept);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section.into();
}

/// The PSD DisplayInfo resource id: one entry per extra channel.
pub const DISPLAY_INFO: u16 = 1077;

/// The DisplayInfo mode that marks a spot channel.
pub const SPOT_CHANNEL_MODE: u8 = 2;

/// One extra channel's stored display information: its on-screen colour and
/// whether it is an alpha channel or a spot ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelDisplayInfo {
    pub color_space: u16,
    pub components: [u16; 4],
    pub opacity: u16,
    /// `0` alpha, `1` inverted alpha, `2` spot.
    pub mode: u8,
}

/// The document's stored extra-channel display information in channel order.
/// An absent or malformed resource is an empty list.
pub fn display_info(document: &Document) -> Vec<ChannelDisplayInfo> {
    let (resources, _) = decode_image_resources_with_len(document);
    resources
        .iter()
        .find(|r| r.id == DISPLAY_INFO)
        .map(|r| decode_display_info(&r.data))
        .unwrap_or_default()
}

fn decode_display_info(data: &[u8]) -> Vec<ChannelDisplayInfo> {
    let mut entries = Vec::new();
    for entry in data.get(4..).unwrap_or_default().as_chunks::<13>().0 {
        let word = |at: usize| u16::from_be_bytes([entry[at], entry[at + 1]]);
        entries.push(ChannelDisplayInfo {
            color_space: word(0),
            components: [word(2), word(4), word(6), word(8)],
            opacity: word(10),
            mode: entry[12],
        });
    }
    entries
}

fn encode_display_info(entries: &[ChannelDisplayInfo]) -> Vec<u8> {
    let mut out = 1u32.to_be_bytes().to_vec();
    for entry in entries {
        for word in [
            entry.color_space,
            entry.components[0],
            entry.components[1],
            entry.components[2],
            entry.components[3],
            entry.opacity,
        ] {
            out.extend_from_slice(&word.to_be_bytes());
        }
        out.push(entry.mode);
    }
    out
}

/// Replace the document's DisplayInfo resource with `entries`, or remove it
/// when `entries` is empty. Unparsed trailing bytes are preserved.
pub fn set_display_info(document: &mut Document, entries: &[ChannelDisplayInfo]) {
    let (resources, consumed) = decode_image_resources_with_len(document);
    let has_resource = resources.iter().any(|r| r.id == DISPLAY_INFO);
    if !has_resource && entries.is_empty() {
        return;
    }
    let mut kept: Vec<ImageResource> = resources
        .into_iter()
        .filter(|r| r.id != DISPLAY_INFO)
        .collect();
    if !entries.is_empty() {
        kept.push(frame_image_resource(
            DISPLAY_INFO,
            "",
            &encode_display_info(entries),
        ));
    }
    let mut section = encode_image_resources(&kept);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section.into();
}

/// The indices of the document's extra channels whose stored mode is spot.
pub fn spot_channel_indices(document: &Document) -> Vec<usize> {
    display_info(document)
        .into_iter()
        .enumerate()
        .filter(|(i, entry)| *i < document.channels.len() && entry.mode == SPOT_CHANNEL_MODE)
        .map(|(i, _)| i)
        .collect()
}

fn decode_pascal_names(data: &[u8]) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let len = *data.get(at)? as usize;
        at += 1;
        let bytes = data.get(at..at.checked_add(len)?)?;
        at += len;
        names.push(crate::engine_data::decode_macroman(bytes));
    }
    Some(names)
}

fn encode_pascal_names(names: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for name in names {
        let mut bytes: Vec<u8> = name
            .chars()
            .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
            .collect();
        bytes.truncate(u8::MAX as usize);
        out.push(bytes.len() as u8);
        out.extend_from_slice(&bytes);
    }
    out
}

fn decode_unicode_names(data: &[u8]) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let count = u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize;
        at += 4;
        let bytes = data.get(at..at.checked_add(count.checked_mul(2)?)?)?;
        at += count * 2;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect();
        names.push(String::from_utf16_lossy(&units));
    }
    Some(names)
}

fn encode_unicode_names(names: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for name in names {
        let units: Vec<u16> = name.encode_utf16().collect();
        out.extend_from_slice(&(units.len() as u32).to_be_bytes());
        for unit in units {
            out.extend_from_slice(&unit.to_be_bytes());
        }
    }
    out
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
    use pictura_core::{BitDepth, Channel, ColorMode};

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
        doc.image_resources = section.into();
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
    fn display_info_round_trips() {
        let mut doc = document_with(Vec::new());
        doc.channels.push(Channel {
            id: 0,
            data: vec![0; 4].into(),
        });
        doc.channels.push(Channel {
            id: 1,
            data: vec![0; 4].into(),
        });
        let entries = [
            ChannelDisplayInfo {
                color_space: 2,
                components: [0, 0, 0, 0],
                opacity: 50,
                mode: 0,
            },
            ChannelDisplayInfo {
                color_space: 1,
                components: [255, 0, 0, 0],
                opacity: 100,
                mode: SPOT_CHANNEL_MODE,
            },
        ];
        set_display_info(&mut doc, &entries);
        assert_eq!(display_info(&doc), entries);
        assert_eq!(spot_channel_indices(&doc), vec![1]);
        set_display_info(&mut doc, &[]);
        assert!(display_info(&doc).is_empty());
    }

    #[test]
    fn empty_display_info_resource_is_removed() {
        let mut doc = document_with(resource(b"8BIM", DISPLAY_INFO, "", &1u32.to_be_bytes()));
        assert!(display_info(&doc).is_empty());
        set_display_info(&mut doc, &[]);
        assert!(!decode_image_resources(&doc)
            .iter()
            .any(|r| r.id == DISPLAY_INFO));
    }

    #[test]
    fn spot_channel_indices_ignore_alpha_and_out_of_range() {
        let mut doc = document_with(Vec::new());
        doc.channels.push(Channel {
            id: 0,
            data: vec![0; 4].into(),
        });
        set_display_info(
            &mut doc,
            &[
                ChannelDisplayInfo {
                    mode: 0,
                    ..ChannelDisplayInfo::default()
                },
                ChannelDisplayInfo {
                    mode: SPOT_CHANNEL_MODE,
                    ..ChannelDisplayInfo::default()
                },
            ],
        );
        assert_eq!(spot_channel_indices(&doc), Vec::<usize>::new());

        doc.channels.push(Channel {
            id: 1,
            data: vec![0; 4].into(),
        });
        assert_eq!(spot_channel_indices(&doc), vec![1]);
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

    fn channel(id: i16, len: usize) -> Channel {
        Channel {
            id,
            data: vec![0u8; len].into(),
        }
    }

    #[test]
    fn alpha_names_round_trip_ascii_and_unicode() {
        let mut doc = Document::new(4, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.channels = vec![channel(0, 8), channel(1, 8)];
        let names = vec!["Hair".to_string(), "Ciel étoilé".to_string()];
        set_alpha_names(&mut doc, &names);
        let back = crate::read_psd(&crate::write_psd(&doc).expect("write")).expect("read");
        assert_eq!(alpha_names(&back), names);
    }

    #[test]
    fn alpha_names_pascal_fallback() {
        let data = encode_pascal_names(&["Legacy".to_string()]);
        let section =
            encode_image_resources(&[frame_image_resource(ALPHA_NAMES_PASCAL, "", &data)]);
        let mut doc = document_with(section);
        doc.channels = vec![channel(0, 1)];
        assert_eq!(alpha_names(&doc), vec!["Legacy".to_string()]);
    }

    #[test]
    fn alpha_names_preserve_other_resources() {
        let other = frame_image_resource(XMP_METADATA, "", b"<x/>");
        let mut doc = document_with(encode_image_resources(&[other]));
        doc.channels = vec![channel(0, 1)];
        set_alpha_names(&mut doc, &["Hair".to_string()]);
        let resources = decode_image_resources(&doc);
        assert!(resources
            .iter()
            .any(|r| r.id == XMP_METADATA && r.data == b"<x/>"));
        assert_eq!(alpha_names(&doc), vec!["Hair".to_string()]);
    }

    #[test]
    fn alpha_names_unchanged_keeps_bytes() {
        let mut doc = document_with(Vec::new());
        doc.channels = vec![channel(0, 1)];
        set_alpha_names(&mut doc, &["Hair".to_string()]);
        let before = doc.image_resources.clone();
        set_alpha_names(&mut doc, &["Hair".to_string()]);
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn alpha_names_empty_removes_the_resources() {
        let mut doc = document_with(Vec::new());
        doc.channels = vec![channel(0, 1)];
        set_alpha_names(&mut doc, &["Hair".to_string()]);
        set_alpha_names(&mut doc, &[]);
        assert!(decode_image_resources(&doc).is_empty());
    }

    #[test]
    fn alpha_names_pascal_encodes_non_ascii_as_a_fallback() {
        let names = vec!["Ciel étoilé".to_string()];
        let encoded = encode_pascal_names(&names);
        assert_eq!(&encoded[1..1 + encoded[0] as usize], b"Ciel ?toil?");
        let unicode = decode_unicode_names(&encode_unicode_names(&names)).expect("unicode");
        assert_eq!(unicode, names);
    }

    #[test]
    fn alpha_names_pascal_reads_unpadded_names() {
        let data = [4u8, b'H', b'a', b'i', b'r', 3, b'B', b'o', b'b'];
        assert_eq!(
            decode_pascal_names(&data),
            Some(vec!["Hair".to_string(), "Bob".to_string()])
        );
    }

    #[test]
    fn alpha_names_pascal_decodes_macroman() {
        assert_eq!(
            decode_pascal_names(&[2, 0x8E, 0xA9]),
            Some(vec!["é©".to_string()])
        );
    }
}
