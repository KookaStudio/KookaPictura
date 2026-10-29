use super::*;

use crate::image_resources::{
    decode_image_resources, encode_image_resources, frame_image_resource, ImageResource,
    EXIF_DATA_1, XMP_METADATA,
};
use crate::read_metadata;
use crate::xmp::parse_xmp;
use pictura_core::{BitDepth, ColorMode, Document};

const PACKET: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\" \
xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\" \
xmlns:acme=\"http://example.com/acme/\" acme:Marker=\"keep-me\">\
<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Title</rdf:li></rdf:Alt></dc:title>\
<photoshop:Credit>Old Credit</photoshop:Credit>\
<acme:Note>keep-me-too</acme:Note></rdf:Description></rdf:RDF></x:xmpmeta>";

const UNKNOWN_PROPERTY: &str = "<acme:Note>keep-me-too</acme:Note>";

const TITLE_ONLY_PACKET: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\
<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Old Title</rdf:li></rdf:Alt></dc:title>\
</rdf:Description></rdf:RDF></x:xmpmeta>";

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

fn resource(id: u16, data: &[u8]) -> ImageResource {
    frame_image_resource(id, "", data)
}

fn document() -> Document {
    document_with(PACKET)
}

fn document_with(packet: &str) -> Document {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources = encode_image_resources(&[
        resource(XMP_METADATA, packet.as_bytes()),
        resource(EXIF_DATA_1, &tiff_with_make()),
    ])
    .into();
    doc
}

fn exif_raw(doc: &Document) -> Vec<u8> {
    decode_image_resources(doc)
        .into_iter()
        .find(|r| r.id == EXIF_DATA_1)
        .expect("exif")
        .raw
}

fn title_only() -> XmpProperties {
    XmpProperties {
        title: Some("New Title".into()),
        ..Default::default()
    }
}

#[test]
fn append_fills_only_empty_fields() {
    let mut doc = document_with(TITLE_ONLY_PACKET);
    let template = XmpProperties {
        title: Some("New Title".into()),
        credit: Some("New Credit".into()),
        ..Default::default()
    };
    assert!(apply_template(&mut doc, &template, MergeMode::Append));
    let props = xmp_properties(&doc);
    assert_eq!(props.title.as_deref(), Some("Old Title"));
    assert_eq!(props.credit.as_deref(), Some("New Credit"));
}

#[test]
fn replace_overwrites_and_clears_omitted_fields() {
    let mut doc = document();
    assert!(apply_template(&mut doc, &title_only(), MergeMode::Replace));
    let props = xmp_properties(&doc);
    assert_eq!(props.title.as_deref(), Some("New Title"));
    assert_eq!(props.credit, None, "the omitted credit is cleared");
    assert_eq!(
        read_metadata(&doc).iptc.get(2, 110),
        None,
        "IIM cleared too"
    );
}

#[test]
fn keep_original_replaces_only_template_fields() {
    let mut doc = document();
    assert!(apply_template(
        &mut doc,
        &title_only(),
        MergeMode::KeepOriginalReplaceMatching
    ));
    let props = xmp_properties(&doc);
    assert_eq!(props.title.as_deref(), Some("New Title"));
    assert_eq!(props.credit.as_deref(), Some("Old Credit"));
}

#[test]
fn an_empty_template_is_a_noop_for_append_and_keep_original() {
    for mode in [MergeMode::Append, MergeMode::KeepOriginalReplaceMatching] {
        let mut doc = document();
        let before = doc.image_resources.clone();
        assert!(
            !apply_template(&mut doc, &XmpProperties::default(), mode),
            "{mode:?} changed something"
        );
        assert_eq!(doc.image_resources, before, "{mode:?}");
    }
}

#[test]
fn unknown_namespace_and_exif_survive_every_mode() {
    let template = XmpProperties {
        title: Some("New Title".into()),
        credit: Some("New Credit".into()),
        ..Default::default()
    };
    for mode in [
        MergeMode::Append,
        MergeMode::Replace,
        MergeMode::KeepOriginalReplaceMatching,
    ] {
        let mut doc = document();
        let exif = exif_raw(&doc);
        apply_template(&mut doc, &template, mode);
        let raw = read_metadata(&doc).xmp;
        assert!(raw.contains(UNKNOWN_PROPERTY), "{mode:?}: {raw}");
        assert!(raw.contains("acme:Marker=\"keep-me\""), "{mode:?}");
        assert_eq!(exif_raw(&doc), exif, "{mode:?}");
    }
}

#[test]
fn the_six_shared_fields_reach_both_xmp_and_iim() {
    let mut doc = document();
    let template = XmpProperties {
        title: Some("T".into()),
        creator: vec!["C".into()],
        rights: Some("R".into()),
        description: Some("D".into()),
        credit: Some("Cr".into()),
        source: Some("S".into()),
        ..Default::default()
    };
    assert!(apply_template(&mut doc, &template, MergeMode::Replace));
    let props = xmp_properties(&doc);
    assert_eq!(props.title.as_deref(), Some("T"));
    assert_eq!(props.creator, vec!["C"]);
    assert_eq!(props.rights.as_deref(), Some("R"));
    assert_eq!(props.description.as_deref(), Some("D"));
    assert_eq!(props.credit.as_deref(), Some("Cr"));
    assert_eq!(props.source.as_deref(), Some("S"));
    let iptc = read_metadata(&doc).iptc;
    assert_eq!(iptc.text(2, 5).as_deref(), Some("T"));
    assert_eq!(iptc.text(2, 80).as_deref(), Some("C"));
    assert_eq!(iptc.text(2, 116).as_deref(), Some("R"));
    assert_eq!(iptc.text(2, 120).as_deref(), Some("D"));
    assert_eq!(iptc.text(2, 110).as_deref(), Some("Cr"));
    assert_eq!(iptc.text(2, 115).as_deref(), Some("S"));
}

#[test]
fn export_carries_managed_fields_only() {
    let text = String::from_utf8(export_template(&document())).expect("utf8");
    assert!(!text.contains("acme"), "no unknown-namespace property");
    assert!(!text.contains("ACME"), "no EXIF bytes");
    let props = parse_xmp(&text);
    assert_eq!(props.title.as_deref(), Some("Old Title"));
    assert_eq!(props.credit.as_deref(), Some("Old Credit"));
}

const CREATOR_PACKET: &str = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
<dc:creator><rdf:Seq><rdf:li>Ada</rdf:li><rdf:li>Grace</rdf:li></rdf:Seq></dc:creator>\
</rdf:Description></rdf:RDF></x:xmpmeta>";

fn document_without_xmp() -> Document {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources =
        encode_image_resources(&[resource(EXIF_DATA_1, &tiff_with_make())]).into();
    doc
}

#[test]
fn an_oversized_shared_value_is_truncated_symmetrically() {
    let mut doc = document();
    let huge = "x".repeat(70_000);
    let template = XmpProperties {
        title: Some(huge.clone()),
        creator: vec![huge.clone(), "Grace".into()],
        ..Default::default()
    };
    assert!(apply_template(
        &mut doc,
        &template,
        MergeMode::KeepOriginalReplaceMatching
    ));
    let props = xmp_properties(&doc);
    let iim_title = read_metadata(&doc).iptc.text(2, 5);
    assert_eq!(
        props.title.as_deref(),
        iim_title.as_deref(),
        "title: both channels agree"
    );
    assert_eq!(
        props.title.as_ref().map(String::len),
        Some(u16::MAX as usize)
    );
    let iim_creator = read_metadata(&doc).iptc.text(2, 80).expect("IIM creator");
    assert_eq!(
        props.creator.first().map(String::len),
        Some(u16::MAX as usize),
        "the first creator item is capped like IIM"
    );
    assert_eq!(
        props.creator.first(),
        Some(&iim_creator),
        "creator[0] equals the IIM record"
    );
}

#[test]
fn a_two_item_creator_is_applied_as_a_list() {
    let mut doc = document_with(TITLE_ONLY_PACKET);
    let template = XmpProperties {
        creator: vec!["Ada".into(), "Grace".into()],
        ..Default::default()
    };
    assert!(apply_template(&mut doc, &template, MergeMode::Replace));
    assert_eq!(xmp_properties(&doc).creator, vec!["Ada", "Grace"]);
}

#[test]
fn replace_clears_an_omitted_list() {
    let mut doc = document_with(CREATOR_PACKET);
    assert!(apply_template(&mut doc, &title_only(), MergeMode::Replace));
    assert!(xmp_properties(&doc).creator.is_empty());
}

#[test]
fn replace_shrinks_a_list_to_a_single_item() {
    let mut doc = document_with(CREATOR_PACKET);
    let template = XmpProperties {
        creator: vec!["Ada".into()],
        ..Default::default()
    };
    assert!(apply_template(&mut doc, &template, MergeMode::Replace));
    assert_eq!(xmp_properties(&doc).creator, vec!["Ada"]);
}

#[test]
fn replace_clears_an_empty_list() {
    let mut doc = document_with(CREATOR_PACKET);
    let template = XmpProperties {
        creator: Vec::new(),
        ..Default::default()
    };
    assert!(apply_template(&mut doc, &template, MergeMode::Replace));
    assert!(xmp_properties(&doc).creator.is_empty());
    assert_eq!(read_metadata(&doc).iptc.get(2, 80), None);
}

#[test]
fn an_empty_template_replace_on_xmpless_document_is_a_noop() {
    let mut doc = document_without_xmp();
    let before = doc.image_resources.clone();
    assert!(!apply_template(
        &mut doc,
        &XmpProperties::default(),
        MergeMode::Replace
    ));
    assert_eq!(doc.image_resources, before);
}
