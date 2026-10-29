//! Unit tests for the ICC read/normalize pass.

use super::*;
use crate::image_resources::decode_image_resources;
use pictura_core::{BitDepth, Channel, ColorMode, Layer, PsdRect};

/// One `8BIM` resource block with an empty Pascal name.
fn block(id: u16, data: &[u8]) -> Vec<u8> {
    let mut out = b"8BIM".to_vec();
    out.extend_from_slice(&id.to_be_bytes());
    out.push(0); // empty Pascal name
    out.push(0); // name pad (1-byte name is odd)
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        out.push(0);
    }
    out
}

fn icc_resource(profile: &[u8]) -> Vec<u8> {
    block(ICC_PROFILE, profile)
}

fn xmp_resource(data: &[u8]) -> Vec<u8> {
    block(crate::image_resources::XMP_METADATA, data)
}

fn rgb_document() -> Document {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = vec![200, 10, 100, 20, 50, 30].into();
    doc
}

#[test]
fn non_srgb_profile_converts_and_drops_the_resource() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(&profile).into();
    let original = doc.composite.data.clone();

    let out = apply_icc(doc);
    assert_eq!(out.source_icc.as_deref(), Some(profile.as_slice()));
    assert_ne!(out.composite.data, original, "pixels were converted");
    assert!(
        decode_image_resources(&out)
            .iter()
            .all(|r| r.id != ICC_PROFILE),
        "the stale profile was removed"
    );
    assert!(out.image_resources.is_empty(), "no other resources remain");
}

#[test]
fn cmyk_document_is_left_to_color_mode_pass() {
    let profile = Profile::coated_cmyk().to_icc();
    let mut doc = Document::new(1, 1, ColorMode::Cmyk, BitDepth::Eight);
    doc.source_mode = Some(ColorMode::Cmyk);
    doc.composite.data = vec![10, 20, 30, 40].into();
    doc.image_resources = icc_resource(&profile).into();
    let before = doc.composite.data.to_vec();
    let out = apply_icc_policy(doc, Policy::Convert);
    assert_eq!(out.composite.data.to_vec(), before);
    assert_eq!(out.image_resources, icc_resource(&profile));
    assert!(out.source_icc.is_none());
}

#[test]
fn srgb_profile_is_left_untouched() {
    let profile = Profile::srgb().to_icc();
    let mut doc = rgb_document();
    let section = icc_resource(&profile);
    doc.image_resources = section.clone().into();
    let original = doc.composite.data.clone();

    let out = apply_icc(doc);
    assert!(out.source_icc.is_none());
    assert_eq!(out.composite.data, original);
    assert_eq!(out.image_resources, section);
}

#[test]
fn undecodable_profile_is_ignored() {
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(b"not an ICC profile").into();
    let original = doc.composite.data.clone();
    let out = apply_icc(doc);
    assert!(out.source_icc.is_none());
    assert_eq!(out.composite.data, original);
}

#[test]
fn a_layer_with_incomplete_color_channels_is_skipped() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(&profile).into();
    {
        let _ = doc.layers.push(Layer {
            name: "partial".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            channels: vec![Channel {
                id: 0,
                data: vec![1, 2].into(),
            }],
            ..Default::default()
        });
    };
    let original = doc.layers[0].channels[0].data.clone();
    let out = apply_icc(doc);
    assert_eq!(out.layers[0].channels[0].data, original);
}

#[test]
fn grayscale_document_is_left_untouched() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite.data = vec![10, 200].into();
    let section = icc_resource(&profile);
    doc.image_resources = section.clone().into();

    let out = apply_icc(doc);
    assert!(out.source_icc.is_none(), "grayscale is not converted");
    assert_eq!(out.composite.data, vec![10, 200]);
    assert_eq!(out.image_resources, section, "the profile is kept");
}

#[test]
fn other_resources_survive_conversion() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    let mut section = icc_resource(&profile);
    section.extend(xmp_resource(b"<x/>"));
    doc.image_resources = section.into();

    let out = apply_icc(doc);
    let resources = decode_image_resources(&out);
    assert!(resources.iter().all(|r| r.id != ICC_PROFILE));
    let xmp = resources
        .iter()
        .find(|r| r.id == crate::image_resources::XMP_METADATA)
        .expect("XMP survives");
    assert_eq!(xmp.data, b"<x/>");
}

#[test]
fn malformed_section_is_a_noop() {
    let mut doc = rgb_document();
    doc.image_resources = b"not a resource section".to_vec().into();
    let original = doc.composite.data.clone();
    let out = apply_icc(doc);
    assert!(out.source_icc.is_none());
    assert_eq!(out.composite.data, original);
}

#[test]
fn layer_channels_are_selected_by_id_not_storage_order() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(&profile).into();
    {
        let _ = doc.layers.push(Layer {
            name: "bgr".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 1,
            },
            // Stored G, B, R; id-based selection must convert [R, G, B].
            channels: vec![
                Channel {
                    id: 1,
                    data: vec![10].into(),
                },
                Channel {
                    id: 2,
                    data: vec![20].into(),
                },
                Channel {
                    id: 0,
                    data: vec![30].into(),
                },
            ],
            ..Default::default()
        });
    };

    let out = apply_icc(doc);
    let id = |id: i16| {
        out.layers[0]
            .channels
            .iter()
            .find(|c| c.id == id)
            .expect("channel")
            .data[0]
    };
    let src = Profile::from_icc(&profile).expect("profile");
    let expected =
        convert_planes(&[vec![30], vec![10], vec![20]], &src, &Profile::srgb()).expect("converts");
    assert_eq!(
        [id(0), id(1), id(2)],
        [expected[0][0], expected[1][0], expected[2][0]]
    );
}

#[test]
fn assign_leaves_composite_and_layers_byte_identical() {
    let mut doc = Document::from_rgba("px", 2, 1, &[10, 20, 30, 255, 40, 50, 60, 128]);
    let before = doc.clone();
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert_eq!(doc.composite, before.composite);
    assert_eq!(doc.layers[0].channels, before.layers[0].channels);
    assert!(doc.document_icc.is_some());
}

#[test]
fn assign_frames_a_1039_block_for_a_non_srgb_profile() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    let resources = decode_image_resources(&doc);
    let icc = resources
        .iter()
        .find(|r| r.id == ICC_PROFILE)
        .expect("a 1039 block is framed");
    assert_eq!(icc.data, profile);
    assert_eq!(doc.document_icc.as_deref(), Some(profile.as_slice()));
}

#[test]
fn assigning_srgb_removes_1039_and_clears_document_icc() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources = icc_resource(&profile).into();
    doc.document_icc = Some(profile.into());
    assign_document_profile(&mut doc, &Profile::srgb());
    assert!(doc.document_icc.is_none());
    assert!(decode_image_resources(&doc)
        .iter()
        .all(|r| r.id != ICC_PROFILE));
}

#[test]
fn assign_preserves_other_resources() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources = xmp_resource(b"<x/>").into();
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    let resources = decode_image_resources(&doc);
    let xmp = resources
        .iter()
        .find(|r| r.id == crate::image_resources::XMP_METADATA)
        .expect("the XMP block survives");
    assert_eq!(xmp.data, b"<x/>");
    assert!(resources.iter().any(|r| r.id == ICC_PROFILE));
}

#[test]
fn convert_changes_pixels_and_tags_destination() {
    let mut doc = rgb_document();
    let before = doc.composite.data.clone();
    assert!(convert_document(&mut doc, &Profile::adobe_rgb()));
    assert_ne!(doc.composite.data, before, "the composite was converted");
    assert!(doc.document_icc.is_some());
    assert!(decode_image_resources(&doc)
        .iter()
        .any(|r| r.id == ICC_PROFILE));
}

#[test]
fn convert_with_incomplete_layer_channels_is_not_an_error() {
    let mut doc = rgb_document();
    {
        let _ = doc.layers.push(Layer {
            name: "partial".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            channels: vec![Channel {
                id: 0,
                data: vec![1, 2].into(),
            }],
            ..Default::default()
        });
    };
    let before = doc.layers[0].channels[0].data.clone();
    assert!(convert_document(&mut doc, &Profile::adobe_rgb()));
    assert_eq!(doc.layers[0].channels[0].data, before);
}

#[test]
fn buffer_to_srgb_is_identity_without_a_document_profile() {
    let doc = rgb_document();
    let converted = buffer_to_srgb(&doc, &doc.composite);
    assert!(
        matches!(converted, Cow::Borrowed(_)),
        "an unprofiled document borrows its composite without a copy"
    );
    assert_eq!(*converted, doc.composite);
}

#[test]
fn buffer_to_srgb_converts_a_profiled_document() {
    let mut doc = rgb_document();
    let before = doc.composite.data.clone();
    doc.document_icc = Some(Profile::adobe_rgb().to_icc().into());
    let out = buffer_to_srgb(&doc, &doc.composite);
    assert_ne!(out.data, before);
    assert_eq!(out.channels, doc.composite.channels);
}

#[test]
fn buffer_to_srgb_reuses_the_transform_and_follows_profile_changes() {
    let mut doc = rgb_document();
    doc.composite.data = (0..doc.composite.data.len())
        .map(|i| (i * 7 % 251) as u8)
        .collect();
    doc.document_icc = Some(Profile::adobe_rgb().to_icc().into());

    let first = buffer_to_srgb(&doc, &doc.composite).into_owned();
    let second = buffer_to_srgb(&doc, &doc.composite).into_owned();
    assert_eq!(first.data, second.data, "repeated conversions agree");

    // The cached path equals a conversion that re-parses the profile.
    let src = Profile::from_icc(doc.document_icc.as_deref().unwrap()).unwrap();
    let fresh = convert_buffer(&doc.composite, &src, &Profile::srgb()).unwrap();
    assert_eq!(first.data, fresh.data, "cached path equals a fresh parse");

    // Changing the working profile invalidates the cached transform.
    doc.document_icc = Some(Profile::pro_photo().to_icc().into());
    let changed = buffer_to_srgb(&doc, &doc.composite).into_owned();
    assert_ne!(changed.data, first.data, "a new profile is used");
}

fn rgb_layer(name: &str) -> Layer {
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 2,
        },
        channels: vec![
            Channel {
                id: 0,
                data: vec![200, 10].into(),
            },
            Channel {
                id: 1,
                data: vec![100, 20].into(),
            },
            Channel {
                id: 2,
                data: vec![50, 30].into(),
            },
        ],
        ..Default::default()
    }
}

#[test]
fn convert_reaches_pixel_layers_nested_in_groups() {
    let mut doc = rgb_document();
    let child = rgb_layer("child");
    let before = child.channels[0].data.clone();
    {
        let _ = doc.layers.push(Layer {
            name: "group".into(),
            is_group: true,
            children: pictura_core::LayerStack::from_layers(vec![child]),
            ..Default::default()
        });
    };
    assert!(convert_document(&mut doc, &Profile::adobe_rgb()));
    assert_ne!(
        doc.layers[0].children[0].channels[0].data, before,
        "the nested layer was converted"
    );
}

#[test]
fn read_conversion_reaches_pixel_layers_nested_in_groups() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(&profile).into();
    let child = rgb_layer("child");
    let before = child.channels[0].data.clone();
    {
        let _ = doc.layers.push(Layer {
            name: "group".into(),
            is_group: true,
            children: pictura_core::LayerStack::from_layers(vec![child]),
            ..Default::default()
        });
    };
    let out = apply_icc(doc);
    assert_ne!(out.layers[0].children[0].channels[0].data, before);
}

#[test]
fn assign_preserves_an_unparsed_resource_tail() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    let mut section = icc_resource(&profile);
    let tail = b"nope\x00\x01\x02\x03tail".to_vec();
    section.extend_from_slice(&tail);
    doc.image_resources = section.into();
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert!(
        doc.image_resources.ends_with(&tail),
        "the unknown-signature tail survives"
    );
    assert!(decode_image_resources(&doc)
        .iter()
        .any(|r| r.id == ICC_PROFILE));
    assert_eq!(doc.document_icc.as_deref(), Some(profile.as_slice()));
}

#[test]
fn read_conversion_preserves_an_unparsed_resource_tail() {
    let profile = Profile::adobe_rgb().to_icc();
    let mut doc = rgb_document();
    let mut section = icc_resource(&profile);
    let tail = b"nope\x00\x01\x02\x03tail".to_vec();
    section.extend_from_slice(&tail);
    doc.image_resources = section.into();
    let out = apply_icc(doc);
    assert_eq!(
        out.image_resources, tail,
        "the unparsed tail survives read-normalisation"
    );
    assert!(decode_image_resources(&out)
        .iter()
        .all(|r| r.id != ICC_PROFILE));
    assert!(out.document_icc.is_none());
}

#[test]
fn convert_document_is_a_noop_for_a_grayscale_document() {
    let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite.data = vec![10, 200].into();
    let before = doc.clone();
    assert!(!convert_document(&mut doc, &Profile::adobe_rgb()));
    assert_eq!(doc, before);
}

#[test]
fn assign_document_profile_is_a_noop_for_a_grayscale_document() {
    let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert!(doc.document_icc.is_none());
    assert!(doc.image_resources.is_empty());
}

#[test]
fn assign_then_convert_back_to_srgb_leaves_it_untagged() {
    let mut doc = Document::from_rgba("px", 2, 1, &[10, 20, 30, 255, 40, 50, 60, 255]);
    assign_document_profile(&mut doc, &Profile::adobe_rgb());
    assert!(doc.document_icc.is_some());
    assert!(convert_document(&mut doc, &Profile::srgb()));
    assert!(doc.document_icc.is_none());
    assert!(decode_image_resources(&doc)
        .iter()
        .all(|r| r.id != ICC_PROFILE));
}

fn has_icc(resources: &[ImageResource]) -> bool {
    resources.iter().any(|r| r.id == ICC_PROFILE)
}

#[test]
fn policy_preserve_keeps_the_embedded_profile_and_pixels() {
    let raw = include_bytes!("../../tests/fixtures/icc_profile.psd");
    let embedded = include_bytes!("../../tests/fixtures/psd_icc_rgb.icc");
    let preserved = crate::read_psd_with(raw, Policy::Preserve).expect("parses");
    let off = crate::read_psd_with(raw, Policy::Off).expect("parses");

    assert_eq!(preserved.composite.data, off.composite.data);
    assert_eq!(preserved.layers, off.layers, "layer pixels unchanged");
    let icc = decode_image_resources(&preserved)
        .into_iter()
        .find(|r| r.id == ICC_PROFILE)
        .expect("resource 1039 is kept");
    assert_eq!(icc.data.as_slice(), embedded);
    assert_eq!(preserved.document_icc.as_deref(), Some(embedded.as_slice()));
    assert!(preserved.source_icc.is_none());
}

#[test]
fn policy_convert_is_the_read_normalisation_and_drops_the_profile() {
    let raw = include_bytes!("../../tests/fixtures/icc_profile.psd");
    let converted = crate::read_psd_with(raw, Policy::Convert).expect("parses");
    let preserved = crate::read_psd_with(raw, Policy::Preserve).expect("parses");

    assert_ne!(converted.composite.data, preserved.composite.data);
    assert!(converted.source_icc.is_some());
    assert!(converted.document_icc.is_none());
    assert!(!has_icc(&decode_image_resources(&converted)));

    let legacy = crate::read_psd(raw).expect("parses");
    assert_eq!(legacy.composite.data, converted.composite.data);
    assert_eq!(legacy.source_icc, converted.source_icc);
}

#[test]
fn policy_off_untags_and_leaves_pixels_byte_identical() {
    let raw = include_bytes!("../../tests/fixtures/icc_profile.psd");
    let off = crate::read_psd_with(raw, Policy::Off).expect("parses");
    let preserved = crate::read_psd_with(raw, Policy::Preserve).expect("parses");

    assert_eq!(off.composite.data, preserved.composite.data);
    assert!(!has_icc(&decode_image_resources(&off)));
    assert!(off.document_icc.is_none());
    assert!(off.source_icc.is_none());
}

#[test]
fn policy_leaves_a_profile_less_file_unchanged() {
    let raw = include_bytes!("../../tests/fixtures/two_layers.psd");
    let baseline = crate::read_psd(raw).expect("parses");
    for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
        let doc = crate::read_psd_with(raw, policy).expect("parses");
        assert_eq!(doc.composite.data, baseline.composite.data, "{policy:?}");
        assert_eq!(doc.layers, baseline.layers, "{policy:?}");
        assert!(doc.document_icc.is_none(), "{policy:?}");
        assert!(doc.source_icc.is_none(), "{policy:?}");
    }
}

#[test]
fn policy_leaves_an_srgb_profile_unchanged() {
    let profile = Profile::srgb().to_icc();
    for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
        let mut doc = rgb_document();
        let section = icc_resource(&profile);
        doc.image_resources = section.clone().into();
        let original = doc.composite.data.clone();

        let out = apply_icc_policy(doc, policy);
        assert_eq!(out.composite.data, original, "{policy:?}");
        assert_eq!(out.image_resources, section, "{policy:?}");
        assert!(out.document_icc.is_none(), "{policy:?}");
        assert!(out.source_icc.is_none(), "{policy:?}");
    }
}

#[test]
fn policy_leaves_a_grayscale_document_unchanged() {
    let profile = Profile::adobe_rgb().to_icc();
    for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
        let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
        doc.composite.data = vec![10, 200].into();
        let section = icc_resource(&profile);
        doc.image_resources = section.clone().into();

        let out = apply_icc_policy(doc, policy);
        assert_eq!(out.composite.data, vec![10, 200], "{policy:?}");
        assert_eq!(out.image_resources, section, "{policy:?}");
        assert!(out.source_icc.is_none(), "{policy:?}");
        assert!(out.document_icc.is_none(), "{policy:?}");
    }
}

#[test]
fn policy_leaves_an_undecodable_profile_unchanged() {
    for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
        let mut doc = rgb_document();
        doc.image_resources = icc_resource(b"not an ICC profile").into();
        let original = doc.image_resources.clone();

        let out = apply_icc_policy(doc, policy);
        assert_eq!(out.image_resources, original, "{policy:?}");
        assert!(out.source_icc.is_none(), "{policy:?}");
        assert!(out.document_icc.is_none(), "{policy:?}");
    }
}

#[test]
fn policy_preserve_rejects_a_profile_the_pixels_cannot_transform() {
    // A CMYK profile on a document `normalize` turned into RGB: it parses and
    // is not sRGB, but no RGB->sRGB transform can be built from it.
    let mut profile = include_bytes!("../../tests/fixtures/psd_icc_rgb.icc").to_vec();
    profile[16..20].copy_from_slice(b"CMYK");
    let mut doc = rgb_document();
    doc.image_resources = icc_resource(&profile).into();
    let composite = doc.composite.data.clone();
    let resources = doc.image_resources.clone();

    let out = apply_icc_policy(doc, Policy::Preserve);
    assert!(out.document_icc.is_none());
    assert!(out.source_icc.is_none());
    assert_eq!(out.composite.data, composite);
    assert_eq!(out.image_resources, resources, "1039 is left in place");
}

#[test]
fn policy_leaves_a_cmyk_fixture_unchanged() {
    let raw = include_bytes!("../../tests/fixtures/cmyk.psd");
    let baseline = crate::read_psd(raw).expect("parses");
    for policy in [Policy::Preserve, Policy::Convert, Policy::Off] {
        let doc = crate::read_psd_with(raw, policy).expect("parses");
        assert_eq!(doc.composite.data, baseline.composite.data, "{policy:?}");
        assert_eq!(doc.image_resources, baseline.image_resources, "{policy:?}");
        assert!(doc.document_icc.is_none(), "{policy:?}");
        assert!(doc.source_icc.is_none(), "{policy:?}");
    }
}

use crate::common::{MODE_CMYK, MODE_GRAYSCALE, MODE_LAB, MODE_RGB};

/// A converted CMYK source: RGB working pixels, the CMYK mode recorded, and
/// `source_depth` set for a 16/32-bit read that retained no planes.
fn converted_cmyk_document(source_depth: Option<BitDepth>) -> Document {
    let mut doc = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.source_mode = Some(ColorMode::Cmyk);
    doc.source_depth = source_depth;
    doc.composite.data = vec![200, 10, 100, 20, 50, 30].into();
    doc
}

/// A real RGB profile with its ICC data-space signature patched.
fn profiled(data_space: &[u8; 4]) -> Vec<u8> {
    let mut profile = include_bytes!("../../tests/fixtures/psd_icc_rgb.icc").to_vec();
    profile[16..20].copy_from_slice(data_space);
    profile
}

/// The image-resource section of a written PSD (26-byte header, then a
/// 4-byte color-mode-data length and data, then the resource length and data).
fn written_resources(psd: &[u8]) -> Vec<u8> {
    let color_mode_data = u32::from_be_bytes(psd[26..30].try_into().unwrap()) as usize;
    let at = 30 + color_mode_data;
    let len = u32::from_be_bytes(psd[at..at + 4].try_into().unwrap()) as usize;
    psd[at + 4..at + 4 + len].to_vec()
}

fn header_mode(psd: &[u8]) -> u16 {
    u16::from_be_bytes(psd[24..26].try_into().unwrap())
}

fn decode_section(section: Vec<u8>) -> Vec<ImageResource> {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.image_resources = section.into();
    decode_image_resources(&doc)
}

#[test]
fn converted_sixteen_bit_cmyk_saves_as_rgb_without_its_profile() {
    let mut doc = converted_cmyk_document(Some(BitDepth::Sixteen));
    doc.image_resources = icc_resource(&profiled(b"CMYK")).into();

    let out = crate::write_psd(&doc).expect("writes");
    assert_eq!(header_mode(&out), MODE_RGB, "the converted mode is RGB");
    let resources = decode_section(written_resources(&out));
    assert!(
        resources.iter().all(|r| r.id != ICC_PROFILE),
        "the stale CMYK profile was dropped"
    );
}

#[test]
fn eight_bit_cmyk_keeps_its_profile() {
    let mut doc = converted_cmyk_document(None);
    let section = icc_resource(&profiled(b"CMYK"));
    doc.image_resources = section.clone().into();

    let out = crate::write_psd(&doc).expect("writes");
    assert_eq!(header_mode(&out), MODE_CMYK, "the CMYK header is kept");
    assert_eq!(
        written_resources(&out),
        section,
        "the matching profile is re-emitted byte-for-byte"
    );
}

#[test]
fn matching_rgb_profile_is_re_emitted_byte_for_byte() {
    let mut doc = rgb_document();
    let section = icc_resource(&profiled(b"RGB "));
    doc.image_resources = section.clone().into();

    let out = crate::write_psd(&doc).expect("writes");
    assert_eq!(header_mode(&out), MODE_RGB);
    assert_eq!(written_resources(&out), section);
}

#[test]
fn mismatched_profile_leaves_a_sibling_xmp_intact() {
    let mut doc = converted_cmyk_document(Some(BitDepth::Sixteen));
    let mut section = icc_resource(&profiled(b"CMYK"));
    section.extend(xmp_resource(b"<x/>"));
    doc.image_resources = section.into();
    let xmp = xmp_resource(b"<x/>");

    let out = crate::write_psd(&doc).expect("writes");
    let resources = decode_section(written_resources(&out));
    assert!(resources.iter().all(|r| r.id != ICC_PROFILE));
    let kept = resources
        .iter()
        .find(|r| r.id == crate::image_resources::XMP_METADATA)
        .expect("XMP survives");
    assert_eq!(kept.raw, xmp, "the XMP block is byte-identical");
}

#[test]
fn profile_too_short_to_classify_is_preserved() {
    let mut doc = rgb_document();
    let section = icc_resource(b"short");
    doc.image_resources = section.clone().into();

    let out = crate::write_psd(&doc).expect("writes");
    assert_eq!(written_resources(&out), section);
}

/// A flat PSD with an image-resource section carrying one `1039` block whose
/// data is `icc`, then a raw merged composite of the native `planes`.
fn flat_psd_with_icc(
    depth: u16,
    mode: u16,
    channels: u16,
    width: u32,
    height: u32,
    icc: &[u8],
    planes: &[&[u8]],
) -> Vec<u8> {
    let resources = icc_resource(icc);
    let mut p = Vec::new();
    p.extend_from_slice(b"8BPS");
    p.extend_from_slice(&1u16.to_be_bytes());
    p.extend_from_slice(&[0u8; 6]);
    p.extend_from_slice(&channels.to_be_bytes());
    p.extend_from_slice(&height.to_be_bytes());
    p.extend_from_slice(&width.to_be_bytes());
    p.extend_from_slice(&depth.to_be_bytes());
    p.extend_from_slice(&mode.to_be_bytes());
    p.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    p.extend_from_slice(&(resources.len() as u32).to_be_bytes());
    p.extend_from_slice(&resources);
    p.extend_from_slice(&0u32.to_be_bytes()); // layer/mask section
    p.extend_from_slice(&crate::common::COMPRESSION_RAW.to_be_bytes());
    for plane in planes {
        p.extend_from_slice(plane);
    }
    p
}

/// Three 16-bit samples; only their length matters once the reader narrows.
const PLANE16: &[u8] = &[0x80, 0x00, 0x40, 0x00, 0x20, 0x00];

#[test]
fn read_sixteen_bit_cmyk_then_write_keeps_cmyk_and_its_profile() {
    let psd = flat_psd_with_icc(16, MODE_CMYK, 4, 3, 1, &profiled(b"CMYK"), &[PLANE16; 4]);

    let convert = crate::read_psd(&psd).expect("parses");
    let preserve = crate::read_psd_with(&psd, Policy::Preserve).expect("parses");
    for (policy, doc) in [("convert", convert), ("preserve", preserve)] {
        assert_eq!(doc.source_mode, Some(ColorMode::Cmyk), "{policy}");
        assert_eq!(doc.source_depth, Some(BitDepth::Sixteen), "{policy}");
        assert_eq!(doc.mode, ColorMode::Rgb, "{policy}");
        assert!(
            decode_section(doc.image_resources.to_vec())
                .iter()
                .any(|r| r.id == ICC_PROFILE),
            "the CMYK profile is retained on read ({policy})"
        );

        // The native samples are retained, so the save keeps CMYK at 16 bits
        // and the CMYK-space profile matches the output mode.
        let out = crate::write_psd(&doc).expect("writes");
        assert_eq!(header_mode(&out), MODE_CMYK, "{policy}");
        assert!(
            decode_section(written_resources(&out))
                .iter()
                .any(|r| r.id == ICC_PROFILE),
            "the matching CMYK profile is kept on write ({policy})"
        );
    }
}

#[test]
fn read_sixteen_bit_lab_then_write_keeps_lab_and_its_profile() {
    let psd = flat_psd_with_icc(16, MODE_LAB, 3, 3, 1, &profiled(b"Lab "), &[PLANE16; 3]);

    let convert = crate::read_psd(&psd).expect("parses");
    let preserve = crate::read_psd_with(&psd, Policy::Preserve).expect("parses");
    for (policy, doc) in [("convert", convert), ("preserve", preserve)] {
        assert_eq!(doc.source_mode, Some(ColorMode::Lab), "{policy}");
        assert_eq!(doc.source_depth, Some(BitDepth::Sixteen), "{policy}");
        assert_eq!(doc.mode, ColorMode::Rgb, "{policy}");
        assert!(
            decode_section(doc.image_resources.to_vec())
                .iter()
                .any(|r| r.id == ICC_PROFILE),
            "the Lab profile is retained on read ({policy})"
        );

        // The native samples are retained, so the save keeps Lab at 16 bits
        // and the Lab-space profile matches the output mode.
        let out = crate::write_psd(&doc).expect("writes");
        assert_eq!(header_mode(&out), MODE_LAB, "{policy}");
        assert!(
            decode_section(written_resources(&out))
                .iter()
                .any(|r| r.id == ICC_PROFILE),
            "the matching Lab profile is kept on write ({policy})"
        );
    }
}

#[test]
fn grayscale_output_keeps_a_matching_gray_profile() {
    let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
    doc.composite.data = vec![10, 200].into();
    let section = icc_resource(&profiled(b"GRAY"));
    doc.image_resources = section.clone().into();

    let out = crate::write_psd(&doc).expect("writes");
    assert_eq!(header_mode(&out), MODE_GRAYSCALE);
    assert_eq!(
        written_resources(&out),
        section,
        "the matching GRAY profile is re-emitted byte-for-byte"
    );
}

#[test]
fn parallel_display_transform_matches_serial() {
    use std::sync::atomic::Ordering;

    let profile = Profile::adobe_rgb().to_icc();
    let (w, h) = (200u32, 100u32);
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * 3];
    for (i, v) in data.iter_mut().enumerate() {
        *v = (i as u32).wrapping_mul(2_654_435_761) as u8;
    }
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = data.into();
    doc.document_icc = Some(profile.clone().into());

    super::FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = buffer_to_srgb(&doc, &doc.composite).into_owned();
    super::FORCE_SERIAL.store(false, Ordering::Relaxed);
    let before = super::PARALLEL_RUNS.load(Ordering::Relaxed);
    let parallel = buffer_to_srgb(&doc, &doc.composite).into_owned();
    assert!(
        super::PARALLEL_RUNS.load(Ordering::Relaxed) > before,
        "the parallel branch must run"
    );
    assert_eq!(
        serial.data, parallel.data,
        "3-channel display conversion differs between serial and parallel"
    );

    // A 4-channel buffer keeps its alpha plane byte-identical.
    let mut data4 = vec![0u8; n * 4];
    for (i, v) in data4.iter_mut().enumerate() {
        *v = (i as u32).wrapping_mul(40_503) as u8;
    }
    let mut doc4 = doc.clone();
    doc4.composite = PixelBuffer {
        width: w,
        height: h,
        channels: 4,
        data: data4.into(),
    };
    let alpha = doc4.composite.data[3 * n..].to_vec();

    super::FORCE_SERIAL.store(true, Ordering::Relaxed);
    let serial = buffer_to_srgb(&doc4, &doc4.composite).into_owned();
    super::FORCE_SERIAL.store(false, Ordering::Relaxed);
    let before = super::PARALLEL_RUNS.load(Ordering::Relaxed);
    let parallel = buffer_to_srgb(&doc4, &doc4.composite).into_owned();
    assert!(
        super::PARALLEL_RUNS.load(Ordering::Relaxed) > before,
        "the parallel branch must run"
    );
    assert_eq!(
        serial.data, parallel.data,
        "4-channel display conversion differs between serial and parallel"
    );
    assert_eq!(
        &parallel.data[3 * n..],
        alpha.as_slice(),
        "the alpha plane must be untouched"
    );
}

#[test]
#[ignore = "profiling: times the parallel display transform against the serial branch"]
fn display_transform_speedup_profile() {
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    let (w, h) = (4000u32, 3000u32);
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * 3];
    for (i, v) in data.iter_mut().enumerate() {
        *v = (i as u32).wrapping_mul(2_654_435_761) as u8;
    }
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.composite.data = data.into();
    doc.document_icc = Some(Profile::adobe_rgb().to_icc().into());

    super::FORCE_SERIAL.store(true, Ordering::Relaxed);
    let start = Instant::now();
    let serial = buffer_to_srgb(&doc, &doc.composite).into_owned();
    let serial_time = start.elapsed();
    super::FORCE_SERIAL.store(false, Ordering::Relaxed);

    let start = Instant::now();
    let parallel = buffer_to_srgb(&doc, &doc.composite).into_owned();
    let parallel_time = start.elapsed();
    assert_eq!(serial.data, parallel.data, "outputs must match");

    eprintln!(
        "pictura perf: display transform 4000x3000 serial={serial_time:?} parallel={parallel_time:?} speedup={:.2}x",
        serial_time.as_secs_f64() / parallel_time.as_secs_f64()
    );
}
