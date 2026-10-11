//! The document's guides: the grid-and-guides image resource (1032).
//!
//! The resource is a version (1), the horizontal and vertical grid cycles, a
//! guide count, then per guide a signed big-endian location in 1/32 document
//! pixels and a direction byte (0 vertical, 1 horizontal). The 1/32 unit is the
//! de-facto convention other readers use; the file-format specification only
//! says "document coordinates".

use pictura_core::{Document, Guide, GuideOrientation};

use crate::image_resources::{decode_section, encode_image_resources, frame_image_resource};

const GRID_AND_GUIDES: u16 = 1032;
/// The grid cycle the specification says to write when there is no grid.
const DEFAULT_GRID_CYCLE: u32 = 576;
const HEADER_LEN: usize = 16;
const GUIDE_LEN: usize = 5;

/// A 1032 payload's guides and `(horizontal, vertical)` grid cycles.
type StoredGuides = (Vec<Guide>, (u32, u32));

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The guides and the `(horizontal, vertical)` grid cycles a 1032 payload
/// holds; `None` when it is truncated.
fn decode_guides(data: &[u8]) -> Option<StoredGuides> {
    if data.len() < HEADER_LEN {
        return None;
    }
    let cycles = (be_u32(&data[4..8]), be_u32(&data[8..12]));
    let count = be_u32(&data[12..16]) as usize;
    let (records, _) = data[HEADER_LEN..].as_chunks::<GUIDE_LEN>();
    if records.len() < count {
        return None;
    }
    let guides = records[..count]
        .iter()
        .map(|record| Guide {
            orientation: if record[4] == 0 {
                GuideOrientation::Vertical
            } else {
                GuideOrientation::Horizontal
            },
            position: f64::from(be_u32(record) as i32) / 32.0,
        })
        .collect();
    Some((guides, cycles))
}

fn encode_guides(guides: &[Guide], (horizontal, vertical): (u32, u32)) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + guides.len() * GUIDE_LEN);
    for word in [1, horizontal, vertical, guides.len() as u32] {
        out.extend_from_slice(&word.to_be_bytes());
    }
    for guide in guides {
        let location = (guide.position * 32.0).round() as i32;
        out.extend_from_slice(&location.to_be_bytes());
        out.push(match guide.orientation {
            GuideOrientation::Vertical => 0,
            GuideOrientation::Horizontal => 1,
        });
    }
    out
}

/// The 1032 payload `section` holds: `None` when absent, `Some(None)` when
/// malformed.
fn stored_guides(section: &[u8]) -> Option<Option<StoredGuides>> {
    decode_section(section)
        .0
        .into_iter()
        .find(|r| r.id == GRID_AND_GUIDES)
        .map(|r| decode_guides(&r.data))
}

/// The 1032 payload `section` holds, decoded.
fn section_guides(section: &[u8]) -> Option<StoredGuides> {
    stored_guides(section).flatten()
}

/// Derive the document's guides from its preserved image-resource section. A
/// malformed resource reads as no guides.
pub(crate) fn resolve_guides(doc: &mut Document) {
    doc.guides = section_guides(&doc.image_resources)
        .map(|(guides, _)| guides)
        .unwrap_or_default();
}

/// The image-resource section a save of `doc` under header mode `mode_code`
/// emits: the path-resource section with the document's current guides.
pub(crate) fn output_resources(doc: &Document, mode_code: u16) -> Vec<u8> {
    with_document_guides(doc, crate::path_resources::output_resources(doc, mode_code))
}

/// `section` with its 1032 resource standing for the document's guides.
/// Returned unchanged when it already holds them (or, with no guides, holds no
/// 1032), so an unedited file keeps its original bytes; the grid cycles of an
/// existing resource are kept.
pub(crate) fn with_document_guides(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    let stored = stored_guides(&section);
    let unchanged = match &stored {
        None => doc.guides.is_empty(),
        Some(Some((guides, _))) => !doc.guides.is_empty() && *guides == doc.guides,
        Some(None) => false,
    };
    if unchanged {
        return section;
    }
    let cycles = stored
        .flatten()
        .map_or((DEFAULT_GRID_CYCLE, DEFAULT_GRID_CYCLE), |(_, c)| c);
    let (resources, consumed) = decode_section(&section);
    let mut kept: Vec<_> = resources
        .into_iter()
        .filter(|r| r.id != GRID_AND_GUIDES)
        .collect();
    if !doc.guides.is_empty() {
        kept.push(frame_image_resource(
            GRID_AND_GUIDES,
            "",
            &encode_guides(&doc.guides, cycles),
        ));
    }
    let mut out = encode_image_resources(&kept);
    out.extend_from_slice(&section[consumed..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read_psd, write_psd};
    use pictura_core::{BitDepth, ColorMode};

    fn guide(orientation: GuideOrientation, position: f64) -> Guide {
        Guide {
            orientation,
            position,
        }
    }

    fn sample() -> Vec<Guide> {
        vec![
            guide(GuideOrientation::Vertical, 40.0),
            guide(GuideOrientation::Horizontal, 12.5),
            guide(GuideOrientation::Horizontal, -3.0),
        ]
    }

    #[test]
    fn guides_round_trip_through_a_psd() {
        let mut doc = Document::new(200, 100, ColorMode::Rgb, BitDepth::Eight);
        doc.guides = sample();
        let back = read_psd(&write_psd(&doc).expect("write")).expect("read");
        assert_eq!(back.guides, sample());
    }

    #[test]
    fn payload_layout() {
        let data = encode_guides(&sample()[..2], (576, 576));
        assert_eq!(
            data,
            [
                0, 0, 0, 1, 0, 0, 2, 64, 0, 0, 2, 64, 0, 0, 0, 2, //
                0, 0, 5, 0, 0, //
                0, 0, 1, 144, 1,
            ]
        );
        assert_eq!(decode_guides(&data[..data.len() - 1]), None);
    }

    #[test]
    fn unchanged_guides_keep_their_bytes_and_grid_cycles() {
        let other = frame_image_resource(1005, "", &[0; 16]);
        let stored =
            frame_image_resource(GRID_AND_GUIDES, "", &encode_guides(&sample(), (100, 200)));
        let section = encode_image_resources(&[other.clone(), stored]);
        let mut doc = Document::new(200, 100, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources = section.clone();
        resolve_guides(&mut doc);
        assert_eq!(doc.guides, sample());
        assert_eq!(with_document_guides(&doc, section.clone()), section);

        doc.guides.pop();
        let edited = with_document_guides(&doc, section.clone());
        let (guides, cycles) = section_guides(&edited).expect("1032 present");
        assert_eq!(guides, &sample()[..2]);
        assert_eq!(cycles, (100, 200));

        doc.guides.clear();
        let cleared = with_document_guides(&doc, section);
        assert_eq!(cleared, encode_image_resources(&[other]));
    }

    #[test]
    fn no_guides_write_no_resource() {
        let doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        assert!(with_document_guides(&doc, Vec::new()).is_empty());
    }

    #[test]
    fn empty_or_malformed_resource_is_dropped_without_guides() {
        let other = frame_image_resource(1005, "", &[0; 16]);
        let expected = encode_image_resources(std::slice::from_ref(&other));
        let doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        for payload in [encode_guides(&[], (576, 576)), vec![0, 0, 0, 1]] {
            let stored = frame_image_resource(GRID_AND_GUIDES, "", &payload);
            let section = encode_image_resources(&[other.clone(), stored]);
            assert_eq!(with_document_guides(&doc, section), expected);
        }
    }
}
