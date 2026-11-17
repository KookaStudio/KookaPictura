//! The document's global light: PSD image resources 1037 (angle) and 1049
//! (altitude), each a signed big-endian 32-bit integer in degrees.
//!
//! The raw resource section stays the source of truth; the typed
//! [`Document::global_light`] is resolved on open and emitted on save, following
//! the guides precedent.

use pictura_core::{Document, GlobalLight};

use crate::image_resources::{decode_section, encode_image_resources, frame_image_resource};

const GLOBAL_ANGLE: u16 = 1037;
const GLOBAL_ALTITUDE: u16 = 1049;
const DEFAULT_DEGREES: i32 = 30;

fn resource_degrees(data: &[u8]) -> Option<i32> {
    let bytes = data.get(..4)?;
    Some(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// The pair stored in `section`, each absent or shorter-than-four-byte value
/// reading `None`.
fn stored(section: &[u8]) -> (Option<i32>, Option<i32>) {
    let (resources, _) = decode_section(section);
    let find = |id: u16| {
        resources
            .iter()
            .find(|resource| resource.id == id)
            .and_then(|resource| resource_degrees(&resource.data))
    };
    (find(GLOBAL_ANGLE), find(GLOBAL_ALTITUDE))
}

/// Derive the document's global light from its preserved image-resource
/// section. A missing or malformed resource contributes Photoshop's default of
/// 30 degrees.
pub fn resolve_global_light(doc: &mut Document) {
    let (angle, altitude) = stored(&doc.image_resources);
    doc.global_light = GlobalLight {
        angle: angle.unwrap_or(DEFAULT_DEGREES),
        altitude: altitude.unwrap_or(DEFAULT_DEGREES),
    };
}

/// `section` with resources 1037/1049 standing for the document's global light.
/// Returned unchanged when both values already match (an absent resource
/// counting as its default), so an unedited file keeps its bytes. When either
/// value differs, both resources are written, replacing existing ones or
/// appending them; every other block and any trailing bytes are preserved.
pub(crate) fn with_document_global_light(doc: &Document, section: Vec<u8>) -> Vec<u8> {
    let (stored_angle, stored_altitude) = stored(&section);
    let angle = doc.global_light.angle;
    let altitude = doc.global_light.altitude;
    if stored_angle.unwrap_or(DEFAULT_DEGREES) == angle
        && stored_altitude.unwrap_or(DEFAULT_DEGREES) == altitude
    {
        return section;
    }
    // ponytail: decode_section stops at an unrecognized signature, so a
    // 1037/1049 behind such a tail would be duplicated by the append; parse
    // the tail if a real file needs it.
    let (resources, consumed) = decode_section(&section);
    let mut kept: Vec<_> = resources
        .into_iter()
        .filter(|resource| resource.id != GLOBAL_ANGLE && resource.id != GLOBAL_ALTITUDE)
        .collect();
    kept.push(frame_image_resource(GLOBAL_ANGLE, "", &angle.to_be_bytes()));
    kept.push(frame_image_resource(
        GLOBAL_ALTITUDE,
        "",
        &altitude.to_be_bytes(),
    ));
    let mut out = encode_image_resources(&kept);
    out.extend_from_slice(&section[consumed..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode};

    fn doc_with_section(section: Vec<u8>) -> Document {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources = section.into();
        doc
    }

    fn pair_section(angle: i32, altitude: i32) -> Vec<u8> {
        encode_image_resources(&[
            frame_image_resource(GLOBAL_ANGLE, "", &angle.to_be_bytes()),
            frame_image_resource(GLOBAL_ALTITUDE, "", &altitude.to_be_bytes()),
        ])
    }

    #[test]
    fn global_light_reads_angle_and_altitude() {
        let mut doc = doc_with_section(pair_section(45, 60));
        resolve_global_light(&mut doc);
        assert_eq!(
            doc.global_light,
            GlobalLight {
                angle: 45,
                altitude: 60
            }
        );
    }

    #[test]
    fn global_light_defaults_for_missing_or_malformed() {
        let mut empty = doc_with_section(Vec::new());
        resolve_global_light(&mut empty);
        assert_eq!(empty.global_light, GlobalLight::default());

        let short = encode_image_resources(&[frame_image_resource(GLOBAL_ANGLE, "", &[7, 7])]);
        let mut malformed = doc_with_section(short);
        resolve_global_light(&mut malformed);
        assert_eq!(
            malformed.global_light,
            GlobalLight {
                angle: 30,
                altitude: 30
            }
        );
    }

    #[test]
    fn global_light_writes_and_preserves_other_resources() {
        let xmp = frame_image_resource(1060, "", b"<x/>");
        let xmp_raw = xmp.raw.clone();
        let mut doc = doc_with_section(encode_image_resources(&[xmp]));
        doc.global_light = GlobalLight {
            angle: 90,
            altitude: 45,
        };
        let bytes = crate::write_psd(&doc).expect("writes");
        let read = crate::read_psd(&bytes).expect("reads");
        assert_eq!(
            read.global_light,
            GlobalLight {
                angle: 90,
                altitude: 45
            }
        );
        let resources = crate::decode_image_resources(&read);
        assert!(resources.iter().any(|resource| resource.id == 1060
            && resource.data == b"<x/>"
            && resource.raw == xmp_raw));
    }

    #[test]
    fn global_light_unchanged_section_round_trips() {
        let doc = doc_with_section(Vec::new());
        let bytes = crate::write_psd(&doc).expect("writes");
        let read = crate::read_psd(&bytes).expect("reads");
        assert!(!crate::decode_image_resources(&read)
            .iter()
            .any(|resource| resource.id == GLOBAL_ANGLE || resource.id == GLOBAL_ALTITUDE));
        assert!(read.image_resources.is_empty());
    }

    #[test]
    fn global_light_matching_section_round_trips() {
        let section = pair_section(45, 60);
        let mut doc = doc_with_section(section.clone());
        doc.global_light = GlobalLight {
            angle: 45,
            altitude: 60,
        };
        let bytes = crate::write_psd(&doc).expect("writes");
        let read = crate::read_psd(&bytes).expect("reads");
        assert_eq!(&read.image_resources[..], section.as_slice());
    }
}
