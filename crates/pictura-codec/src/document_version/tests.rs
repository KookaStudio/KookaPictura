use super::*;
use crate::image_resources::EXIF_DATA_1;
use pictura_core::{BitDepth, ColorMode};

fn document(resources: &[ImageResource]) -> Document {
    let mut doc = Document::from_rgba("Layer", 1, 1, &[30, 60, 90, 255]);
    doc.image_resources = encode_image_resources(resources).into();
    doc
}

fn adobe_info(merged: bool) -> ImageResource {
    let mut data = vec![0, 0, 0, 1, u8::from(merged)];
    for name in ["Éditeur 🎨", "読み手"] {
        let units: Vec<_> = name.encode_utf16().chain([0]).collect();
        data.extend_from_slice(&(units.len() as u32).to_be_bytes());
        for unit in units {
            data.extend_from_slice(&unit.to_be_bytes());
        }
    }
    data.extend_from_slice(&73u32.to_be_bytes());
    frame_image_resource(VERSION_INFO, "version", &data)
}

fn round_trip(doc: &Document, psb: bool) -> Document {
    let output = crate::write_versioned(doc, psb, None, "test", false).unwrap();
    assert_eq!(output.bytes[5], if psb { 2 } else { 1 });
    crate::read_psd(&output.bytes).unwrap()
}

#[test]
fn version_info_preserves_unicode_and_versions() {
    let info = adobe_info(true);
    let doc = document(std::slice::from_ref(&info));
    for psb in [false, true] {
        let back = round_trip(&doc, psb);
        let decoded = document_version(&back).version_info.unwrap();
        assert_eq!(decoded.writer, "Éditeur 🎨");
        assert_eq!(decoded.reader, "読み手");
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.file_version, 73);
        assert!(decoded.has_real_merged_data);
        assert!(crate::decode_image_resources(&back).contains(&info));
    }
}

#[test]
fn version_info_handles_missing_or_unsupported_data() {
    let absent = document(&[]);
    assert!(document_version(&absent).version_info.is_none());
    assert!(document_version(&round_trip(&absent, false))
        .version_info
        .is_some());
    for bytes in [
        vec![0, 0, 0, 2, 1],
        vec![0, 0, 0, 1],
        vec![0, 0, 0, 1, 1, 255, 255, 255, 255],
    ] {
        let doc = document(&[frame_image_resource(VERSION_INFO, "", &bytes)]);
        let reading = document_version(&doc);
        assert!(reading.version_info.is_none());
        assert!(reading.access.is_read_only());
        assert!(crate::write_versioned(&doc, false, None, "test", false).is_err());
        let back = crate::read_psd(&crate::write_psd(&doc).unwrap()).unwrap();
        assert_eq!(back.image_resources, doc.image_resources);
    }
}

#[test]
fn version_info_reports_written_merged_image() {
    for psb in [false, true] {
        for merged in [false, true] {
            let mut doc = document(&[adobe_info(!merged)]);
            doc.composite.set_merged_output(merged);
            let back = round_trip(&doc, psb);
            let info = document_version(&back).version_info.unwrap();
            assert_eq!(info.has_real_merged_data, merged);
            assert_eq!(back.composite.merged_output(), merged);
            assert_eq!(info.file_version, 73);
        }
    }
}

#[test]
fn versioned_write_preserves_unrelated_native_bytes() {
    let xmp = br#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:demo="urn:demo"><demo:Note>keep &#38; me</demo:Note></rdf:Description></rdf:RDF>"#;
    let foreign = frame_image_resource(PRIVATE_VERSION, "Foreign plug-in", b"opaque");
    let exif = frame_image_resource(EXIF_DATA_1, "", b"test");
    let mut doc = document(&[
        foreign.clone(),
        exif.clone(),
        frame_image_resource(XMP_METADATA, "", xmp),
    ]);
    let opaque = pictura_core::LayerBlock {
        key: *b"Dvpt",
        data: vec![1, 2].into(),
    };
    assert!(doc.layers[0].set_metadata_block(opaque.key, Some(opaque.data.clone())));
    let back = round_trip(&doc, false);
    let reading = document_version(&back);
    assert_eq!(
        reading.stamp.unwrap().op_document_version,
        CURRENT_DOCUMENT_VERSION
    );
    assert_eq!(reading.access, VersionAccess::Editable);
    assert!(back.layers[0].extra_blocks().contains(&opaque));
    let resources = crate::decode_image_resources(&back);
    assert!(resources.contains(&foreign));
    assert!(resources.contains(&exif));
    let xmp = resources.iter().find(|r| r.id == XMP_METADATA).unwrap();
    let packet = std::str::from_utf8(&xmp.data).unwrap();
    assert!(packet.contains("<demo:Note>keep &#38; me</demo:Note>"));
    assert_eq!(tree::read(packet, VERSION_NAMESPACE).leaves.len(), 3);
}

#[test]
fn ambiguous_version_info_refuses_versioned_write() {
    for resources in [
        vec![adobe_info(false), adobe_info(true)],
        vec![frame_image_resource(VERSION_INFO, "", &[0, 0, 0, 2, 1])],
        vec![frame_image_resource(VERSION_INFO, "", &[0, 0, 0, 1, 1])],
    ] {
        let mut doc = document(&resources);
        doc.composite.set_merged_output(false);
        assert!(document_version(&doc).access.is_read_only());
        assert!(crate::write_versioned(&doc, false, None, "test", false).is_err());
        assert_eq!(
            crate::read_psd(&crate::write_psd(&doc).unwrap())
                .unwrap()
                .image_resources,
            doc.image_resources
        );
    }
}

#[test]
fn incomplete_resource_section_blocks_versioned_write() {
    let mut doc = document(&[]);
    let stamp = frame_image_resource(
        PRIVATE_VERSION,
        PRIVATE_NAME,
        b"KPVR\0\x01\0\x01\0\x01\0\x01x",
    );
    let mut truncated = stamp.raw.clone();
    truncated.pop();
    let mut hidden = b"????".to_vec();
    hidden.extend_from_slice(&stamp.raw);
    let mut good_prefix = stamp.raw;
    good_prefix.extend_from_slice(b"????");
    for section in [truncated, hidden, good_prefix] {
        doc.image_resources = section.into();
        assert!(document_version(&doc).access.is_read_only());
        assert!(crate::write_versioned(&doc, true, None, "test", false).is_err());
    }
}

#[test]
fn exposes_icc_resource_by_id() {
    assert_eq!(crate::ICC_PROFILE, 1039);
    let profile = frame_image_resource(crate::ICC_PROFILE, "ICC", b"profile");
    let doc = document(std::slice::from_ref(&profile));
    assert_eq!(crate::decode_image_resources(&doc), vec![profile]);
}

#[test]
fn preserves_unknown_blend_key_on_write() {
    let mut doc = document(&[]);
    doc.layers[0].blend_key = Some(*b"Dvpt");
    let back = crate::read_psd(&crate::write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].blend_key, Some(*b"Dvpt"));
}

#[test]
fn raw_write_preserves_resource_tail() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut resources = adobe_info(true).raw;
    resources.extend_from_slice(b"tail");
    doc.image_resources = resources.into();
    let back = crate::read_psd(&crate::write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.image_resources, doc.image_resources);
    assert!(crate::write_versioned(&doc, false, None, "test", false).is_err());
}

#[test]
fn versioned_write_marks_merged_transparency_without_marking_saved_channels() {
    let mut doc = document(&[]);
    for psb in [false, true] {
        let written = crate::write_versioned(&doc, psb, None, "test", false).unwrap();
        assert!(native_merged_has_transparency(&written.bytes, psb));
    }
    doc.composite
        .replace(pictura_core::PixelBuffer::new(1, 1, 3))
        .unwrap();
    for psb in [false, true] {
        let written = crate::write_versioned(&doc, psb, None, "test", false).unwrap();
        assert!(!native_merged_has_transparency(&written.bytes, psb));
    }
}

#[test]
fn unknown_group_blend_round_trips() {
    let mut doc = document(&[]);
    let mut group = pictura_core::Layer::new(pictura_core::LayerContent::Group(
        pictura_core::LayerStack::from_layers(vec![doc.layers.first().unwrap().clone()]),
    ));
    group.blend_key = Some(*b"zzzz");
    doc.layers = pictura_core::LayerStack::from_layers(vec![group]);
    let bytes = crate::write_psd(&doc).unwrap();
    let read = crate::read_psd(&bytes).unwrap();
    assert_eq!(read.layers.first().unwrap().blend_key, Some(*b"zzzz"));
    assert!(unsupported_document_feature(&read).is_some());
    let rewritten = crate::read_psd(&crate::write_psd(&read).unwrap()).unwrap();
    assert_eq!(rewritten.layers.first().unwrap().blend_key, Some(*b"zzzz"));
}
