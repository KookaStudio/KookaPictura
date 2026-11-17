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
    let mut background = Layer::default();
    background.rect = layer.rect;
    background.set_channels(layer.channels().clone());
    doc.layers = LayerStack::from_layers(vec![background, layer]);
    doc
}

#[test]
fn source_free_adjustment_constructors_preserve_supported_parameters() {
    let blocks = [
        encode_invert(),
        encode_posterize(4),
        encode_threshold(128),
        encode_brightness_contrast(10, -20),
        encode_hue_saturation(15, -20, 25),
        encode_photo_filter([255, 180, 80], 25.0, true),
        encode_color_balance([1.0, 2.0, 3.0], [-2.0, 4.0, 1.0], [0.0; 3], true),
        encode_channel_mixer(
            false,
            [90.0, 10.0, 0.0],
            [0.0, 100.0, 0.0],
            [0.0, 0.0, 100.0],
            [0.0; 3],
        ),
        encode_channel_mixer_ink(ChannelMixerParams::default().ink, [0.0; 4]),
        encode_curves(
            &[(0, 0), (128, 150), (255, 255)],
            Some(&[(0, 3), (255, 240)]),
            None,
            None,
        ),
        encode_selective_color(
            SelectiveColorMethod::Absolute,
            &[SelectiveRange::default(); 9],
        ),
        encode_gradient_map(
            &GradientMapParams {
                stops: vec![
                    GradientStop {
                        location: 0,
                        color: [1, 2, 3],
                    },
                    GradientStop {
                        location: 4096,
                        color: [240, 241, 242],
                    },
                ],
                reverse: true,
                transparency: Vec::new(),
            },
            false,
        ),
        encode_shadows_highlights(25, 40),
    ];
    for block in blocks {
        let expected = decode_adjustment(&block).unwrap();
        let doc = host(LayerContent::Adjustment(Some(Arc::new(expected.clone()))));
        assert!(doc.layers[1].adjustment().is_none());
        let rendered = pictura_render::composite_rgba(&doc);
        for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
            let back = read_psd(&bytes).unwrap();
            assert_eq!(back.layers[1].kind(), LayerKind::Adjustment);
            assert_eq!(
                back.layers[1].adjustment_parameters(),
                Some(&expected),
                "{:?}",
                block.key
            );
            assert_eq!(pictura_render::composite_rgba(&back), rendered);
            let saved = if back.is_psb {
                write_psb(&back)
            } else {
                write_psd(&back)
            }
            .unwrap();
            let reopened = read_psd(&saved).unwrap();
            assert_eq!(reopened.layers[1].kind(), LayerKind::Adjustment);
            assert_eq!(reopened.layers[1].adjustment_parameters(), Some(&expected));
            assert_eq!(pictura_render::composite_rgba(&reopened), rendered);
        }
    }
}

#[test]
fn source_free_text_constructor_preserves_its_model() {
    let spec = TypeSpec::new("Hello", "Liberation Sans", 12.0);
    let expected = author_type_tool(&spec, [0.0, -12.0, 40.0, 4.0], None, None);
    let doc = host(LayerContent::Text(Some(Arc::new(expected.clone()))));
    assert!(doc.layers[1].extra_block(b"TySh").is_none());
    let rendered = pictura_render::composite_rgba(&doc);
    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let back = read_psd(&bytes).unwrap();
        assert_eq!(back.layers[1].kind(), LayerKind::Text);
        assert_eq!(back.layers[1].type_tool(), Some(&expected));
        assert_eq!(
            back.layers[1].extra_block(b"TySh").unwrap().data.as_slice(),
            encode_type_tool(&expected)
        );
        assert_eq!(pictura_render::composite_rgba(&back), rendered);
        let saved = if back.is_psb {
            write_psb(&back)
        } else {
            write_psd(&back)
        }
        .unwrap();
        let reopened = read_psd(&saved).unwrap();
        assert_eq!(reopened.layers[1].kind(), LayerKind::Text);
        assert_eq!(reopened.layers[1].type_tool(), Some(&expected));
        assert_eq!(pictura_render::composite_rgba(&reopened), rendered);
    }
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
