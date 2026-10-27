//! Typed read and in-place edit of the `crs:` XMP settings inside an embedded
//! smart-object payload.
//!
//! ponytail: only the fixed PV2012 property set is modelled and only the literal
//! `crs:` prefix is matched; a packet binding the same URI to a different prefix
//! is preserved but not edited. Widen to a URI-bound scanner when a fixture
//! needs it.

use pictura_core::{CrsSettings, Document, Layer, LayerStack, SmartObject, SmartObjectKind};

use crate::error::PsdError;
use crate::smart_object::{find_subslice, replace_embedded_payload};

/// The fixed `crs:` property set, in packet-name order.
const CRS_NAMES: [&str; 11] = [
    "Exposure2012",
    "Contrast2012",
    "Highlights2012",
    "Shadows2012",
    "Whites2012",
    "Blacks2012",
    "Clarity2012",
    "Vibrance",
    "Saturation",
    "Temperature",
    "Tint",
];

/// Lift the fixed property set out of a bounded `crs:` packet. A missing,
/// non-finite, or unparsable value leaves its field `None`; unknown keys stay in
/// the raw packet.
pub(crate) fn parse_crs(packet: &[u8]) -> CrsSettings {
    CrsSettings {
        exposure: parse_value(packet, "Exposure2012"),
        contrast: parse_value(packet, "Contrast2012"),
        highlights: parse_value(packet, "Highlights2012"),
        shadows: parse_value(packet, "Shadows2012"),
        whites: parse_value(packet, "Whites2012"),
        blacks: parse_value(packet, "Blacks2012"),
        clarity: parse_value(packet, "Clarity2012"),
        vibrance: parse_value(packet, "Vibrance"),
        saturation: parse_value(packet, "Saturation"),
        temperature: parse_value(packet, "Temperature"),
        tint: parse_value(packet, "Tint"),
    }
}

fn parse_value(packet: &[u8], name: &str) -> Option<f64> {
    let (start, end) = crs_value_span(packet, name)?;
    let raw = std::str::from_utf8(&packet[start..end]).ok()?;
    let text = raw.trim();
    let text = text.strip_prefix('+').unwrap_or(text);
    let value: f64 = text.parse().ok()?;
    value.is_finite().then_some(value)
}

/// Replace one fixed-set property inside the packet, or insert it as an
/// attribute on an `rdf:Description` start tag that already carries a `crs:`
/// attribute. `None` when the packet offers no safe rewrite point.
fn patch_crs_packet(packet: &[u8], name: &str, value: f64) -> Option<Vec<u8>> {
    if !value.is_finite() {
        return None;
    }
    let rendered = value.to_string();
    match crs_value_span(packet, name) {
        Some((start, end)) => {
            let mut out = Vec::with_capacity(packet.len() + rendered.len());
            out.extend_from_slice(&packet[..start]);
            out.extend_from_slice(rendered.as_bytes());
            out.extend_from_slice(&packet[end..]);
            Some(out)
        }
        None => insert_crs_attribute(packet, name, &rendered),
    }
}

/// The byte span of a property's value (attribute text or element text) in the
/// packet. Attribute form wins over element form when both exist.
fn crs_value_span(packet: &[u8], name: &str) -> Option<(usize, usize)> {
    let needle = format!("crs:{name}");
    let needle = needle.as_bytes();
    let mut from = 0;
    while let Some(rel) = find_subslice(&packet[from..], needle) {
        let after = from + rel + needle.len();
        if packet.get(after).copied().is_none_or(is_name_byte) {
            from = after;
            continue;
        }
        let i = skip_ws(packet, after);
        if packet.get(i) == Some(&b'=') {
            let i = skip_ws(packet, i + 1);
            let quote = *packet.get(i)?;
            if quote == b'"' || quote == b'\'' {
                let start = i + 1;
                let end = packet[start..].iter().position(|&b| b == quote)? + start;
                return Some((start, end));
            }
        }
        if packet.get(i) == Some(&b'>') {
            let start = i + 1;
            let close = format!("</crs:{name}>");
            let end = find_subslice(&packet[start..], close.as_bytes())? + start;
            return Some((start, end));
        }
        from = after;
    }
    None
}

fn insert_crs_attribute(packet: &[u8], name: &str, rendered: &str) -> Option<Vec<u8>> {
    let open = b"<rdf:Description";
    let mut from = 0;
    while let Some(rel) = find_subslice(&packet[from..], open) {
        let at = from + rel;
        let gt = start_tag_end(packet, at + open.len())?;
        if find_subslice(&packet[at..gt], b"crs:").is_some() {
            let mut k = gt;
            while k > at && is_ws(packet[k - 1]) {
                k -= 1;
            }
            let insert_at = if k > at && packet[k - 1] == b'/' {
                k - 1
            } else {
                gt
            };
            let fragment = format!(" crs:{name}=\"{rendered}\"");
            let mut out = Vec::with_capacity(packet.len() + fragment.len());
            out.extend_from_slice(&packet[..insert_at]);
            out.extend_from_slice(fragment.as_bytes());
            out.extend_from_slice(&packet[insert_at..]);
            return Some(out);
        }
        from = gt + 1;
    }
    None
}

/// The index of the `>` closing the start tag that begins at `from`, skipping
/// any `>` inside quoted attribute values.
fn start_tag_end(packet: &[u8], from: usize) -> Option<usize> {
    let mut quote = 0u8;
    let mut i = from;
    while i < packet.len() {
        let b = packet[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
        } else if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Set one fixed-set `crs:` property on the embedded smart object whose uuid
/// matches `uuid`, rewriting the packet, the object's payload, and the matching
/// preserved `liFD` record. Any error leaves `doc` unchanged.
pub fn set_crs_property(
    doc: &mut Document,
    uuid: &str,
    name: &str,
    value: f64,
) -> Result<(), PsdError> {
    if !CRS_NAMES.contains(&name) {
        return Err(PsdError::Unsupported(format!("crs property {name}")));
    }
    if !value.is_finite() {
        return Err(PsdError::Invalid(format!(
            "crs value for {name} is not finite"
        )));
    }

    let (old_packet, old_payload) = {
        let so = find_smart_object(&doc.layers, uuid)
            .ok_or_else(|| PsdError::Invalid(format!("no smart object with uuid {uuid}")))?;
        if so.kind != SmartObjectKind::Embedded {
            return Err(PsdError::Unsupported(format!(
                "smart object {uuid} is not embedded"
            )));
        }
        let packet = so
            .crs_xmp
            .clone()
            .ok_or_else(|| PsdError::Invalid(format!("smart object {uuid} has no crs packet")))?;
        let payload = so
            .payload()
            .map(<[u8]>::to_vec)
            .ok_or_else(|| PsdError::Invalid(format!("smart object {uuid} has no payload")))?;
        (packet, payload)
    };

    let new_packet = patch_crs_packet(&old_packet, name, value)
        .ok_or_else(|| PsdError::Invalid(format!("crs packet cannot be patched for {name}")))?;
    let at = find_subslice(&old_payload, &old_packet)
        .ok_or_else(|| PsdError::Invalid("crs packet not found in payload".into()))?;
    let mut new_payload = Vec::with_capacity(old_payload.len() + new_packet.len());
    new_payload.extend_from_slice(&old_payload[..at]);
    new_payload.extend_from_slice(&new_packet);
    new_payload.extend_from_slice(&old_payload[at + old_packet.len()..]);

    let new_section =
        replace_embedded_payload(&doc.layer_section_extra, uuid, &new_payload, doc.is_psb)
            .ok_or_else(|| PsdError::Invalid(format!("no embedded record for uuid {uuid}")))?;
    let new_crs = parse_crs(&new_packet);

    let so = find_smart_object_mut(&mut doc.layers, uuid)
        .ok_or_else(|| PsdError::Invalid(format!("smart object {uuid} vanished")))?;
    so.crs_xmp = Some(new_packet);
    so.set_payload(new_payload);
    so.crs = Some(new_crs);
    doc.layer_section_extra = new_section;
    Ok(())
}

fn find_smart_object<'a>(layers: &'a [Layer], uuid: &str) -> Option<&'a SmartObject> {
    for layer in layers {
        if let Some(so) = layer.smart_object.as_ref() {
            if so.uuid == uuid {
                return Some(so);
            }
        }
        if let Some(so) = find_smart_object(&layer.children, uuid) {
            return Some(so);
        }
    }
    None
}

fn find_smart_object_mut<'a>(
    layers: &'a mut LayerStack,
    uuid: &str,
) -> Option<&'a mut SmartObject> {
    for layer in layers.iter_mut() {
        if layer
            .smart_object
            .as_ref()
            .is_some_and(|so| so.uuid == uuid)
        {
            return layer.smart_object.as_mut();
        }
        if let Some(so) = find_smart_object_mut(&mut layer.children, uuid) {
            return Some(so);
        }
    }
    None
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn skip_ws(packet: &[u8], mut i: usize) -> usize {
    while packet.get(i).copied().is_some_and(is_ws) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerBlock, PsdRect};

    const UUID: &str = "12345678-1234-1234-1234-123456789abc";

    fn packet() -> Vec<u8> {
        b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description \
crs:Exposure2012=\"+0.50\" crs:Contrast2012=\"-10\"/></rdf:RDF></x:xmpmeta>"
            .to_vec()
    }

    fn payload() -> Vec<u8> {
        let mut v = b"8BPS\x00\x02".to_vec();
        v.extend_from_slice(&packet());
        v.extend_from_slice(b"TRAILER");
        v
    }

    fn pascal(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut v = vec![b.len() as u8];
        v.extend_from_slice(b);
        v
    }

    fn unicode(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut v = (units.len() as u32).to_be_bytes().to_vec();
        for unit in units {
            v.extend_from_slice(&unit.to_be_bytes());
        }
        v
    }

    fn plld(uuid: &str) -> Vec<u8> {
        let mut v = b"plcL".to_vec();
        v.extend_from_slice(&3u32.to_be_bytes());
        v.extend_from_slice(&pascal(uuid));
        v
    }

    fn linked_layer(kind: &[u8; 4], version: u32, uuid: &str, data: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(kind);
        v.extend_from_slice(&version.to_be_bytes());
        v.extend_from_slice(&pascal(uuid));
        v.extend_from_slice(&unicode("raw.psb"));
        v.extend_from_slice(b"8BPB");
        v.extend_from_slice(b"8BIM");
        v.extend_from_slice(&(data.len() as u64).to_be_bytes());
        v.push(0);
        match kind {
            b"liFD" => v.extend_from_slice(data),
            b"liFE" => {
                v.extend_from_slice(&16u32.to_be_bytes());
                v.extend_from_slice(&0u32.to_be_bytes());
                v.extend_from_slice(&0u32.to_be_bytes());
                v.extend_from_slice(b"null");
                v.extend_from_slice(&0u32.to_be_bytes());
                v.extend_from_slice(&0u64.to_be_bytes());
            }
            _ => {}
        }
        if version >= 5 {
            v.extend_from_slice(&unicode("\0"));
        }
        if version >= 6 {
            v.extend_from_slice(&0f64.to_be_bytes());
        }
        if version >= 7 {
            v.push(0);
        }
        v
    }

    fn tagged(key: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = b"8BIM".to_vec();
        v.extend_from_slice(key);
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        v.extend_from_slice(&vec![0u8; (4 - data.len() % 4) % 4]);
        v
    }

    fn linked_list(blocks: &[Vec<u8>]) -> Vec<u8> {
        let mut v = Vec::new();
        for b in blocks {
            v.extend_from_slice(&(b.len() as u64).to_be_bytes());
            v.extend_from_slice(b);
            v.extend_from_slice(&vec![0u8; (4 - b.len() % 4) % 4]);
        }
        v
    }

    fn base_doc(section: Vec<u8>) -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        for (i, b) in doc.composite.data.iter_mut().enumerate() {
            *b = (i * 7 + 1) as u8;
        }
        let layer = Layer {
            name: "Smart".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![10; 16].into(),
                },
                Channel {
                    id: 1,
                    data: vec![20; 16].into(),
                },
                Channel {
                    id: 2,
                    data: vec![30; 16].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; 16].into(),
                },
            ],
            extra_blocks: vec![LayerBlock {
                key: *b"PlLd",
                data: plld(UUID),
            }],
            ..Default::default()
        };
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        crate::smart_object::resolve_smart_objects(&mut doc.layers, &section, false);
        doc.layer_section_extra = section;
        doc
    }

    fn crs_doc_with_payload(payload: &[u8]) -> Document {
        let section = tagged(
            b"lnk2",
            &linked_list(&[linked_layer(b"liFD", 7, UUID, payload)]),
        );
        base_doc(section)
    }

    #[test]
    fn synthetic_packet_parses_the_fixed_set() {
        let crs = parse_crs(&packet());
        assert_eq!(crs.exposure, Some(0.5));
        assert_eq!(crs.contrast, Some(-10.0));
        assert_eq!(crs.highlights, None);
        assert_eq!(crs.tint, None);
    }

    #[test]
    fn element_form_and_unparsable_values_are_handled() {
        let element = b"<x:xmpmeta><rdf:RDF><rdf:Description>\
<crs:Exposure2012>1.5</crs:Exposure2012>\
<crs:Shadows2012>nope</crs:Shadows2012></rdf:Description></rdf:RDF></x:xmpmeta>";
        let crs = parse_crs(element);
        assert_eq!(crs.exposure, Some(1.5));
        assert_eq!(crs.shadows, None, "unparsable stays None");
    }

    #[test]
    fn edit_round_trips_through_write_and_read() {
        let mut doc = crs_doc_with_payload(&payload());
        let so = find_smart_object(&doc.layers, UUID).unwrap();
        let old_payload = so.payload().map(<[u8]>::to_vec).unwrap();
        let old_packet = so.crs_xmp.clone().unwrap();

        set_crs_property(&mut doc, UUID, "Exposure2012", 1.25).expect("edit succeeds");

        let so = find_smart_object(&doc.layers, UUID).unwrap();
        assert_eq!(so.crs.as_ref().unwrap().exposure, Some(1.25));
        let new_payload = so.payload().map(<[u8]>::to_vec).unwrap();
        let new_packet = so.crs_xmp.clone().unwrap();
        let at = find_subslice(&old_payload, &old_packet).unwrap();
        let mut expected = old_payload[..at].to_vec();
        expected.extend_from_slice(&new_packet);
        expected.extend_from_slice(&old_payload[at + old_packet.len()..]);
        assert_eq!(
            new_payload, expected,
            "bytes outside the patched packet span are unchanged"
        );

        let needle = b"crs:Exposure2012=\"1.25\"";
        assert!(doc
            .layer_section_extra
            .windows(needle.len())
            .any(|w| w == needle));
        let old_needle = b"crs:Exposure2012=\"+0.50\"";
        assert!(!doc
            .layer_section_extra
            .windows(old_needle.len())
            .any(|w| w == old_needle));

        let back = crate::read_psd(&crate::write_psd(&doc).expect("writes")).expect("re-reads");
        let so = find_smart_object(&back.layers, UUID).unwrap();
        assert_eq!(so.crs.as_ref().unwrap().exposure, Some(1.25));
    }

    #[test]
    fn insert_adds_an_absent_attribute_on_the_description_tag() {
        let packet = b"<x:xmpmeta><rdf:RDF><rdf:Description crs:Contrast2012=\"-5\"/>\
</rdf:RDF></x:xmpmeta>";
        let doc = crs_doc_with_payload(packet);
        let mut doc = doc;
        set_crs_property(&mut doc, UUID, "Whites2012", 0.75).expect("insert succeeds");
        let so = find_smart_object(&doc.layers, UUID).unwrap();
        assert_eq!(so.crs.as_ref().unwrap().whites, Some(0.75));
        assert_eq!(so.crs.as_ref().unwrap().contrast, Some(-5.0));
        assert!(so
            .crs_xmp
            .as_ref()
            .unwrap()
            .windows(b"crs:Whites2012=\"0.75\"".len())
            .any(|w| w == b"crs:Whites2012=\"0.75\""));
    }

    #[test]
    fn errors_leave_the_document_unchanged() {
        let mut doc = crs_doc_with_payload(&payload());
        let section = doc.layer_section_extra.clone();
        let payload = find_smart_object(&doc.layers, UUID)
            .unwrap()
            .payload()
            .map(<[u8]>::to_vec);

        assert!(matches!(
            set_crs_property(&mut doc, UUID, "Nope", 1.0),
            Err(PsdError::Unsupported(_))
        ));
        assert!(set_crs_property(
            &mut doc,
            "00000000-0000-0000-0000-000000000000",
            "Exposure2012",
            1.0
        )
        .is_err());
        assert!(set_crs_property(&mut doc, UUID, "Exposure2012", f64::NAN).is_err());

        assert_eq!(doc.layer_section_extra, section);
        assert_eq!(
            find_smart_object(&doc.layers, UUID).unwrap().payload(),
            payload.as_deref()
        );
    }

    #[test]
    fn no_packet_and_non_embedded_error_without_mutation() {
        let mut doc = crs_doc_with_payload(b"8BPS\x00\x02no xmp here");
        assert!(set_crs_property(&mut doc, UUID, "Exposure2012", 1.0).is_err());

        let external = tagged(
            b"lnk2",
            &linked_list(&[linked_layer(b"liFE", 3, UUID, &[])]),
        );
        let mut doc = base_doc(external);
        let section = doc.layer_section_extra.clone();
        assert!(matches!(
            set_crs_property(&mut doc, UUID, "Exposure2012", 1.0),
            Err(PsdError::Unsupported(_))
        ));
        assert_eq!(doc.layer_section_extra, section);
    }

    #[test]
    fn unmodified_document_round_trips_byte_identical() {
        let doc = crs_doc_with_payload(&payload());
        let section = doc.layer_section_extra.clone();
        let written = crate::write_psd(&doc).expect("writes");
        let back = crate::read_psd(&written).expect("re-reads");
        assert_eq!(
            back.layer_section_extra, section,
            "the preserved linked-record section is unchanged"
        );
        assert_eq!(
            find_smart_object(&back.layers, UUID)
                .unwrap()
                .crs
                .as_ref()
                .unwrap()
                .exposure,
            Some(0.5)
        );
        assert_eq!(
            crate::write_psd(&back).expect("re-writes"),
            written,
            "open to save is byte-identical"
        );
    }

    #[test]
    fn payload_digest_tracks_replace_and_crs_patch() {
        let mut doc = crs_doc_with_payload(&payload());
        let before = find_smart_object(&doc.layers, UUID)
            .unwrap()
            .payload_digest();
        assert_ne!(before, 0, "a decoded payload has a digest");

        set_crs_property(&mut doc, UUID, "Exposure2012", 1.25).expect("edit succeeds");
        let patched = find_smart_object(&doc.layers, UUID)
            .unwrap()
            .payload_digest();
        assert_ne!(
            before, patched,
            "patching the CRS packet changes the payload"
        );

        find_smart_object_mut(&mut doc.layers, UUID)
            .unwrap()
            .set_payload(b"8BPS\x00\x01replacement".to_vec());
        let replaced = find_smart_object(&doc.layers, UUID)
            .unwrap()
            .payload_digest();
        assert_ne!(
            patched, replaced,
            "replacing the payload changes the digest"
        );
    }
}
