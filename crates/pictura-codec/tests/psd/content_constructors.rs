use pictura_codec::*;
use pictura_core::*;
use std::sync::Arc;

fn host(content: LayerContent) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let mut layer = Layer::new(content);
    layer.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };
    layer.set_channels(
        [0, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: vec![if id == -1 { 255 } else { 73 }; 4].into(),
            })
            .collect(),
    );
    let background = Layer::from_parts(
        pictura_core::LayerParts {
            rect: layer.rect,
            channels: layer.channels().clone(),
            ..Default::default()
        },
        LayerContent::Pixel,
    );
    doc.layers = LayerStack::from_layers(vec![background, layer]);
    doc
}

#[test]
fn unrepresentable_source_free_content_is_refused_explicitly() {
    let cases = [
        LayerContent::Adjustment(None),
        LayerContent::Text(None),
        LayerContent::Shape(Arc::new(FillContent::Solid([1, 2, 3, 255]))),
        LayerContent::SmartObject(Arc::new(SmartObject::default())),
        LayerContent::Adjustment(Some(Arc::new(Adjustment::Auto(AutoKind::Tone)))),
        LayerContent::Adjustment(Some(Arc::new(Adjustment::BrightnessContrast(
            BrightnessContrastParams {
                brightness: 1,
                contrast: 2,
                use_legacy: true,
            },
        )))),
        LayerContent::Adjustment(Some(Arc::new(Adjustment::Threshold(0)))),
        LayerContent::Adjustment(Some(Arc::new(Adjustment::HueSaturation(
            HueSaturationParams {
                hue: 0,
                saturation: 0,
                lightness: 0,
                ranges: vec![HueRange {
                    begin_ramp: 0,
                    begin_sustain: 10,
                    end_sustain: 20,
                    end_ramp: 30,
                    hue: 10,
                    saturation: 0,
                    lightness: 0,
                }],
            },
        )))),
    ];
    for content in cases {
        let doc = host(content);
        let before = doc.clone();
        for result in [write_psd(&doc), write_psb(&doc)] {
            assert!(matches!(result, Err(PsdError::Unsupported(_))));
        }
        assert_eq!(doc, before);
    }
    let mut text = author_type_tool(
        &TypeSpec::new("before", "Liberation Sans", 12.0),
        [0.0, -12.0, 40.0, 4.0],
        None,
        None,
    );
    text.text = "unsynchronized".into();
    let doc = host(LayerContent::Text(Some(Arc::new(text))));
    let before = doc.clone();
    for result in [write_psd(&doc), write_psb(&doc)] {
        assert!(matches!(result, Err(PsdError::Unsupported(_))));
    }
    assert_eq!(doc, before);
}
