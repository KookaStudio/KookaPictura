use pictura_codec::{
    decode_adjustment, decode_fill, decode_vector_fill, read_psd, write_psb, write_psd, DescValue,
};
use pictura_core::{
    Adjustment, AdjustmentData, BitDepth, Channel, ColorMode, Document, FillContent,
    GradientFillParams, GradientKind, GradientMapParams, GradientStop, Layer, LayerBlock,
    LayerKind, LayerParts, LayerStack, PatternFillParams, PsdRect,
};

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

fn block(key: &[u8; 4], bytes: Vec<u8>) -> AdjustmentData {
    AdjustmentData {
        key: *key,
        data: bytes.into(),
    }
}

fn descriptor(items: Vec<(Vec<u8>, DescValue)>) -> Vec<u8> {
    pictura_codec::write_descriptor(&DescValue::Object {
        name: String::new(),
        class_id: b"test".to_vec(),
        items,
    })
}

fn stops() -> Vec<GradientStop> {
    vec![
        GradientStop {
            location: 0,
            color: [0; 3],
        },
        GradientStop {
            location: 4096,
            color: [255; 3],
        },
    ]
}

#[test]
fn adjustment_candidates_preserve_raw_bytes() {
    for key in [
        *b"levl", *b"curv", *b"brit", *b"expA", *b"vibA", *b"hue2", *b"hue ", *b"blwh", *b"phfl",
        *b"blnc", *b"mixr", *b"grdm", *b"gdrm", *b"invr", *b"nvrt", *b"post", *b"thrs", *b"selc",
        *b"clrL", *b"SoCo", *b"GdFl", *b"PtFl",
    ] {
        let original = block(&key, vec![0, 0, 231, 19, 11]);
        let doc = document(layer(Some(original.clone()), Vec::new()));
        for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
            let back = read_psd(&bytes).unwrap();
            assert_eq!(back.layers[0].adjustment(), Some(&original), "{key:?}");
        }
    }
}

#[test]
fn adjustment_decoders_preserve_parameter_contract() {
    use pictura_core::{
        BlackWhiteParams, ChannelMixerParams, ColorBalanceParams, CurvesParams, ExposureParams,
        PhotoFilterParams, SelectiveColorParams, VibranceParams,
    };
    let mut exposure = 1u16.to_be_bytes().to_vec();
    for value in [1.0f32, 0.5, 1.25] {
        exposure.extend_from_slice(&value.to_be_bytes());
    }
    let vibrance = block(
        b"vibA",
        descriptor(vec![
            (b"vibrance".to_vec(), DescValue::Long(10)),
            (b"Strt".to_vec(), DescValue::Long(-5)),
        ]),
    );
    let black_white = block(b"blwh", descriptor(Vec::new()));
    let mut photo_xyz = 3u16.to_be_bytes().to_vec();
    for value in [0u32, 0, 0, 25] {
        photo_xyz.extend_from_slice(&value.to_be_bytes());
    }
    photo_xyz.push(1);
    let gradient = GradientMapParams {
        stops: stops(),
        reverse: true,
        transparency: Vec::new(),
    };
    let curves = CurvesParams {
        points: vec![(0, 0), (128, 170), (255, 255)],
        red: Some(vec![(0, 10), (255, 240)]),
        green: None,
        blue: None,
    };
    let mut selective = SelectiveColorParams::default();
    selective.ranges[0].c = 20;
    selective.ranges[8].k = -30;
    let cases = vec![
        (pictura_render::encode_invert(), Adjustment::Invert),
        (
            block(b"expA", exposure),
            Adjustment::Exposure(ExposureParams {
                exposure: 1.0,
                offset: 0.5,
                gamma: 1.25,
            }),
        ),
        (
            vibrance,
            Adjustment::Vibrance(VibranceParams {
                vibrance: 10,
                saturation: -5,
            }),
        ),
        (
            black_white,
            Adjustment::BlackWhite(BlackWhiteParams {
                red: 40.0,
                yellow: 60.0,
                green: 40.0,
                cyan: 60.0,
                blue: 20.0,
                magenta: 80.0,
                tint: false,
                tint_color: [0; 3],
            }),
        ),
        (
            pictura_render::encode_photo_filter([255, 180, 80], 25.0, true),
            Adjustment::PhotoFilter(PhotoFilterParams {
                color: [255, 180, 80],
                density: 25.0,
                preserve_luminosity: true,
            }),
        ),
        (
            block(b"phfl", photo_xyz),
            Adjustment::PhotoFilter(PhotoFilterParams {
                color: [0; 3],
                density: 25.0,
                preserve_luminosity: true,
            }),
        ),
        (
            pictura_render::encode_gradient_map(&gradient, false),
            Adjustment::GradientMap(gradient),
        ),
        (
            pictura_render::encode_color_balance(
                [1.0, -2.0, 3.0],
                [4.0, -5.0, 6.0],
                [7.0, -8.0, 9.0],
                true,
            ),
            Adjustment::ColorBalance(ColorBalanceParams {
                shadows: [1.0, -2.0, 3.0],
                midtones: [4.0, -5.0, 6.0],
                highlights: [7.0, -8.0, 9.0],
                preserve_luminosity: true,
            }),
        ),
        (
            pictura_render::encode_channel_mixer(
                false,
                [30.0, -10.0, 50.0],
                [10.0, 90.0, 0.0],
                [0.0, 20.0, 110.0],
                [5.0, -20.0, 40.0],
            ),
            Adjustment::ChannelMixer(ChannelMixerParams {
                red: [30.0, -10.0, 50.0],
                green: [10.0, 90.0, 0.0],
                blue: [0.0, 20.0, 110.0],
                constant: [5.0, -20.0, 40.0],
                ..Default::default()
            }),
        ),
        (
            pictura_render::encode_channel_mixer(
                true,
                [20.0, 40.0, 60.0],
                [0.0, 100.0, 0.0],
                [0.0, 0.0, 100.0],
                [-15.0, 0.0, 0.0],
            ),
            Adjustment::ChannelMixer(ChannelMixerParams {
                monochrome: true,
                red: [20.0, 40.0, 60.0],
                constant: [-15.0, 0.0, 0.0],
                ..Default::default()
            }),
        ),
        (
            pictura_render::encode_curves(&curves.points, curves.red.as_deref(), None, None),
            Adjustment::Curves(curves),
        ),
        (
            pictura_render::encode_curves(&[], Some(&[(0, 10), (255, 240)]), None, None),
            Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (255, 255)],
                red: Some(vec![(0, 10), (255, 240)]),
                green: None,
                blue: None,
            }),
        ),
        (
            pictura_render::encode_selective_color(selective.method, &selective.ranges),
            Adjustment::SelectiveColor(selective),
        ),
    ];
    for (encoded, expected) in cases {
        assert_eq!(
            decode_adjustment(&encoded),
            Some(expected.clone()),
            "{:?}",
            encoded.key
        );
        let mut trailing = encoded.clone();
        let mut bytes = trailing.data.to_vec();
        bytes.extend_from_slice(&[219, 17, 3]);
        trailing.data = bytes.into();
        assert_eq!(
            decode_adjustment(&trailing),
            Some(expected),
            "trailing {:?}",
            encoded.key
        );
        for length in [0, 1] {
            if &encoded.key != b"nvrt" {
                assert!(decode_adjustment(&block(
                    &encoded.key,
                    encoded.data[..length.min(encoded.data.len())].to_vec()
                ))
                .is_none());
            }
        }
    }
    for key in [
        b"expA", b"vibA", b"blwh", b"phfl", b"grdm", b"blnc", b"mixr", b"curv", b"selc",
    ] {
        assert!(
            decode_adjustment(&block(key, vec![255; 5])).is_none(),
            "{key:?}"
        );
    }
    assert!(decode_adjustment(&block(b"post", vec![0, 1])).is_none());
    assert!(decode_adjustment(&block(b"thrs", vec![0, 0])).is_none());
    let encoded = pictura_render::encode_gradient_map(
        &GradientMapParams {
            stops: stops(),
            reverse: false,
            transparency: Vec::new(),
        },
        false,
    );
    let expected = decode_adjustment(&encoded).unwrap();
    let mut version_three = encoded.data.to_vec();
    version_three[..2].copy_from_slice(&3u16.to_be_bytes());
    version_three.splice(4..4, *b"Lnr ");
    assert_eq!(
        decode_adjustment(&block(b"grdm", version_three)),
        Some(expected)
    );
    let encoded = pictura_render::encode_selective_color(
        pictura_core::SelectiveColorMethod::Relative,
        &[pictura_core::SelectiveRange::default(); 9],
    );
    let expected = decode_adjustment(&encoded).unwrap();
    let mut reserved = encoded.data.to_vec();
    reserved[4..12].fill(255);
    assert_eq!(decode_adjustment(&block(b"selc", reserved)), Some(expected));
}

#[test]
fn fill_decoders_preserve_parameter_contract() {
    assert_eq!(
        decode_fill(&block(b"SoCo", vec![10, 20, 30, 40])),
        Some(FillContent::Solid([10, 20, 30, 40]))
    );
    assert_eq!(
        decode_fill(&pictura_render::encode_solid_color_fill([10, 20, 30])),
        Some(FillContent::Solid([10, 20, 30, 255]))
    );
    for kind in [
        GradientKind::Linear,
        GradientKind::Radial,
        GradientKind::Angle,
        GradientKind::Reflected,
        GradientKind::Diamond,
    ] {
        let params = GradientFillParams {
            stops: stops(),
            reverse: true,
            kind,
            angle_deg: 45.0,
            scale: 60.0,
        };
        let encoded = pictura_render::encode_gradient_fill_params(&params);
        assert_eq!(decode_fill(&encoded), Some(FillContent::Gradient(params)));
        assert!(decode_adjustment(&encoded).is_none());
        let defaults = pictura_render::encode_gradient_fill(kind, &stops(), 0.0);
        assert!(
            matches!(decode_fill(&defaults), Some(FillContent::Gradient(p)) if !p.reverse && p.scale == 100.0 && p.angle_deg == 0.0)
        );
    }
    let params = PatternFillParams {
        pattern_id: "p1".into(),
        scale: 50.0,
        link_with_layer: false,
        origin: (3, 4),
    };
    assert_eq!(
        decode_fill(&pictura_render::encode_pattern_fill(&params)),
        Some(FillContent::Pattern(params))
    );
    for key in [b"SoCo", b"GdFl", b"PtFl"] {
        for bytes in [vec![], vec![255; 3], descriptor(Vec::new())] {
            assert!(decode_fill(&block(key, bytes)).is_none());
        }
    }
    for params in [
        GradientFillParams {
            stops: vec![],
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        },
        GradientFillParams {
            stops: vec![GradientStop {
                location: 0,
                color: [0; 3],
            }],
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        },
        GradientFillParams {
            stops: vec![
                GradientStop {
                    location: 20,
                    color: [0; 3],
                },
                GradientStop {
                    location: 10,
                    color: [255; 3],
                },
            ],
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        },
        GradientFillParams {
            stops: stops(),
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: f32::NAN,
            scale: 100.0,
        },
    ] {
        assert!(decode_fill(&pictura_render::encode_gradient_fill_params(&params)).is_none());
    }
    let bad_pattern = PatternFillParams {
        pattern_id: "p1".into(),
        scale: f32::INFINITY,
        link_with_layer: true,
        origin: (0, 0),
    };
    assert!(decode_fill(&pictura_render::encode_pattern_fill(&bad_pattern)).is_none());
}

#[test]
fn vector_fill_decoders_preserve_contract() {
    for encoded in [
        pictura_render::encode_solid_color_fill([10, 20, 30]),
        pictura_render::encode_gradient_fill(GradientKind::Radial, &stops(), 45.0),
    ] {
        let mut vector = b"zzzz".to_vec();
        vector.extend_from_slice(&encoded.data);
        assert_eq!(decode_vector_fill(&vector), decode_fill(&encoded));
        for bytes in [
            vec![],
            vec![0; 3],
            [b"SoCo".as_slice(), &15u32.to_be_bytes()].concat(),
        ] {
            assert!(decode_vector_fill(&bytes).is_none());
        }
    }
    let encoded = pictura_render::encode_solid_color_fill([10, 20, 30]);
    let mut vector = b"SoCo".to_vec();
    vector.extend_from_slice(&encoded.data);
    let primary = block(b"brit", vec![255]);
    let blocked = layer(
        Some(primary.clone()),
        vec![LayerBlock {
            key: *b"vscg",
            data: vector.into(),
        }],
    );
    assert_eq!(blocked.kind(), LayerKind::Adjustment);
    assert_eq!(blocked.adjustment(), Some(&primary));
    assert!(blocked.fill_content().is_none());
}

#[test]
fn conflicting_import_candidates_follow_one_precedence() {
    let fill = pictura_render::encode_solid_color_fill([10, 20, 30]);
    let mut vector = b"SoCo".to_vec();
    vector.extend_from_slice(&fill.data);
    let extras = vec![
        LayerBlock {
            key: *b"vscg",
            data: vector.into(),
        },
        LayerBlock {
            key: *b"TySh",
            data: vec![1, 2, 3].into(),
        },
    ];
    let mut parts = LayerParts {
        rect: rect(),
        channels: pixels(),
        adjustment: Some(fill),
        extra_blocks: extras.clone(),
        is_group: true,
        children: LayerStack::from_layers(vec![Layer::default()]),
        ..Default::default()
    };
    let group = pictura_codec::layer_from_parts(parts.clone());
    assert_eq!(group.kind(), LayerKind::Group);
    parts.is_group = false;
    parts.children = LayerStack::default();
    assert_eq!(
        pictura_codec::layer_from_parts(parts.clone()).kind(),
        LayerKind::Fill
    );
    parts.adjustment = Some(block(b"brit", vec![255]));
    assert_eq!(
        pictura_codec::layer_from_parts(parts.clone()).kind(),
        LayerKind::Adjustment
    );
    parts.adjustment = None;
    let vector = pictura_codec::layer_from_parts(parts.clone());
    assert_eq!(vector.kind(), LayerKind::Fill);
    assert_eq!(vector.extra_blocks().as_slice(), extras.as_slice());
    parts.smart_object = Some(pictura_core::SmartObject::default());
    assert_eq!(
        pictura_codec::layer_from_parts(parts.clone()).kind(),
        LayerKind::Fill
    );
    parts.extra_blocks.remove(0);
    assert_eq!(
        pictura_codec::layer_from_parts(parts.clone()).kind(),
        LayerKind::SmartObject
    );
    parts.smart_object = None;
    assert_eq!(
        pictura_codec::layer_from_parts(parts.clone()).kind(),
        LayerKind::Text
    );
    parts.extra_blocks.clear();
    assert_eq!(
        pictura_codec::layer_from_parts(parts).kind(),
        LayerKind::Pixel
    );
}

#[test]
fn conflicting_content_preserves_psd_and_psb_data() {
    let original = block(b"brit", vec![0, 0, 255]);
    let extra = vec![
        LayerBlock {
            key: *b"TySh",
            data: vec![2, 3, 4].into(),
        },
        LayerBlock {
            key: *b"zzzz",
            data: vec![111, 222, 33].into(),
        },
    ];
    let mut parts = LayerParts {
        rect: rect(),
        channels: pixels(),
        adjustment: Some(original.clone()),
        extra_blocks: extra.clone(),
        ..Default::default()
    };
    parts.raw_channels.push(pictura_core::RawChannel {
        id: -3,
        data: vec![0, 0, 10, 20, 30, 40].into(),
    });
    let source = pictura_codec::layer_from_parts(parts);
    let raw = source.raw_channels().to_vec();
    let doc = document(source);
    for bytes in [write_psd(&doc).unwrap(), write_psb(&doc).unwrap()] {
        let back = read_psd(&bytes).unwrap();
        assert_eq!(back.layers[0].kind(), LayerKind::Adjustment);
        assert_eq!(back.layers[0].adjustment(), Some(&original));
        for block in &extra {
            assert_eq!(back.layers[0].extra_block(&block.key), Some(block));
        }
        assert_eq!(back.layers[0].raw_channels().as_slice(), raw.as_slice());
        assert_eq!(back.layers[0].channels(), doc.layers[0].channels());
    }
}

#[test]
fn content_writer_preserves_serialized_baselines() {
    let golden = include_bytes!("fixtures/default_before.psd");
    let doc = read_psd(golden).unwrap();
    assert_eq!(write_psd(&doc).unwrap(), golden.as_slice());
    for bytes in [
        include_bytes!("fixtures/solid_fill.psd").as_slice(),
        include_bytes!("fixtures/gradient_fill.psd").as_slice(),
        include_bytes!("fixtures/pattern_fill.psd").as_slice(),
    ] {
        let doc = read_psd(bytes).unwrap();
        let back = read_psd(&write_psd(&doc).unwrap()).unwrap();
        for (a, b) in doc.layers.iter().zip(back.layers.iter()) {
            assert_eq!(a.kind(), b.kind());
            assert_eq!(a.adjustment(), b.adjustment());
            assert_eq!(a.extra_blocks(), b.extra_blocks());
            assert_eq!(a.channels(), b.channels());
        }
    }
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
        assert_eq!(
            pictura_render::composite_rgba(&back),
            pictura_render::composite_rgba(&doc)
        );
    }
}
