use super::*;
use pictura_core::{BitDepth, ColorMode, Document, Layer, LayerStack, PsdRect, SmartObject};

fn smart_layer(payload: Vec<u8>) -> Layer {
    let mut so = SmartObject::default().with_payload(payload);
    so.kind = pictura_core::SmartObjectKind::Embedded;
    Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        smart_object: Some(so),
        ..Default::default()
    }
}

#[test]
fn a_channel_less_smart_object_with_a_decodable_payload_is_transformable() {
    let source = Document::from_rgba(
        "source",
        2,
        2,
        &[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    );
    let payload = pictura_codec::write_psd(&source).expect("encode source");
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![smart_layer(payload)]);
    assert!(can_free_transform(&doc, "0"));
}

#[test]
fn a_channel_less_smart_object_with_an_undecodable_payload_is_not_transformable() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![smart_layer(vec![0, 1, 2])]);
    assert!(!can_free_transform(&doc, "0"));
}

#[test]
fn a_channel_less_non_smart_layer_is_not_transformable() {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![Layer {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        },
        ..Default::default()
    }]);
    assert!(!can_free_transform(&doc, "0"));
}
