use std::sync::Arc;

use pictura_codec::*;
use pictura_core::*;
use pictura_render::composite_rgba;

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
        let rendered = composite_rgba(&doc);
        for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
            let back = read_psd(&bytes).unwrap();
            assert_eq!(
                back.layers[1].kind(),
                LayerKind::Adjustment,
                "{:?}",
                block.key
            );
            assert_eq!(
                back.layers[1].adjustment_parameters(),
                Some(&expected),
                "{:?}",
                block.key
            );
            assert_eq!(composite_rgba(&back), rendered);
            let saved = if back.is_psb {
                write_psb(&back)
            } else {
                write_psd(&back)
            }
            .unwrap();
            let reopened = read_psd(&saved).unwrap();
            assert_eq!(reopened.layers[1].kind(), LayerKind::Adjustment);
            assert_eq!(reopened.layers[1].adjustment_parameters(), Some(&expected));
            assert_eq!(composite_rgba(&reopened), rendered);
        }
    }
}

#[test]
fn source_free_text_constructor_preserves_its_model() {
    let spec = TypeSpec::new("Hello", "Liberation Sans", 12.0);
    let expected = author_type_tool(&spec, [0.0, -12.0, 40.0, 4.0], None, None);
    let mut encoded = encode_type_tool(&expected);
    encoded.resize(encoded.len() + (encoded.len() & 1), 0);
    let doc = host(LayerContent::Text(Some(Arc::new(expected.clone()))));
    assert!(doc.layers[1].extra_block(b"TySh").is_none());
    let rendered = composite_rgba(&doc);
    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let back = read_psd(&bytes).unwrap();
        assert_eq!(back.layers[1].kind(), LayerKind::Text);
        assert_eq!(back.layers[1].type_tool(), Some(&expected));
        assert_eq!(
            back.layers[1].extra_block(b"TySh").unwrap().data.as_slice(),
            encoded
        );
        assert_eq!(composite_rgba(&back), rendered);
        let saved = if back.is_psb {
            write_psb(&back)
        } else {
            write_psd(&back)
        }
        .unwrap();
        let reopened = read_psd(&saved).unwrap();
        assert_eq!(reopened.layers[1].kind(), LayerKind::Text);
        assert_eq!(reopened.layers[1].type_tool(), Some(&expected));
        assert_eq!(composite_rgba(&reopened), rendered);
    }
}

fn rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    }
}

fn pixels() -> Vec<Channel> {
    [0, 1, 2, -1]
        .into_iter()
        .map(|id| Channel {
            id,
            data: vec![if id == -1 { 255 } else { 90 }; 4].into(),
        })
        .collect()
}

fn layer(adjustment: Option<AdjustmentData>, extra_blocks: Vec<LayerBlock>) -> Layer {
    pictura_codec::layer_from_parts(LayerParts {
        rect: rect(),
        channels: pixels(),
        adjustment,
        extra_blocks,
        ..Default::default()
    })
}

fn document(layer: Layer) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![layer]);
    doc
}

#[test]
fn opaque_text_preserves_identity_and_bytes() {
    let original = LayerBlock {
        key: *b"TySh",
        data: vec![1, 2, 3, 4].into(),
    };
    let doc = document(layer(None, vec![original.clone()]));
    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let back = read_psd(&bytes).unwrap();
        let text = &back.layers[0];
        assert_eq!(text.kind(), LayerKind::Text);
        assert!(text.type_tool().is_none());
        assert_eq!(text.extra_block(b"TySh"), Some(&original));
        assert_eq!(text.channels(), doc.layers[0].channels());
        assert_eq!(composite_rgba(&back), composite_rgba(&doc));
    }
}

fn cube(size: usize, count: usize) -> Vec<u8> {
    let mut bytes = format!("LUT_3D_SIZE {size}\n").into_bytes();
    for _ in 0..count {
        bytes.extend_from_slice(b"0.25 0.5 0.75\n");
    }
    bytes
}

fn image_budget(per_allocation: u64, total: u64) -> ImageBudget {
    ImageBudget {
        max_dimension: 16,
        max_alloc_bytes: per_allocation,
        max_document_bytes: total,
        max_tiled_bytes: total,
    }
}

fn lookup_document(payload: &[u8], count: usize) -> pictura_core::Document {
    use pictura_core::{BitDepth, ColorMode, Document, LayerParts, LayerStack, PsdRect};

    let mut document = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    document.layers = LayerStack::from_layers(
        (0..count)
            .map(|index| {
                pictura_codec::layer_from_parts(LayerParts {
                    name: format!("Hidden Lookup {index}"),
                    rect: PsdRect {
                        top: 0,
                        left: 0,
                        bottom: 1,
                        right: 1,
                    },
                    visible: false,
                    adjustment: Some(pictura_core::AdjustmentData {
                        key: *b"clrL",
                        data: payload.to_vec().into(),
                    }),
                    ..Default::default()
                })
            })
            .collect(),
    );
    document
}

#[test]
fn admitted_content_preserves_parameters_and_bytes() {
    use pictura_core::{AdjustmentData, FillContent, LayerKind, LayerParts, PsdRect};

    let mut cube_bytes = cube(2, 27);
    cube_bytes.extend_from_slice(b"LUT_3D_SIZE 3\n");
    let mut raw = pictura_codec::encode_color_lookup(&cube_bytes, "Later Header");
    let mut data = raw.data.to_vec();
    data.resize(data.len() + (data.len() & 1), 0);
    raw.data = data.into();
    let mut document = lookup_document(&raw.data, 1);
    document
        .layers
        .push(pictura_codec::layer_from_parts(LayerParts {
            name: "Fill".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 1,
            },
            adjustment: Some(AdjustmentData {
                key: *b"SoCo",
                data: vec![10, 20, 30, 255].into(),
            }),
            ..Default::default()
        }))
        .expect("valid fill layer");
    let bytes = pictura_codec::write_psd(&document).unwrap();
    let reopened = pictura_codec::read_psd_within_options(
        &bytes,
        &pictura_codec::OpenOptions::default(),
        image_budget(1 << 20, 1 << 20),
    )
    .expect("all derived content fits the explicit budget");
    let Some(pictura_core::Adjustment::ColorLookup(parameters)) =
        reopened.layers[0].adjustment_parameters()
    else {
        panic!("retained typed lookup");
    };
    let lookup = parameters.lookup.as_ref().unwrap();
    assert_eq!(lookup.size, 3);
    assert_eq!(lookup.points, vec![[0.25, 0.5, 0.75]; 27]);
    assert_eq!(reopened.layers[1].kind(), LayerKind::Fill);
    assert_eq!(
        reopened.layers[1].fill_content(),
        Some(&FillContent::Solid([10, 20, 30, 255]))
    );
    assert_eq!(
        composite_rgba(&reopened).data.as_slice(),
        &[10, 20, 30, 255]
    );
    for (original, decoded) in document.layers.iter().zip(reopened.layers.iter()) {
        assert_eq!(decoded.adjustment(), original.adjustment());
    }
    assert_eq!(pictura_codec::write_psd(&reopened).unwrap(), bytes);
}
