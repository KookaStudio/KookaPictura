use super::*;
use pictura_core::path::PathPoint;
use pictura_core::VectorSubpath;

fn doc(width: u32, height: u32) -> Document {
    Document::new(
        width,
        height,
        pictura_core::ColorMode::Rgb,
        pictura_core::BitDepth::Eight,
    )
}

fn svg(width: u32, height: u32, nodes: Vec<SvgNode>) -> String {
    encode_svg(
        &doc(width, height),
        &SvgOptions {
            nodes,
            precision: 3,
        },
    )
}

fn straight_subpath() -> Subpath {
    Subpath {
        points: vec![
            PathPoint {
                anchor: (0.0, 0.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
            PathPoint {
                anchor: (10.0, 0.0),
                in_handle: None,
                out_handle: None,
                smooth: false,
            },
        ],
        closed: false,
    }
}

fn raster(width: u32, height: u32) -> SvgContent {
    SvgContent::Raster {
        rgba: vec![0u8; (width * height * 4) as usize],
        width,
        height,
    }
}

#[test]
fn straight_segment_is_a_line() {
    assert_eq!(subpath_data(&straight_subpath(), 3), "M 0 0 L 10 0");
}

#[test]
fn curved_closed_subpath_is_cubics_with_z() {
    let subpath = Subpath {
        points: vec![
            PathPoint {
                anchor: (0.0, 0.0),
                in_handle: Some((-1.0, 0.0)),
                out_handle: Some((1.0, 0.0)),
                smooth: true,
            },
            PathPoint {
                anchor: (2.0, 2.0),
                in_handle: Some((1.0, 2.0)),
                out_handle: Some((3.0, 2.0)),
                smooth: true,
            },
        ],
        closed: true,
    };
    let d = subpath_data(&subpath, 3);
    assert!(d.starts_with("M 0 0 C "), "{d}");
    assert!(d.ends_with(" Z"), "{d}");
    assert_eq!(d.matches('C').count(), 2, "two segments: {d}");
}

#[test]
fn coordinates_trim_trailing_zeros() {
    assert_eq!(format_coord(1.5, 3), "1.5");
    assert_eq!(format_coord(2.0, 3), "2");
    assert_eq!(format_coord(-0.0004, 3), "0");
    assert_eq!(format_coord(1.23456, 2), "1.23");
}

#[test]
fn a_raster_document_matches_the_cc_sample() {
    let svg = svg(1000, 652, vec![SvgNode::plain(raster(1000, 652))]);
    assert!(
        svg.starts_with(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" preserveAspectRatio=\"xMidYMid\" width=\"1000\" height=\"652\" viewBox=\"0 0 1000 652\">\n"
        ),
        "{svg}"
    );
    assert!(
        svg.contains("  <image xlink:href=\"data:img/png;base64,"),
        "{svg}"
    );
    assert!(
        svg.contains("\" width=\"1000\" height=\"652\"/>\n"),
        "{svg}"
    );
    assert!(svg.ends_with("</svg>\n"), "{svg}");
}

#[test]
fn image_element_is_well_formed() {
    assert_eq!(
        image_element("AAAA", 10, 5, None),
        "<image xlink:href=\"data:img/png;base64,AAAA\" width=\"10\" height=\"5\"/>"
    );
}

#[test]
fn a_zero_opacity_node_is_skipped() {
    let node = SvgNode {
        opacity: 0,
        ..SvgNode::plain(raster(4, 4))
    };
    assert!(!svg(4, 4, vec![node]).contains("image"));
}

#[test]
fn opacity_wraps_in_a_group() {
    let node = SvgNode {
        opacity: 128,
        blend: BlendMode::Multiply,
        ..SvgNode::plain(raster(4, 4))
    };
    let svg = svg(4, 4, vec![node]);
    assert!(
        svg.contains("<g opacity=\"0.502\" style=\"mix-blend-mode:multiply\">"),
        "{svg}"
    );
}

#[test]
fn a_vector_mask_becomes_a_clip_path() {
    let mask = VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![[0, 0], [2560, 0], [2560, 2560], [0, 2560]],
        }],
        ..Default::default()
    };
    let clipped = |mask: &VectorMask| SvgNode {
        clip: Some(mask.clone()),
        ..SvgNode::plain(raster(10, 10))
    };
    let svg = svg(10, 10, vec![clipped(&mask), clipped(&mask)]);
    assert!(svg.contains("<clipPath id=\"clip0\">"), "{svg}");
    assert!(svg.contains("clip-path=\"url(#clip0)\""), "{svg}");
    assert!(
        svg.contains("clip-path=\"url(#clip1)\""),
        "ids are unique: {svg}"
    );
    assert!(svg.contains("M 0 0 L 10 0 L 10 10 L 0 10 Z"), "{svg}");
}

fn closed_square() -> Subpath {
    Subpath {
        points: [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
            .into_iter()
            .map(|anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .collect(),
        closed: true,
    }
}

fn shape(paint: SvgPaint, stroke: Option<SvgStroke>) -> SvgNode {
    SvgNode::plain(SvgContent::Shape {
        subpaths: vec![closed_square()],
        paint,
        fill_opacity: 255,
        stroke,
        live: None,
    })
}

#[test]
fn a_shape_content_emits_a_filled_path() {
    let svg = svg(
        10,
        10,
        vec![shape(SvgPaint::Solid([10, 20, 30, 255]), None)],
    );
    assert!(
        svg.contains("<path d=\"M 0 0 L 10 0 L 10 10 L 0 10 L 0 0 Z\""),
        "{svg}"
    );
    assert!(svg.contains("fill=\"#0a141e\""), "{svg}");
}

#[test]
fn a_live_shape_emits_its_parametric_primitive() {
    let shape = |fill, live| {
        SvgNode::plain(SvgContent::Shape {
            subpaths: Vec::new(),
            paint: SvgPaint::Solid(fill),
            fill_opacity: 255,
            stroke: None,
            live: Some(live),
        })
    };
    let rect = LiveShape {
        origin_type: crate::ORIGIN_RECTANGLE,
        bounds: (1.0, 2.0, 6.0, 8.0),
        radii: [0.0; 4],
    };
    let svg_rect = svg(10, 10, vec![shape([255, 0, 0, 128], rect)]);
    assert!(
        svg_rect.contains("<rect x=\"1\" y=\"2\" width=\"5\" height=\"6\""),
        "{svg_rect}"
    );
    assert!(
        svg_rect.contains("fill=\"#ff0000\" fill-opacity=\"0.502\""),
        "{svg_rect}"
    );

    let ellipse = LiveShape {
        origin_type: crate::ORIGIN_ELLIPSE,
        bounds: (0.0, 0.0, 10.0, 4.0),
        radii: [0.0; 4],
    };
    let svg_ellipse = svg(10, 10, vec![shape([0, 0, 0, 255], ellipse)]);
    assert!(
        svg_ellipse.contains("<ellipse cx=\"5\" cy=\"2\" rx=\"5\" ry=\"2\""),
        "{svg_ellipse}"
    );
}

fn two_stops() -> Vec<SvgGradientStop> {
    vec![
        SvgGradientStop {
            offset: 0.0,
            rgba: [255, 0, 0, 255],
        },
        SvgGradientStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ]
}

#[test]
fn a_linear_gradient_fill_registers_a_def() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Gradient(SvgGradient::Linear {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                stops: two_stops(),
            }),
            None,
        )],
    );
    assert!(
        svg.contains(
            "<linearGradient id=\"grad0\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"10\" y2=\"0\">"
        ),
        "{svg}"
    );
    assert!(
        svg.contains("<stop offset=\"0\" stop-color=\"#ff0000\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains("<stop offset=\"1\" stop-color=\"#0000ff\"/>"),
        "{svg}"
    );
    assert!(svg.contains("fill=\"url(#grad0)\""), "{svg}");
}

#[test]
fn a_radial_gradient_fill_registers_a_def() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Gradient(SvgGradient::Radial {
                cx: 5.0,
                cy: 5.0,
                rx: 5.0,
                ry: 3.0,
                stops: two_stops(),
            }),
            None,
        )],
    );
    assert!(
        svg.contains(
            "<radialGradient id=\"grad0\" gradientUnits=\"userSpaceOnUse\" cx=\"0\" cy=\"0\" r=\"1\" gradientTransform=\"translate(5 5) scale(5 3)\">"
        ),
        "{svg}"
    );
    assert!(svg.contains("fill=\"url(#grad0)\""), "{svg}");
}

#[test]
fn a_pattern_fill_registers_a_pattern_tile() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Pattern(SvgPattern {
                rgba: vec![255; 2 * 2 * 4],
                width: 2,
                height: 2,
                x: 3.0,
                y: 4.0,
            }),
            None,
        )],
    );
    assert!(
        svg.contains(
            "<pattern id=\"pat0\" patternUnits=\"userSpaceOnUse\" patternContentUnits=\"userSpaceOnUse\" x=\"3\" y=\"4\" width=\"2\" height=\"2\">"
        ),
        "{svg}"
    );
    assert!(svg.contains("data:img/png;base64,"), "{svg}");
    assert!(svg.contains("fill=\"url(#pat0)\""), "{svg}");
}

fn stroke(position: SvgStrokePosition) -> SvgStroke {
    SvgStroke {
        rgb: [255, 0, 0],
        width: 2.0,
        opacity: 0.5,
        position,
    }
}

#[test]
fn an_outside_stroke_emits_a_masked_stroke_element() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Solid([10, 20, 30, 255]),
            Some(stroke(SvgStrokePosition::Outside)),
        )],
    );
    assert!(
        svg.contains(
            "<mask id=\"mask0\" maskUnits=\"userSpaceOnUse\" maskContentUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"10\" height=\"10\">"
        ),
        "{svg}"
    );
    assert!(
        svg.contains("<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"white\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains("fill=\"black\" fill-rule=\"evenodd\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains(
            "fill=\"none\" stroke=\"#ff0000\" mask=\"url(#mask0)\" stroke-width=\"4\" stroke-opacity=\"0.5\""
        ),
        "{svg}"
    );
}

#[test]
fn an_inside_stroke_emits_a_masked_stroke_element() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Solid([10, 20, 30, 255]),
            Some(stroke(SvgStrokePosition::Inside)),
        )],
    );
    assert!(
        svg.contains("<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"black\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains("fill=\"white\" fill-rule=\"evenodd\"/>"),
        "{svg}"
    );
    assert!(svg.contains("mask=\"url(#mask0)\""), "{svg}");
    assert!(svg.contains("stroke-width=\"4\""), "{svg}");
}

#[test]
fn a_center_stroke_stays_on_one_element() {
    let svg = svg(
        10,
        10,
        vec![shape(
            SvgPaint::Solid([10, 20, 30, 255]),
            Some(stroke(SvgStrokePosition::Center)),
        )],
    );
    assert!(!svg.contains("<mask"), "{svg}");
    assert!(
        svg.contains("stroke=\"#ff0000\" stroke-width=\"2\" stroke-opacity=\"0.5\""),
        "{svg}"
    );
}

#[test]
fn a_group_nests_its_children() {
    let group = |isolated, opacity| SvgNode {
        opacity,
        ..SvgNode::plain(SvgContent::Group {
            children: vec![SvgNode::plain(raster(2, 2))],
            isolated,
        })
    };
    let svg_pass = svg(2, 2, vec![group(false, 255)]);
    assert!(svg_pass.contains("  <g>\n    <image "), "{svg_pass}");
    let svg_iso = svg(2, 2, vec![group(true, 255)]);
    assert!(
        svg_iso.contains("<g style=\"isolation:isolate\">"),
        "{svg_iso}"
    );
    let svg_faded = svg(2, 2, vec![group(true, 128)]);
    assert!(svg_faded.contains("<g opacity=\"0.502\">"), "{svg_faded}");
    assert!(!svg_faded.contains("isolation"), "{svg_faded}");
}

#[test]
fn an_empty_group_is_omitted() {
    let group = SvgNode::plain(SvgContent::Group {
        children: Vec::new(),
        isolated: true,
    });
    assert!(!svg(2, 2, vec![group]).contains("<g"));
}

#[test]
fn primitives_format_their_attributes() {
    assert_eq!(
        rect_element(1.0, 2.0, 30.0, 40.0, 3),
        "<rect x=\"1\" y=\"2\" width=\"30\" height=\"40\"/>"
    );
    assert_eq!(
        rounded_rect_element(0.0, 0.0, 10.0, 10.0, 2.5, 3),
        "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" rx=\"2.5\" ry=\"2.5\"/>"
    );
    assert_eq!(
        ellipse_element(5.0, 5.0, 5.0, 2.0, 3),
        "<ellipse cx=\"5\" cy=\"5\" rx=\"5\" ry=\"2\"/>"
    );
}

#[test]
fn text_and_href_are_xml_escaped() {
    let svg = text_element(0.0, 0.0, "a<b&c", "Font \"X\"", 12.0, [255, 0, 0, 128], 3);
    assert!(svg.contains("a&lt;b&amp;c"), "{svg}");
    assert!(svg.contains("font-family=\"Font &quot;X&quot;\""), "{svg}");
    assert!(
        svg.contains("fill=\"#ff0000\" fill-opacity=\"0.502\""),
        "{svg}"
    );
}

#[test]
fn only_css_blend_modes_map() {
    assert_eq!(blend_css(BlendMode::Multiply), Some("multiply"));
    assert_eq!(blend_css(BlendMode::Normal), None);
    assert_eq!(blend_css(BlendMode::Dissolve), None);
    assert_eq!(blend_css(BlendMode::LinearBurn), None);
}
