use super::*;
use pictura_adjust::{GradientKind, GradientStop, PatternFillParams};
use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
use pictura_core::{AdjustmentData, BitDepth, ColorMode, LayerBlock};

const SIZE: u32 = 8;

fn vscg_solid(rgb: [u8; 3]) -> Vec<u8> {
    let fill = crate::encode_solid_color_fill(rgb);
    let mut block = fill.key.to_vec();
    block.extend_from_slice(&fill.data);
    block
}

fn shape_layer(rgb: [u8; 3]) -> Layer {
    let outline = outline_in_box(
        ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
        (1.0, 1.0, 5.0, 5.0),
    )
    .unwrap();
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: pictura_core::PsdRect {
            left: 0,
            top: 0,
            right: SIZE as i32,
            bottom: SIZE as i32,
        },
        extra_blocks: vec![
            LayerBlock {
                key: *b"vmsk",
                data: pictura_codec::encode_vector_mask(std::slice::from_ref(&outline), SIZE, SIZE)
                    .into(),
            },
            LayerBlock {
                key: *b"vscg",
                data: vscg_solid(rgb).into(),
            },
        ],
        ..Default::default()
    })
}

fn pixel(name: &str) -> Layer {
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        name: name.into(),
        ..Default::default()
    })
}

fn group(blend: BlendMode, children: Vec<Layer>) -> Layer {
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        is_group: true,
        blend,
        children: pictura_core::LayerStack::from_layers(children),
        ..Default::default()
    })
}

fn adjustment() -> Layer {
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(AdjustmentData {
            key: *b"nvrt",
            data: Vec::new().into(),
        }),
        ..Default::default()
    })
}

fn doc(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

/// The nodes for `layers`, plus each raster call's scratch layer names. A
/// raster is filled with the count of its scratch layers, so a test can tell a
/// flattened run from a single layer.
fn nodes(layers: Vec<Layer>) -> (Vec<SvgNode>, Vec<Vec<String>>) {
    let doc = doc(layers);
    let mut calls = Vec::new();
    let nodes = svg_nodes(&doc, |scratch: &Document| {
        calls.push(scratch.layers.iter().map(|l| l.name.clone()).collect());
        vec![scratch.layers.len() as u8; (SIZE * SIZE * 4) as usize]
    });
    (nodes, calls)
}

fn raster_fill(node: &SvgNode) -> u8 {
    match &node.content {
        SvgContent::Raster { rgba, .. } => rgba[0],
        other => panic!("expected a raster, got {other:?}"),
    }
}

#[test]
fn an_imported_shape_layer_becomes_a_filled_primitive() {
    let (nodes, calls) = nodes(vec![shape_layer([10, 20, 30])]);
    assert!(calls.is_empty(), "a shape needs no raster");
    match &nodes[0].content {
        SvgContent::Shape {
            paint, subpaths, ..
        } => {
            assert!(matches!(paint, SvgPaint::Solid([10, 20, 30, 255])));
            assert!(!subpaths.is_empty());
        }
        other => panic!("expected a shape, got {other:?}"),
    }
}

#[test]
fn a_shape_fill_opacity_scales_its_fill_alpha() {
    let mut layer = shape_layer([1, 2, 3]);
    layer.fill = 128;
    let (nodes, _) = nodes(vec![layer]);
    assert!(matches!(
        nodes[0].content,
        SvgContent::Shape {
            paint: SvgPaint::Solid([1, 2, 3, 255]),
            fill_opacity: 128,
            ..
        }
    ));
}

#[test]
fn a_shape_with_effects_is_rasterized() {
    let mut layer = shape_layer([1, 2, 3]);
    assert!(crate::set_layer_style_value(
        &mut layer,
        "dropShadow.on",
        1.0
    ));
    let (nodes, _) = nodes(vec![layer]);
    raster_fill(&nodes[0]);
}

#[test]
fn a_pixel_layer_raster_does_not_bake_its_opacity_or_blend() {
    let mut layer = pixel("p");
    layer.opacity = 128;
    layer.blend = BlendMode::Multiply;
    let doc = doc(vec![layer]);
    let mut seen = Vec::new();
    let nodes = svg_nodes(&doc, |scratch: &Document| {
        seen.push((scratch.layers[0].opacity, scratch.layers[0].blend));
        vec![0; (SIZE * SIZE * 4) as usize]
    });
    assert_eq!(seen, vec![(255, BlendMode::Normal)]);
    assert_eq!(nodes[0].opacity, 128);
    assert_eq!(nodes[0].blend, BlendMode::Multiply);
}

#[test]
fn hidden_and_zero_opacity_layers_are_skipped() {
    let mut hidden = pixel("hidden");
    hidden.visible = false;
    let mut clear = pixel("clear");
    clear.opacity = 0;
    let (nodes, calls) = nodes(vec![hidden, clear]);
    assert!(nodes.is_empty());
    assert!(calls.is_empty());
}

#[test]
fn a_group_recurses_into_its_children() {
    let mut faded = group(
        BlendMode::Normal,
        vec![shape_layer([1, 2, 3]), shape_layer([4, 5, 6])],
    );
    faded.opacity = 128;
    let (nodes, calls) = nodes(vec![faded]);
    assert!(calls.is_empty(), "a vector group needs no raster");
    assert_eq!(nodes[0].opacity, 128);
    match &nodes[0].content {
        SvgContent::Group { children, isolated } => {
            assert!(*isolated);
            assert_eq!(children.len(), 2);
            assert!(matches!(children[1].content, SvgContent::Shape { .. }));
        }
        other => panic!("expected a group, got {other:?}"),
    }
}

#[test]
fn a_pass_through_group_is_not_isolated() {
    let (nodes, _) = nodes(vec![group(
        BlendMode::PassThrough,
        vec![shape_layer([1, 2, 3])],
    )]);
    assert!(matches!(
        nodes[0].content,
        SvgContent::Group {
            isolated: false,
            ..
        }
    ));
}

#[test]
fn a_clipping_stack_rasterizes_as_one_image() {
    let mut clipped = pixel("clipped");
    clipped.clipping = true;
    let (nodes, calls) = nodes(vec![pixel("below"), pixel("base"), clipped]);
    assert_eq!(nodes.len(), 2);
    assert_eq!(calls[1], vec!["base".to_string(), "clipped".to_string()]);
    assert_eq!(raster_fill(&nodes[1]), 2);
}

#[test]
fn a_blended_clipping_stack_flattens_the_backdrop() {
    let mut clipped = pixel("clipped");
    clipped.clipping = true;
    clipped.blend = BlendMode::Multiply;
    let (nodes, _) = nodes(vec![pixel("below"), pixel("base"), clipped]);
    assert_eq!(nodes.len(), 1);
    assert_eq!(raster_fill(&nodes[0]), 3);
}

#[test]
fn a_non_css_blend_flattens_everything_beneath_it() {
    let mut burn = pixel("burn");
    burn.blend = BlendMode::LinearBurn;
    let (nodes, _) = nodes(vec![pixel("a"), shape_layer([1, 2, 3]), burn, pixel("top")]);
    assert_eq!(nodes.len(), 2, "the flattened run, then the top layer");
    assert_eq!(raster_fill(&nodes[0]), 3);
    assert_eq!(raster_fill(&nodes[1]), 1);
}

#[test]
fn an_adjustment_layer_flattens_everything_beneath_it() {
    let (nodes, _) = nodes(vec![pixel("a"), pixel("b"), adjustment()]);
    assert_eq!(nodes.len(), 1);
    assert_eq!(raster_fill(&nodes[0]), 3);
}

#[test]
fn an_adjustment_in_an_isolated_group_flattens_only_the_group() {
    let inner = group(BlendMode::Normal, vec![pixel("x"), adjustment()]);
    let (nodes, _) = nodes(vec![pixel("outside"), inner]);
    assert_eq!(nodes.len(), 2);
    match &nodes[1].content {
        SvgContent::Group { children, .. } => {
            assert_eq!(children.len(), 1);
            assert_eq!(raster_fill(&children[0]), 2);
        }
        other => panic!("expected a group, got {other:?}"),
    }
}

#[test]
fn an_adjustment_in_a_pass_through_group_flattens_the_parent() {
    let inner = group(BlendMode::PassThrough, vec![pixel("x"), adjustment()]);
    let (nodes, _) = nodes(vec![pixel("outside"), inner]);
    assert_eq!(nodes.len(), 1);
    assert_eq!(raster_fill(&nodes[0]), 2, "outside + the group");
}

#[test]
fn encode_document_svg_nests_a_group() {
    let doc = doc(vec![group(BlendMode::Normal, vec![shape_layer([1, 2, 3])])]);
    let svg = encode_document_svg(&doc, |_: &Document| Vec::new());
    assert!(svg.contains("<g style=\"isolation:isolate\">"), "{svg}");
    assert!(svg.contains("fill=\"#010203\""), "{svg}");
}

#[test]
fn a_type_layer_uses_its_raster_fallback() {
    let mut text = pixel("text");
    pictura_codec::edit_layer(text.as_mut(), |parts| {
        parts.extra_blocks.push(LayerBlock {
            key: *b"TySh",
            data: Vec::new().into(),
        })
    });
    let (nodes, _) = nodes(vec![text]);
    raster_fill(&nodes[0]);
}

fn gradient_layer(kind: GradientKind) -> Layer {
    let mut layer = shape_layer([0, 0, 0]);
    pictura_codec::edit_layer(layer.as_mut(), |parts| {
        parts.adjustment = Some(crate::encode_gradient_fill(
            kind,
            &[
                GradientStop {
                    location: 0,
                    color: [255, 0, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [0, 0, 255],
                },
            ],
            0.0,
        ))
    });
    layer
}

fn pattern_layer() -> Layer {
    let mut layer = shape_layer([0, 0, 0]);
    pictura_codec::edit_layer(layer.as_mut(), |parts| {
        parts.adjustment = Some(crate::encode_pattern_fill(&PatternFillParams {
            pattern_id: "missing".into(),
            scale: 100.0,
            link_with_layer: true,
            origin: (0, 0),
        }))
    });
    layer
}

#[test]
fn a_linear_gradient_shape_becomes_a_gradient_fill() {
    let (nodes, calls) = nodes(vec![gradient_layer(GradientKind::Linear)]);
    assert!(calls.is_empty(), "a gradient shape needs no raster");
    assert!(matches!(
        nodes[0].content,
        SvgContent::Shape {
            paint: SvgPaint::Gradient(SvgGradient::Linear { .. }),
            ..
        }
    ));
}

#[test]
fn a_radial_gradient_shape_becomes_a_radial_fill() {
    let (nodes, calls) = nodes(vec![gradient_layer(GradientKind::Radial)]);
    assert!(calls.is_empty(), "a gradient shape needs no raster");
    assert!(matches!(
        nodes[0].content,
        SvgContent::Shape {
            paint: SvgPaint::Gradient(SvgGradient::Radial { .. }),
            ..
        }
    ));
}

#[test]
fn an_unsupported_gradient_kind_is_rasterized() {
    for kind in [
        GradientKind::Angle,
        GradientKind::Reflected,
        GradientKind::Diamond,
    ] {
        let (nodes, _) = nodes(vec![gradient_layer(kind)]);
        raster_fill(&nodes[0]);
    }
}

#[test]
fn a_gradient_fill_layer_becomes_a_canvas_primitive() {
    let layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: pictura_core::PsdRect {
            left: 2,
            top: 3,
            right: 7,
            bottom: 8,
        },
        fill: 255,
        adjustment: Some(crate::encode_gradient_fill(
            GradientKind::Linear,
            &[
                GradientStop {
                    location: 0,
                    color: [255, 0, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [0, 0, 255],
                },
            ],
            0.0,
        )),
        ..Default::default()
    });
    let (nodes, calls) = nodes(vec![layer]);
    assert!(calls.is_empty(), "a gradient fill needs no raster");
    match &nodes[0].content {
        SvgContent::Shape {
            subpaths,
            paint: SvgPaint::Gradient(_),
            ..
        } => {
            assert_eq!(subpaths[0].points[0].anchor, (2.0, 3.0));
            assert_eq!(subpaths[0].points[2].anchor, (7.0, 8.0));
        }
        other => panic!("expected a gradient shape, got {other:?}"),
    }
}

fn white_mask() -> pictura_core::LayerMask {
    pictura_core::LayerMask {
        rect: pictura_core::PsdRect {
            left: 0,
            top: 0,
            right: SIZE as i32,
            bottom: SIZE as i32,
        },
        default_color: 255,
        data: Some(vec![255u8; (SIZE * SIZE) as usize].into()),
        ..Default::default()
    }
}

#[test]
fn a_full_white_layer_mask_keeps_a_gradient_fill_vector() {
    let mut layer = gradient_layer(GradientKind::Linear);
    layer.mask = Some(white_mask());
    let (nodes, calls) = nodes(vec![layer]);
    assert!(calls.is_empty(), "a no-op mask must not force a raster");
    assert!(matches!(
        nodes[0].content,
        SvgContent::Shape {
            paint: SvgPaint::Gradient(SvgGradient::Linear { .. }),
            ..
        }
    ));
}

#[test]
fn a_painted_layer_mask_rasterizes_a_gradient_fill() {
    let mut layer = gradient_layer(GradientKind::Linear);
    layer.mask = Some(pictura_core::LayerMask {
        data: Some(vec![0u8; (SIZE * SIZE) as usize].into()),
        ..white_mask()
    });
    let (nodes, _) = nodes(vec![layer]);
    raster_fill(&nodes[0]);
}

#[test]
fn a_pattern_shape_becomes_a_pattern_fill() {
    let (nodes, calls) = nodes(vec![pattern_layer()]);
    assert!(calls.is_empty(), "a pattern shape needs no raster");
    match &nodes[0].content {
        SvgContent::Shape {
            paint: SvgPaint::Pattern(pattern),
            ..
        } => {
            assert_eq!((pattern.width, pattern.height), (1, 1));
            assert_eq!(pattern.rgba, vec![128, 128, 128, 255]);
        }
        other => panic!("expected a pattern, got {other:?}"),
    }
}

fn stroked_layer(position: crate::StrokePosition) -> Layer {
    let mut layer = shape_layer([1, 2, 3]);
    crate::set_shape_stroke(
        &mut layer,
        Some(&crate::ShapeStroke::solid([255, 0, 0], 3, position)),
    );
    layer
}

#[test]
fn a_solid_shape_stroke_becomes_a_stroke() {
    for (position, expected) in [
        (crate::StrokePosition::Outside, SvgStrokePosition::Outside),
        (crate::StrokePosition::Inside, SvgStrokePosition::Inside),
        (crate::StrokePosition::Center, SvgStrokePosition::Center),
    ] {
        let (nodes, calls) = nodes(vec![stroked_layer(position)]);
        assert!(calls.is_empty(), "a stroked shape needs no raster");
        match &nodes[0].content {
            SvgContent::Shape {
                stroke: Some(stroke),
                ..
            } => {
                assert_eq!(stroke.rgb, [255, 0, 0]);
                assert_eq!(stroke.width, 3.0);
                assert_eq!(stroke.position, expected);
            }
            other => panic!("expected a stroke, got {other:?}"),
        }
    }
}

#[test]
fn a_shape_with_another_effect_is_rasterized() {
    let mut layer = stroked_layer(crate::StrokePosition::Outside);
    assert!(crate::set_layer_style_value(
        &mut layer,
        "dropShadow.on",
        1.0
    ));
    let (nodes, _) = nodes(vec![layer]);
    raster_fill(&nodes[0]);
}

#[test]
fn raster_normalization_keeps_edited_typed_content() {
    let original = pictura_codec::encode_solid_color_fill([10, 20, 30]);
    let mut layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        opacity: 123,
        blend: BlendMode::Multiply,
        adjustment: Some(original.clone()),
        ..Default::default()
    });
    assert!(layer.set_fill_content(pictura_core::FillContent::Solid([40, 50, 60, 77])));
    let neutral = normalized(&layer);
    assert_eq!(neutral.opacity, 255);
    assert_eq!(neutral.blend, BlendMode::Normal);
    assert_eq!(neutral.fill_content(), layer.fill_content());
    assert!(neutral.fill_was_edited());
    assert_eq!(neutral.adjustment(), Some(&original));
}
