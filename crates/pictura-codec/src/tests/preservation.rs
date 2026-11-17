use super::*;

#[test]
fn opaque_document_sections_round_trip() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.composite
        .replace_data((0..12).collect())
        .expect("valid composite plane");
    doc.color_mode_data = vec![1, 2, 3].into();
    doc.image_resources = vec![4, 5].into();
    doc.global_layer_mask = vec![6, 7, 8, 9].into();
    doc.layer_section_extra = vec![10, 11].into();
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.color_mode_data, doc.color_mode_data);
    assert_eq!(back.image_resources, doc.image_resources);
    assert_eq!(back.global_layer_mask, doc.global_layer_mask);
    assert_eq!(back.layer_section_extra, doc.layer_section_extra);
    assert_eq!(back, doc);
}

#[test]
fn opaque_layer_blocks_round_trip() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Opaque", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.blend_key = Some(*b"zzzz");
    layer.blending_ranges = vec![1, 2, 3, 4].into();
    crate::edit_layer(layer.as_mut(), |parts| { parts.extra_blocks = vec![LayerBlock {
        key: *b"lfx2",
        data: vec![1, 2, 3, 4].into(),
    }] });
    crate::edit_layer(layer.as_mut(), |parts| { parts.raw_channels = vec![RawChannel {
        id: -3,
        data: vec![0, 0, 9, 9].into(),
    }] });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back, doc);
    assert_eq!(back.layers[0].blend_key, Some(*b"zzzz"));
    assert_eq!(back.layers[0].extra_blocks()[0].key, *b"lfx2");
    assert_eq!(back.layers[0].raw_channels()[0].data, vec![0, 0, 9, 9]);

    // A changed mode wins over the stale unknown key.
    let mut changed = doc.clone();
    changed.layers[0].blend = BlendMode::Multiply;
    let back = read_psd(&write_psd(&changed).unwrap()).unwrap();
    assert_eq!(back.layers[0].blend_key, None);
}

#[test]
fn recognized_blend_key_is_not_stored() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let known = pixel("Known", rect(0, 0, 2, 2), 3, BlendMode::Multiply, 255);
    doc.layers = pictura_core::LayerStack::from_layers(vec![known]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].blend_key, None);
    assert_eq!(back, doc);
}

#[test]
fn mask_extra_round_trips() {
    let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = pixel("Masked", rect(0, 0, 2, 2), 3, BlendMode::Normal, 255);
    layer.mask = Some(LayerMask {
        rect: rect(0, 0, 2, 2),
        data: Some(vec![9u8; 4].into()),
        extra: vec![7, 8, 9].into(),
        ..Default::default()
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
    assert_eq!(back.layers[0].mask.as_ref().unwrap().extra, vec![7, 8, 9]);
    assert_eq!(back, doc);
}

#[test]
fn truncated_preserved_blocks_error() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.global_layer_mask = vec![1, 2, 3, 4].into();
    let bytes = write_psd(&doc).unwrap();
    // Drop the composite and the last two global-mask bytes.
    let cut = bytes.len() - doc.composite.data.len() - 2 - 2;
    assert!(matches!(read_psd(&bytes[..cut]), Err(PsdError::Truncated)));
}

