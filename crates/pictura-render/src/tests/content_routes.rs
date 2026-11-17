use super::*;
use crate::composite_route::{content_route, ContentRoute};
use pictura_core::{LayerKind, LayerParts, SmartObject, SmartObjectKind};

fn expected(rgba: [u8; 4]) -> PixelBuffer {
    let mut data = Vec::new();
    for value in rgba {
        data.extend_from_slice(&[value; 4]);
    }
    PixelBuffer {
        width: 2,
        height: 2,
        channels: 4,
        data: data.into(),
    }
}

fn pixel() -> Layer {
    solid(
        "pixel",
        full(2, 2),
        (100, 50, 25),
        255,
        BlendMode::Normal,
        255,
    )
}

fn raw(primary: Option<AdjustmentData>, extra_blocks: Vec<pictura_core::LayerBlock>) -> Layer {
    pictura_codec::layer_from_parts(LayerParts {
        rect: full(2, 2),
        adjustment: primary,
        extra_blocks,
        ..Default::default()
    })
}

fn vector_fill(color: [u8; 3]) -> pictura_core::LayerBlock {
    let primary = encode_solid_color_fill(color);
    let mut bytes = b"SoCo".to_vec();
    bytes.extend_from_slice(&primary.data);
    pictura_core::LayerBlock {
        key: *b"vscg",
        data: bytes.into(),
    }
}

#[test]
fn pixel_content_routes_preserve_output() {
    let layer = pixel();
    assert_eq!(layer.kind(), LayerKind::Pixel);
    assert!(matches!(content_route(&layer), ContentRoute::Pixels));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![layer.clone()])),
        expected([100, 50, 25, 255])
    );
    let mut hidden = layer.clone();
    hidden.visible = false;
    assert_eq!(composite_rgba(&doc(2, 2, vec![hidden])), expected([0; 4]));
    let mut masked = layer.clone();
    masked.mask = Some(LayerMask {
        rect: full(2, 2),
        data: Some(vec![0; 4].into()),
        ..Default::default()
    });
    assert_eq!(composite_rgba(&doc(2, 2, vec![masked])), expected([0; 4]));
    let opaque_vector = pictura_codec::layer_from_parts(LayerParts {
        extra_blocks: vec![pictura_core::LayerBlock {
            key: *b"vscg",
            data: vec![255; 3].into(),
        }],
        ..pictura_core::LayerParts::from(layer)
    });
    assert!(matches!(
        content_route(&opaque_vector),
        ContentRoute::UndecodedVectorFill
    ));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![opaque_vector])),
        expected([100, 50, 25, 255])
    );
}

#[test]
fn adjustment_content_routes_preserve_output() {
    let mut invert = raw(Some(encode_invert()), Vec::new());
    assert_eq!(invert.kind(), LayerKind::Adjustment);
    assert!(matches!(
        content_route(&invert),
        ContentRoute::Adjustment(_)
    ));
    invert.fill = 0;
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![pixel(), invert.clone()])),
        expected([155, 205, 230, 255])
    );
    invert.opacity = 0;
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![pixel(), invert.clone()])),
        expected([100, 50, 25, 255])
    );
    invert.opacity = 255;
    invert.mask = Some(LayerMask {
        rect: full(2, 2),
        data: Some(vec![0; 4].into()),
        ..Default::default()
    });
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![pixel(), invert])),
        expected([100, 50, 25, 255])
    );
}

#[test]
fn group_content_routes_preserve_output() {
    for blend in [BlendMode::PassThrough, BlendMode::Normal] {
        for opacity in [255, 128] {
            let layer = group("group", blend, opacity, None, vec![pixel()]);
            assert_eq!(layer.kind(), LayerKind::Group);
            assert_eq!(
                matches!(content_route(&layer), ContentRoute::PassThroughGroup),
                blend == BlendMode::PassThrough && opacity == 255
            );
            assert_eq!(
                composite_rgba(&doc(2, 2, vec![layer])),
                expected([100, 50, 25, opacity])
            );
        }
    }
    let mask = LayerMask {
        rect: full(2, 2),
        data: Some(vec![0; 4].into()),
        ..Default::default()
    };
    let layer = group(
        "group",
        BlendMode::PassThrough,
        255,
        Some(mask),
        vec![pixel()],
    );
    assert!(matches!(content_route(&layer), ContentRoute::IsolatedGroup));
    assert_eq!(composite_rgba(&doc(2, 2, vec![layer])), expected([0; 4]));
    let mut layer = group("group", BlendMode::PassThrough, 255, None, vec![pixel()]);
    layer.vector_mask = Some(pictura_core::VectorMask::default());
    assert!(matches!(content_route(&layer), ContentRoute::IsolatedGroup));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![layer])),
        expected([100, 50, 25, 255])
    );
}

#[test]
fn fill_content_routes_preserve_output() {
    use pictura_core::{GradientFillParams, GradientKind, GradientStop, PatternFillParams};
    let gradient = GradientFillParams {
        stops: vec![
            GradientStop {
                location: 0,
                color: [10, 20, 30],
            },
            GradientStop {
                location: 4096,
                color: [10, 20, 30],
            },
        ],
        reverse: false,
        kind: GradientKind::Linear,
        angle_deg: 0.0,
        scale: 100.0,
    };
    let pattern = PatternFillParams {
        pattern_id: "missing".into(),
        scale: 100.0,
        link_with_layer: true,
        origin: (0, 0),
    };
    for (primary, rgba) in [
        (encode_solid_color_fill([10, 20, 30]), [10, 20, 30, 255]),
        (encode_gradient_fill_params(&gradient), [10, 20, 30, 255]),
        (encode_pattern_fill(&pattern), [128, 128, 128, 255]),
    ] {
        let layer = raw(Some(primary), Vec::new());
        assert_eq!(layer.kind(), LayerKind::Fill);
        assert!(matches!(content_route(&layer), ContentRoute::Fill(_)));
        assert_eq!(composite_rgba(&doc(2, 2, vec![layer])), expected(rgba));
    }
}

#[test]
fn shape_content_routes_preserve_output() {
    for fixture in ["vector_fill.psd", "vector_mask_vsms.psd"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../pictura-codec/tests/fixtures")
            .join(fixture);
        let bytes = std::fs::read(path).unwrap();
        let source = pictura_codec::read_psd(&bytes).unwrap();
        let layer = source
            .layers
            .iter()
            .find(|layer| layer.name == "Shape")
            .unwrap();
        assert_eq!(layer.kind(), LayerKind::Shape);
        assert!(matches!(content_route(layer), ContentRoute::Fill(_)));
        let before = composite_rgba(&source);
        let back = pictura_codec::read_psd(&pictura_codec::write_psd(&source).unwrap()).unwrap();
        assert_eq!(composite_rgba(&back), before);
        assert_eq!(
            back.layers
                .iter()
                .find(|layer| layer.name == "Shape")
                .unwrap()
                .pixel_data_irrelevant,
            layer.pixel_data_irrelevant
        );
    }
}

#[test]
fn text_content_routes_preserve_output() {
    use crate::composite_route::tests::{proxy, type_layer};
    let proxied = type_layer(proxy());
    assert_eq!(proxied.kind(), LayerKind::Text);
    assert!(matches!(content_route(&proxied), ContentRoute::Pixels));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![proxied])),
        expected([90, 90, 90, 90])
    );
    let source = type_layer(Vec::new());
    assert_eq!(source.kind(), LayerKind::Text);
    assert!(matches!(content_route(&source), ContentRoute::TypeSource));
    let mut blank =
        crate::tests::composite::type_layer(Some(crate::tests::composite::type_style()));
    let mut parts = LayerParts::from(blank.clone());
    parts.type_tool.as_mut().unwrap().text = "   ".into();
    blank = pictura_codec::layer_from_parts(parts);
    let backdrop = solid(
        "base",
        full(200, 80),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    );
    assert_eq!(
        composite_rgba(&doc(200, 80, vec![backdrop.clone(), blank])),
        composite_rgba(&doc(200, 80, vec![backdrop]))
    );
}

#[test]
fn smart_object_content_routes_preserve_output() {
    use crate::composite_route::tests::{filter, proxy, smart};
    for master in [true, false] {
        let layer = smart(
            proxy(),
            vec![filter(pictura_codec::CAMERA_RAW_FILTER_ID, true)],
            master,
        );
        assert_eq!(layer.kind(), LayerKind::SmartObject);
        assert!(matches!(content_route(&layer), ContentRoute::SmartFiltered));
    }
    for filters in [
        vec![],
        vec![filter(1, true)],
        vec![filter(pictura_codec::CAMERA_RAW_FILTER_ID, false)],
    ] {
        let layer = smart(proxy(), filters, true);
        assert!(matches!(content_route(&layer), ContentRoute::Pixels));
        assert_eq!(
            composite_rgba(&doc(2, 2, vec![layer])),
            expected([90, 90, 90, 90])
        );
    }
    let source = doc(2, 2, vec![pixel()]);
    let mut object =
        SmartObject::default().with_payload(pictura_codec::write_psd(&source).unwrap());
    object.kind = SmartObjectKind::Embedded;
    let layer = pictura_codec::layer_from_parts(LayerParts {
        rect: full(2, 2),
        smart_object: Some(object),
        ..Default::default()
    });
    assert!(matches!(content_route(&layer), ContentRoute::SmartSource));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![layer])),
        expected([100, 50, 25, 255])
    );
}

#[test]
fn proxy_routes_preserve_semantic_identity() {
    use crate::composite_route::tests::{proxy, smart, type_layer};
    for (a, b, kind) in [
        (type_layer(proxy()), type_layer(Vec::new()), LayerKind::Text),
        (
            smart(proxy(), vec![], true),
            smart(vec![], vec![], true),
            LayerKind::SmartObject,
        ),
    ] {
        assert_eq!(a.kind(), kind);
        assert_eq!(b.kind(), kind);
        assert!(matches!(content_route(&a), ContentRoute::Pixels));
        assert!(matches!(
            content_route(&b),
            ContentRoute::TypeSource | ContentRoute::SmartSource
        ));
    }
    let mut proxied = smart(proxy(), vec![], true);
    pictura_codec::edit_layer(&mut proxied, |parts| {
        parts.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"vscg",
            data: vec![255; 3].into(),
        });
    });
    assert_eq!(proxied.kind(), LayerKind::SmartObject);
    assert!(matches!(
        content_route(&proxied),
        ContentRoute::UndecodedVectorFill
    ));
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![proxied.clone()])),
        expected([90, 90, 90, 90])
    );
    proxied.channels_mut().clear();
    assert!(matches!(content_route(&proxied), ContentRoute::SmartSource));
    proxied.set_channels(proxy());
    pictura_codec::edit_layer(&mut proxied, |parts| {
        parts.smart_object.as_mut().unwrap().smart_filters =
            vec![crate::composite_route::tests::filter(
                pictura_codec::CAMERA_RAW_FILTER_ID,
                true,
            )];
    });
    assert!(matches!(
        content_route(&proxied),
        ContentRoute::SmartFiltered
    ));
}

#[test]
fn undecodable_primary_content_blocks_fallthrough() {
    let primary = AdjustmentData {
        key: *b"brit",
        data: vec![255].into(),
    };
    let layer = pictura_codec::layer_from_parts(LayerParts {
        adjustment: Some(primary.clone()),
        extra_blocks: vec![vector_fill([255, 0, 0])],
        ..LayerParts::from(pixel())
    });
    assert!(matches!(
        content_route(&layer),
        ContentRoute::UndecodedAdjustment
    ));
    assert_eq!(layer.adjustment(), Some(&primary));
    assert!(layer.extra_block(b"vscg").is_some());
    assert_eq!(
        composite_rgba(&doc(2, 2, vec![pixel(), layer])),
        expected([100, 50, 25, 255])
    );
}

#[test]
fn decoded_vector_fill_precedes_smart_and_type_sources() {
    use crate::composite_route::tests::{proxy, smart, type_layer};
    for source in [smart(proxy(), vec![], true), type_layer(vec![])] {
        let layer = pictura_codec::layer_from_parts(LayerParts {
            extra_blocks: vec![vector_fill([10, 20, 30])],
            ..LayerParts::from(source)
        });
        assert!(matches!(content_route(&layer), ContentRoute::Fill(_)));
        assert_eq!(
            composite_rgba(&doc(2, 2, vec![layer])),
            expected([10, 20, 30, 255])
        );
    }
}
