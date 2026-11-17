use std::sync::Arc;

use crate::{
    Adjustment, Channel, FillContent, GradientFillParams, GradientKind, GradientStop, Layer,
    LayerContent, LayerKind, LayerStack, PatternFillParams, SmartObject, TypeTool,
};

fn text() -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "content".into(),
        bounds: [0, 0, 8, 8],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

#[test]
fn content_constructors_have_one_active_kind() {
    let fill = Arc::new(FillContent::Solid([12, 34, 56, 255]));
    let cases = [
        (LayerContent::Pixel, LayerKind::Pixel),
        (
            LayerContent::Adjustment(Some(Arc::new(Adjustment::Invert))),
            LayerKind::Adjustment,
        ),
        (LayerContent::Fill(fill.clone()), LayerKind::Fill),
        (LayerContent::Text(Some(Arc::new(text()))), LayerKind::Text),
        (LayerContent::Shape(fill), LayerKind::Shape),
        (
            LayerContent::SmartObject(Arc::new(SmartObject::default())),
            LayerKind::SmartObject,
        ),
        (
            LayerContent::Group(LayerStack::from_layers(vec![Layer::default()])),
            LayerKind::Group,
        ),
    ];
    for (content, expected) in cases {
        let layer = Layer::new(content);
        assert_eq!(layer.kind(), expected);
        assert_eq!(layer.is_group(), expected == LayerKind::Group);
        assert_eq!(layer.is_type(), expected == LayerKind::Text);
        assert_eq!(
            layer.smart_object().is_some(),
            expected == LayerKind::SmartObject
        );
        assert_eq!(
            layer.adjustment_parameters().is_some(),
            expected == LayerKind::Adjustment
        );
        assert_eq!(
            layer.fill_content().is_some(),
            matches!(expected, LayerKind::Fill | LayerKind::Shape)
        );
        assert_eq!(
            layer.children().len(),
            usize::from(expected == LayerKind::Group)
        );
    }
    let pixel = Layer::default();
    assert_eq!(pixel.kind(), LayerKind::Pixel);
    assert!(pixel.adjustment_parameters().is_none());
}

#[test]
fn content_types_have_no_execution_dependency() {
    let manifest = include_str!("../../Cargo.toml");
    for dependency in [
        "pictura-adjust",
        "pictura-codec",
        "pictura-render",
        "cxx-qt",
    ] {
        assert!(!manifest
            .lines()
            .any(|line| line.trim_start().starts_with(dependency)));
    }
    let layer = Layer::new(LayerContent::Adjustment(Some(Arc::new(Adjustment::Invert))));
    assert!(matches!(layer.content(), LayerContent::Adjustment(Some(_))));
}

#[test]
fn fill_content_retains_each_parameter_set() {
    let solid = FillContent::Solid([10, 20, 30, 40]);
    assert_eq!(solid, FillContent::Solid([10, 20, 30, 40]));
    for kind in [
        GradientKind::Linear,
        GradientKind::Radial,
        GradientKind::Angle,
        GradientKind::Reflected,
        GradientKind::Diamond,
    ] {
        let params = GradientFillParams {
            stops: vec![
                GradientStop {
                    location: 12,
                    color: [10, 20, 30],
                },
                GradientStop {
                    location: 4000,
                    color: [200, 210, 220],
                },
            ],
            reverse: true,
            kind,
            angle_deg: 45.0,
            scale: 60.0,
        };
        let layer = Layer::new(LayerContent::Fill(Arc::new(FillContent::Gradient(
            params.clone(),
        ))));
        assert_eq!(layer.fill_content(), Some(&FillContent::Gradient(params)));
    }
    let pattern = PatternFillParams {
        pattern_id: "pattern".into(),
        scale: 50.0,
        link_with_layer: false,
        origin: (3, -4),
    };
    let layer = Layer::new(LayerContent::Fill(Arc::new(FillContent::Pattern(
        pattern.clone(),
    ))));
    assert_eq!(layer.fill_content(), Some(&FillContent::Pattern(pattern)));
}

#[test]
fn incompatible_content_transitions_leave_the_layer_unchanged() {
    let mut layer = Layer::from_parts(
        LayerParts {
            name: "pixel".into(),
            opacity: 128,
            ..Default::default()
        },
        LayerContent::Pixel,
    );
    let before = layer.clone();
    assert!(!layer.set_fill_content(FillContent::Solid([1, 2, 3, 255])));
    assert_eq!(layer, before);
    let mut group = Layer::new(LayerContent::Group(LayerStack::default()));
    let before = group.clone();
    assert!(!group.set_fill_content(FillContent::Solid([1, 2, 3, 255])));
    assert_eq!(group, before);
}

#[test]
fn proxies_and_masks_preserve_content_identity() {
    let channels = vec![Channel {
        id: 0,
        data: vec![42u8; 4].into(),
    }];
    for content in [
        LayerContent::Pixel,
        LayerContent::Text(Some(Arc::new(text()))),
        LayerContent::SmartObject(Arc::new(SmartObject::default())),
        LayerContent::Fill(Arc::new(FillContent::Solid([1, 2, 3, 255]))),
    ] {
        let mut layer = Layer::new(content);
        let kind = layer.kind();
        layer.set_channels(channels.clone());
        if kind == LayerKind::Pixel {
            layer.vector_mask = Some(crate::VectorMask::default());
        }
        if kind == LayerKind::Fill {
            layer.mask = Some(crate::LayerMask::default());
        }
        let copy = layer.clone();
        assert_eq!(layer.kind(), kind);
        assert_eq!(copy.kind(), kind);
        assert_eq!(layer.channels()[0].data, channels[0].data);
        assert!(layer.channels()[0].data.shares(&copy.channels()[0].data));
    }
}

#[test]
fn metadata_cannot_install_an_active_content_definition() {
    let mut layer = Layer::default();
    assert!(layer.set_metadata_block(*b"zzzz", Some(vec![1, 2, 3].into())));
    let before = layer.clone();
    for key in [
        *b"TySh", *b"SoCo", *b"vscg", *b"vmsk", *b"SoLd", *b"PiSt", *b"lsct",
    ] {
        assert!(!layer.set_metadata_block(key, Some(vec![0; 8].into())));
        assert_eq!(layer, before);
    }
    assert!(layer.set_metadata_block(*b"zzzz", None));
    assert_eq!(layer.kind(), LayerKind::Pixel);
}

#[test]
fn content_generations_track_edits_without_changing_semantic_equality() {
    let content = LayerContent::Fill(Arc::new(FillContent::Solid([1, 2, 3, 255])));
    let mut layer = Layer::new(content.clone());
    let equal = Layer::new(content);
    assert_eq!(layer, equal);
    assert_ne!(layer.content_identity(), equal.content_identity());
    let original = layer.clone();
    assert_eq!(layer.content_identity(), original.content_identity());
    let mut previous = layer.content_identity();
    for value in [10, 20, 30, 40] {
        assert!(layer.set_fill_content(FillContent::Solid([value, 2, 3, 255])));
        assert_ne!(layer.content_identity(), previous);
        previous = layer.content_identity();
        assert_eq!(
            original.fill_content(),
            Some(&FillContent::Solid([1, 2, 3, 255]))
        );
    }
}
