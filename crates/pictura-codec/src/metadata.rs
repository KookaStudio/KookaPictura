//! A read-only metadata view over a document's image resources.
//!
//! `read_metadata` gathers the decoded EXIF (1058/1059), IPTC-IIM (1028), and
//! the raw XMP packet (1060) so the app can show them; [`xmp_properties`] adds
//! the parsed XMP fields. Edits go through [`set_iptc_fields`] (IIM only),
//! [`set_xmp_fields`] (XMP only), or [`set_file_info_fields`] (both), each
//! preserving every other resource and the unparsed tail byte-for-byte.
//!
//! ponytail: only the six IPTC-Core fields are synced between the channels;
//! extend the mapping when another shared field is needed.

use pictura_core::Document;

use crate::exif::{parse_exif, Exif};
use crate::image_resources::{
    decode_image_resources, decode_image_resources_with_len, encode_image_resources,
    frame_image_resource, EXIF_DATA_1, EXIF_DATA_3, IPTC_NAA, XMP_METADATA,
};
use crate::iptc::{encode_iptc, parse_iptc, Iptc};
use crate::xmp::{parse_xmp, patch_xmp_values, XmpField, XmpProperties};

pub mod template;

pub use template::{apply_template, export_template, MergeMode};

/// A document's decoded metadata. An absent resource yields the empty value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub exif: Exif,
    pub iptc: Iptc,
    /// The raw XMP packet as UTF-8 (lossy) text, empty when absent.
    pub xmp: String,
}

impl DocumentMetadata {
    pub fn is_empty(&self) -> bool {
        self.exif.is_empty() && self.iptc.is_empty() && self.xmp.is_empty()
    }
}

/// Gather the decoded metadata from a document's preserved image resources.
pub fn read_metadata(document: &Document) -> DocumentMetadata {
    let mut metadata = DocumentMetadata::default();
    for resource in decode_image_resources(document) {
        match resource.id {
            EXIF_DATA_1 | EXIF_DATA_3 if metadata.exif.is_empty() => {
                metadata.exif = parse_exif(&resource.data);
            }
            IPTC_NAA if metadata.iptc.is_empty() => {
                metadata.iptc = parse_iptc(&resource.data);
            }
            XMP_METADATA if metadata.xmp.is_empty() => {
                metadata.xmp = String::from_utf8_lossy(&resource.data).into_owned();
            }
            _ => {}
        }
    }
    metadata
}

/// Apply IPTC-IIM edits to a document in place, re-framing resource 1028 and
/// preserving every other resource and any unparsed tail byte-for-byte. An
/// empty value removes the record. Returns whether the section changed.
pub fn set_iptc_fields(document: &mut Document, fields: &[(u8, u8, String)]) -> bool {
    // Edit only when the parsed prefix re-encodes losslessly; the unparsed tail
    // below it is copied through untouched.
    let (mut resources, consumed) = decode_image_resources_with_len(document);
    if encode_image_resources(&resources) != document.image_resources[..consumed] {
        return false;
    }
    let mut iptc = resources
        .iter()
        .find(|r| r.id == IPTC_NAA)
        .map(|r| parse_iptc(&r.data))
        .unwrap_or_default();

    let mut changed = false;
    for (record, dataset, value) in fields {
        if value.is_empty() {
            // Clearing a field; an already-empty record is a no-op.
            if iptc.get(*record, *dataset).is_some_and(|v| !v.is_empty()) {
                changed |= iptc.remove(*record, *dataset);
            }
        } else {
            changed |= iptc.set(*record, *dataset, value.as_bytes());
        }
    }
    if !changed {
        return false;
    }

    let block = frame_image_resource(IPTC_NAA, "", &encode_iptc(&iptc));
    match resources.iter_mut().find(|r| r.id == IPTC_NAA) {
        Some(slot) => *slot = block,
        None => resources.push(block),
    }
    let mut section = encode_image_resources(&resources);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section.into();
    true
}

/// The document's parsed XMP properties, or empty when it has no XMP resource.
pub fn xmp_properties(document: &Document) -> XmpProperties {
    decode_image_resources(document)
        .into_iter()
        .find(|r| r.id == XMP_METADATA)
        .map(|r| parse_xmp(&String::from_utf8_lossy(&r.data)))
        .unwrap_or_default()
}

/// Apply single-value XMP property edits in place. See [`set_xmp_values`] for
/// the list-aware form.
pub fn set_xmp_fields(document: &mut Document, updates: &[(XmpField, String)]) -> bool {
    let updates: Vec<(XmpField, Vec<String>)> = updates
        .iter()
        .map(|(field, value)| (*field, vec![value.clone()]))
        .collect();
    set_xmp_values(document, &updates)
}

/// Apply XMP property edits in place, writing one `rdf:li` per value for the
/// list fields. A document with no XMP resource gets a minimal well-formed
/// packet only when the edit actually changes it; every other resource and any
/// unparsed tail are preserved byte-for-byte. An empty value vector removes the
/// property. Returns whether the packet changed.
pub fn set_xmp_values(document: &mut Document, updates: &[(XmpField, Vec<String>)]) -> bool {
    if updates.is_empty() {
        return false;
    }
    let (mut resources, consumed) = decode_image_resources_with_len(document);
    if encode_image_resources(&resources) != document.image_resources[..consumed] {
        return false;
    }

    let packet = match resources.iter().find(|r| r.id == XMP_METADATA) {
        Some(resource) => {
            let Ok(packet) = std::str::from_utf8(&resource.data) else {
                return false;
            };
            match patch_xmp_values(packet, updates) {
                Some(changed) if changed != packet => changed.into_bytes(),
                _ => return false,
            }
        }
        None => {
            let minimal = minimal_xmp_packet();
            match patch_xmp_values(minimal, updates) {
                Some(changed) if changed != minimal => changed.into_bytes(),
                _ => return false,
            }
        }
    };

    let block = frame_image_resource(XMP_METADATA, "", &packet);
    match resources.iter_mut().find(|r| r.id == XMP_METADATA) {
        Some(slot) => *slot = block,
        None => resources.push(block),
    }
    let mut section = encode_image_resources(&resources);
    section.extend_from_slice(&document.image_resources[consumed..]);
    document.image_resources = section.into();
    true
}

/// Apply the six shared IPTC-Core fields to both the XMP packet and the IIM
/// record in one call, so the two channels cannot disagree. A field whose XMP
/// side cannot be written is skipped in both channels. A mapped value is
/// truncated to the IIM length before either write, so the channels stay equal.
/// Returns whether either channel changed.
pub fn set_file_info_fields(document: &mut Document, fields: &[(u8, u8, String)]) -> bool {
    let fields: Vec<(u8, u8, String)> = fields
        .iter()
        .map(|(record, dataset, value)| {
            let value = if file_info_xmp_field(*record, *dataset).is_some() {
                truncate_to_iim(value)
            } else {
                value.clone()
            };
            (*record, *dataset, value)
        })
        .collect();

    let xmp_updates: Vec<(XmpField, String)> = fields
        .iter()
        .filter_map(|(record, dataset, value)| {
            file_info_xmp_field(*record, *dataset).map(|field| (field, value.clone()))
        })
        .collect();
    let xmp_changed = set_xmp_fields(document, &xmp_updates);

    let props = xmp_properties(document);
    let iptc_fields: Vec<(u8, u8, String)> = fields
        .iter()
        .filter(
            |(record, dataset, value)| match file_info_xmp_field(*record, *dataset) {
                Some(field) => xmp_has(&props, field, value),
                None => true,
            },
        )
        .cloned()
        .collect();
    let iptc_changed = set_iptc_fields(document, &iptc_fields);
    xmp_changed || iptc_changed
}

/// Truncate to the IIM record length (a `u16`), backing off to a UTF-8 boundary
/// so the same bytes go to both channels.
fn truncate_to_iim(value: &str) -> String {
    let max = u16::MAX as usize;
    if value.len() <= max {
        return value.to_string();
    }
    let mut end = max;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn file_info_xmp_field(record: u8, dataset: u8) -> Option<XmpField> {
    Some(match (record, dataset) {
        (2, 5) => XmpField::Title,
        (2, 80) => XmpField::Creator,
        (2, 116) => XmpField::Rights,
        (2, 120) => XmpField::Description,
        (2, 110) => XmpField::Credit,
        (2, 115) => XmpField::Source,
        _ => return None,
    })
}

fn xmp_has(props: &XmpProperties, field: XmpField, value: &str) -> bool {
    if value.is_empty() {
        // An empty request is satisfied by the property being absent, so a
        // successful clear in XMP does not block the matching IIM clear.
        return match field {
            XmpField::Creator => props.creator.is_empty(),
            XmpField::Title => props.title.is_none(),
            XmpField::Description => props.description.is_none(),
            XmpField::Rights => props.rights.is_none(),
            XmpField::Credit => props.credit.is_none(),
            XmpField::Source => props.source.is_none(),
            _ => false,
        };
    }
    match field {
        XmpField::Creator => props.creator.first().map(String::as_str) == Some(value),
        XmpField::Title => props.title.as_deref() == Some(value),
        XmpField::Description => props.description.as_deref() == Some(value),
        XmpField::Rights => props.rights.as_deref() == Some(value),
        XmpField::Credit => props.credit.as_deref() == Some(value),
        XmpField::Source => props.source.as_deref() == Some(value),
        _ => false,
    }
}

/// A minimal well-formed packet with a declared prefix per managed namespace,
/// so a first edit on a document with no XMP resource can be framed.
fn minimal_xmp_packet() -> &'static str {
    "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\n\
</rdf:Description>\n\
</rdf:RDF></x:xmpmeta>\n\
<?xpacket end=\"w\"?>"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_resources::encode_image_resources;
    use pictura_core::{BitDepth, ColorMode};

    fn resource(id: u16, name: &str, data: &[u8]) -> crate::image_resources::ImageResource {
        let mut raw = b"8BIM".to_vec();
        raw.extend_from_slice(&id.to_be_bytes());
        raw.push(name.len() as u8);
        raw.extend_from_slice(name.as_bytes());
        if !(name.len() + 1).is_multiple_of(2) {
            raw.push(0);
        }
        raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
        raw.extend_from_slice(data);
        if !data.len().is_multiple_of(2) {
            raw.push(0);
        }
        crate::image_resources::ImageResource {
            id,
            name: name.to_string(),
            data: data.to_vec(),
            raw,
        }
    }

    fn document_with(resources: &[crate::image_resources::ImageResource]) -> Document {
        let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
        doc.image_resources = encode_image_resources(resources).into();
        doc
    }

    #[test]
    fn gathers_exif_iptc_and_xmp() {
        let mut tiff = b"II".to_vec();
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&0u16.to_le_bytes()); // one IFD, zero entries
        tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD
        let mut iim = vec![0x1c, 2, 5];
        iim.extend_from_slice(&2u16.to_be_bytes());
        iim.extend_from_slice(b"Hi");

        let doc = document_with(&[
            resource(XMP_METADATA, "xmp", b"<x:xmpmeta/>"),
            resource(EXIF_DATA_1, "exif", &tiff),
            resource(IPTC_NAA, "iptc", &iim),
        ]);
        let metadata = read_metadata(&doc);
        assert_eq!(metadata.xmp, "<x:xmpmeta/>");
        assert_eq!(metadata.iptc.text(2, 5).as_deref(), Some("Hi"));
        // The zero-entry TIFF parses to an empty Exif, which is still "present".
        assert!(metadata.exif.is_empty());
    }

    #[test]
    fn no_metadata_resources_is_empty() {
        let doc = document_with(&[resource(crate::image_resources::ICC_PROFILE, "ICC", b"x")]);
        let metadata = read_metadata(&doc);
        assert!(metadata.is_empty());
    }

    fn tiff_with_make() -> Vec<u8> {
        let make = b"ACME\x00";
        let mut out = b"II".to_vec();
        out.extend_from_slice(&42u16.to_le_bytes());
        out.extend_from_slice(&8u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0x010fu16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&(make.len() as u32).to_le_bytes());
        out.extend_from_slice(&26u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(make);
        out
    }

    #[test]
    fn decodes_exif_from_the_third_resource() {
        let doc = document_with(&[resource(EXIF_DATA_3, "exif", &tiff_with_make())]);
        let metadata = read_metadata(&doc);
        assert_eq!(
            metadata.exif.get(0x010f),
            Some(&crate::exif::ExifValue::Ascii("ACME".into()))
        );
    }

    fn iim_object_name(value: &[u8]) -> Vec<u8> {
        let mut out = vec![0x1c, 2, 5];
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value);
        out
    }

    fn other_resources(doc: &Document) -> Vec<Vec<u8>> {
        decode_image_resources(doc)
            .into_iter()
            .filter(|r| r.id != IPTC_NAA)
            .map(|r| r.raw)
            .collect()
    }

    #[test]
    fn set_iptc_fields_edits_and_preserves_others() {
        let mut doc = document_with(&[
            resource(EXIF_DATA_1, "exif", &tiff_with_make()),
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", b"<x/>"),
        ]);
        let before = other_resources(&doc);
        assert!(set_iptc_fields(&mut doc, &[(2, 5, "New".into())]));
        assert_eq!(read_metadata(&doc).iptc.text(2, 5).as_deref(), Some("New"));
        assert_eq!(other_resources(&doc), before, "others are untouched");
    }

    #[test]
    fn set_iptc_fields_creates_a_resource_when_absent() {
        let mut doc = document_with(&[resource(XMP_METADATA, "xmp", b"<x/>")]);
        assert!(set_iptc_fields(&mut doc, &[(2, 5, "New".into())]));
        assert_eq!(read_metadata(&doc).iptc.text(2, 5).as_deref(), Some("New"));
    }

    #[test]
    fn set_iptc_fields_empty_removes_and_unchanged_is_a_noop() {
        let mut doc = document_with(&[resource(IPTC_NAA, "iptc", &iim_object_name(b"Old"))]);
        assert!(
            !set_iptc_fields(&mut doc, &[(2, 5, "Old".into())]),
            "same value changes nothing"
        );
        assert!(set_iptc_fields(&mut doc, &[(2, 5, String::new())]));
        assert_eq!(read_metadata(&doc).iptc.get(2, 5), None);
    }

    #[test]
    fn an_already_empty_record_is_not_touched() {
        let mut doc = document_with(&[resource(IPTC_NAA, "iptc", &iim_object_name(b""))]);
        assert!(!set_iptc_fields(&mut doc, &[(2, 5, String::new())]));
    }

    #[test]
    fn set_iptc_fields_preserves_an_unparsed_tail() {
        let mut doc = document_with(&[resource(IPTC_NAA, "iptc", &iim_object_name(b"Old"))]);
        doc.image_resources.extend_from_slice(b"garbage!");
        assert!(
            set_iptc_fields(&mut doc, &[(2, 5, "New".into())]),
            "the edit lands despite the tail"
        );
        assert_eq!(read_metadata(&doc).iptc.text(2, 5).as_deref(), Some("New"));
        assert!(
            doc.image_resources.ends_with(b"garbage!"),
            "the unparsed tail survives"
        );
    }

    const XMP_PACKET: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
xmlns:acme=\"http://example.com/acme/\" acme:Marker=\"keep\">\
<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Title</rdf:li></rdf:Alt></dc:title>\
<dc:creator><rdf:Seq><rdf:li>Ada</rdf:li></rdf:Seq></dc:creator>\
<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Caption</rdf:li></rdf:Alt></dc:description>\
<dc:rights><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Rights</rdf:li></rdf:Alt></dc:rights>\
<photoshop:Credit>Old Credit</photoshop:Credit>\
<photoshop:Source>Old Source</photoshop:Source>\
<acme:Note>keep me</acme:Note></rdf:Description></rdf:RDF></x:xmpmeta>";

    const ALT_XMP_PACKET: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:title><rdf:Alt><rdf:li xml:lang=\"fr\">Titre</rdf:li>\
<rdf:li xml:lang=\"x-default\">Title</rdf:li></rdf:Alt></dc:title>\
</rdf:Description></rdf:RDF></x:xmpmeta>";

    fn xmp_document() -> Document {
        document_with(&[
            resource(EXIF_DATA_1, "exif", &tiff_with_make()),
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes()),
        ])
    }

    #[test]
    fn xmp_properties_reads_the_managed_set() {
        let props = xmp_properties(&xmp_document());
        assert_eq!(props.title.as_deref(), Some("Old Title"));
        assert_eq!(props.creator, vec!["Ada"]);
        assert_eq!(props.description.as_deref(), Some("Old Caption"));
        assert_eq!(props.rights.as_deref(), Some("Old Rights"));
        assert_eq!(props.credit.as_deref(), Some("Old Credit"));
        assert_eq!(props.source.as_deref(), Some("Old Source"));
    }

    #[test]
    fn set_xmp_fields_creates_a_packet_when_absent() {
        let mut doc = document_with(&[resource(EXIF_DATA_1, "exif", &tiff_with_make())]);
        assert!(set_xmp_fields(
            &mut doc,
            &[(XmpField::Title, "New Title".into())]
        ));
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("New Title"));
        assert!(
            decode_image_resources(&doc)
                .iter()
                .any(|r| r.id == EXIF_DATA_1),
            "the other resource survives"
        );
    }

    #[test]
    fn set_xmp_fields_equal_value_is_a_noop() {
        let mut doc = xmp_document();
        let before = doc.image_resources.clone();
        assert!(!set_xmp_fields(
            &mut doc,
            &[(XmpField::Title, "Old Title".into())]
        ));
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn set_xmp_fields_leaves_an_unrecognised_packet_untouched() {
        let mut doc = document_with(&[resource(XMP_METADATA, "xmp", b"<not-xmp/>")]);
        let before = doc.image_resources.clone();
        assert!(!set_xmp_fields(
            &mut doc,
            &[(XmpField::Title, "New".into())]
        ));
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn set_xmp_fields_preserves_others_and_the_unparsed_tail() {
        let mut doc = xmp_document();
        doc.image_resources.extend_from_slice(b"garbage-tail");
        let exif_before = decode_image_resources(&doc)
            .into_iter()
            .find(|r| r.id == EXIF_DATA_1)
            .expect("exif")
            .raw;
        assert!(set_xmp_fields(&mut doc, &[(XmpField::Title, "New".into())]));
        assert!(doc.image_resources.ends_with(b"garbage-tail"));
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("New"));
        assert_eq!(
            decode_image_resources(&doc)
                .into_iter()
                .find(|r| r.id == EXIF_DATA_1)
                .expect("exif")
                .raw,
            exif_before,
            "the other resource is byte-identical"
        );
    }

    #[test]
    fn set_file_info_fields_syncs_xmp_and_iim() {
        let mut doc = xmp_document();
        assert!(set_file_info_fields(
            &mut doc,
            &[(2, 5, "Synced Title".into())]
        ));
        assert_eq!(
            read_metadata(&doc).iptc.text(2, 5).as_deref(),
            Some("Synced Title")
        );
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("Synced Title"));
    }

    #[test]
    fn set_file_info_fields_maps_all_six_core_fields() {
        let mut doc = document_with(&[resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes())]);
        let fields: Vec<(u8, u8, String)> = vec![
            (2, 5, "T".into()),
            (2, 80, "C".into()),
            (2, 116, "R".into()),
            (2, 120, "D".into()),
            (2, 110, "Cr".into()),
            (2, 115, "S".into()),
        ];
        assert!(set_file_info_fields(&mut doc, &fields));
        let props = xmp_properties(&doc);
        assert_eq!(props.title.as_deref(), Some("T"));
        assert_eq!(props.creator, vec!["C"]);
        assert_eq!(props.rights.as_deref(), Some("R"));
        assert_eq!(props.description.as_deref(), Some("D"));
        assert_eq!(props.credit.as_deref(), Some("Cr"));
        assert_eq!(props.source.as_deref(), Some("S"));
        assert!(
            read_metadata(&doc).xmp.contains("acme:Marker=\"keep\""),
            "the unknown namespace survives"
        );
        let iptc = read_metadata(&doc).iptc;
        assert_eq!(iptc.text(2, 5).as_deref(), Some("T"));
        assert_eq!(iptc.text(2, 80).as_deref(), Some("C"));
        assert_eq!(iptc.text(2, 116).as_deref(), Some("R"));
        assert_eq!(iptc.text(2, 120).as_deref(), Some("D"));
        assert_eq!(iptc.text(2, 110).as_deref(), Some("Cr"));
        assert_eq!(iptc.text(2, 115).as_deref(), Some("S"));
    }

    #[test]
    fn set_file_info_fields_unchanged_is_a_noop() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old Title")),
            resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes()),
        ]);
        let before = doc.image_resources.clone();
        assert!(!set_file_info_fields(
            &mut doc,
            &[(2, 5, "Old Title".into())]
        ));
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn set_file_info_fields_skips_a_field_xmp_cannot_hold() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", b"<not-xmp/>"),
        ]);
        let before = doc.image_resources.clone();
        assert!(!set_file_info_fields(&mut doc, &[(2, 5, "New".into())]));
        assert_eq!(
            doc.image_resources, before,
            "neither channel diverges on an unwritable packet"
        );
    }

    #[test]
    fn xmp_properties_missing_resource_is_empty() {
        let doc = document_with(&[resource(EXIF_DATA_1, "exif", &tiff_with_make())]);
        assert!(xmp_properties(&doc).is_empty());
    }

    #[test]
    fn set_file_info_fields_clears_both_channels() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old Title")),
            resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes()),
        ]);
        assert!(set_file_info_fields(&mut doc, &[(2, 5, String::new())]));
        let props = xmp_properties(&doc);
        assert_eq!(props.title, None, "XMP title is removed");
        assert_eq!(
            read_metadata(&doc).iptc.get(2, 5),
            None,
            "IIM Object Name is removed too"
        );
    }

    #[test]
    fn set_file_info_fields_updates_both_with_an_unparsed_tail() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes()),
        ]);
        doc.image_resources.extend_from_slice(b"garbage-tail");
        assert!(set_file_info_fields(
            &mut doc,
            &[(2, 5, "New Title".into())]
        ));
        assert_eq!(
            xmp_properties(&doc).title.as_deref(),
            Some("New Title"),
            "XMP is updated"
        );
        assert_eq!(
            read_metadata(&doc).iptc.text(2, 5).as_deref(),
            Some("New Title"),
            "IIM is updated and the two channels agree"
        );
        assert!(doc.image_resources.ends_with(b"garbage-tail"));
    }

    #[test]
    fn set_file_info_fields_truncates_oversized_values_symmetrically() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", XMP_PACKET.as_bytes()),
        ]);
        let huge = "x".repeat(70_000);
        assert!(set_file_info_fields(&mut doc, &[(2, 110, huge)]));
        let xmp = xmp_properties(&doc).credit.expect("credit");
        let iim = read_metadata(&doc).iptc.text(2, 110).expect("credit");
        assert_eq!(xmp, iim, "both channels hold the same bytes");
        assert_eq!(xmp.len(), u16::MAX as usize);
    }

    #[test]
    fn a_non_primary_alternative_syncs_both_channels() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", ALT_XMP_PACKET.as_bytes()),
        ]);
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("Title"));
        assert!(set_file_info_fields(&mut doc, &[(2, 5, "Titre".into())]));
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("Titre"));
        assert_eq!(
            read_metadata(&doc).iptc.text(2, 5).as_deref(),
            Some("Titre"),
            "the XMP scalar and the IIM value agree"
        );
    }

    #[test]
    fn a_scalar_noop_still_syncs_iim_without_collapsing() {
        let mut doc = document_with(&[
            resource(IPTC_NAA, "iptc", &iim_object_name(b"Old")),
            resource(XMP_METADATA, "xmp", ALT_XMP_PACKET.as_bytes()),
        ]);
        assert!(set_file_info_fields(&mut doc, &[(2, 5, "Title".into())]));
        assert_eq!(
            read_metadata(&doc).iptc.text(2, 5).as_deref(),
            Some("Title"),
            "IIM is synced to the existing scalar"
        );
        assert_eq!(xmp_properties(&doc).title.as_deref(), Some("Title"));
        assert!(
            read_metadata(&doc).xmp.contains("xml:lang=\"fr\""),
            "the sibling alternative survives the no-op"
        );
    }

    #[test]
    fn set_file_info_fields_with_no_fields_is_a_noop() {
        let mut doc = xmp_document();
        let before = doc.image_resources.clone();
        assert!(!set_file_info_fields(&mut doc, &[]));
        assert_eq!(doc.image_resources, before);
    }

    #[test]
    fn set_xmp_fields_empty_on_missing_resource_is_a_noop() {
        let mut doc = document_with(&[resource(EXIF_DATA_1, "exif", &tiff_with_make())]);
        let before = doc.image_resources.clone();
        assert!(!set_xmp_fields(
            &mut doc,
            &[(XmpField::Title, String::new())]
        ));
        assert_eq!(doc.image_resources, before);
    }
}
