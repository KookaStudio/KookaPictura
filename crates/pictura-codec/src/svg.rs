//! SVG export: serialize a document to Photoshop-CC-shaped SVG text.
//!
//! Photoshop's SVG export is not a traced raster. It walks the document's
//! vector content and emits SVG primitives, embedding raster and effect content
//! as a base64 PNG fallback. The header and the raster fallback shape are taken
//! from a shipped CC sample; the primitives are inferred and marked below.
//!
//! This module is Qt-free and does no compositing or fill decoding: the caller
//! supplies each top-level layer's [`SvgContent`] — a decoded shape (outline,
//! fill, live form) or a raster fallback as RGBA8888, which the codec encodes to
//! a base64 PNG. `pictura-render` derives that content
//! (`pictura_render::layer_content`), because it owns the `vscg` fill decode and
//! the compositor, and it already depends on this crate.

use pictura_core::path::Subpath;
use pictura_core::{BlendMode, Document, Layer, VectorFillRule, VectorMask};

use crate::LiveShape;

/// One top-level layer's SVG content, supplied by the caller. The engine cannot
/// derive this itself: a shape layer's fill is a `vscg` vector-fill block
/// decoded in `pictura-render`, and a pixel layer's fallback is a raster the
/// caller composites. Index-aligned with `Document::layers`.
#[derive(Clone, Debug)]
pub enum SvgContent {
    /// A shape layer: its outline in document pixels, its solid fill, and the
    /// live-shape form when the layer is still parametric.
    Shape {
        subpaths: Vec<Subpath>,
        fill: [u8; 4],
        live: Option<LiveShape>,
    },
    /// A raster fallback as straight-alpha RGBA8888 at its own size; the codec
    /// encodes it to a base64 PNG `<image>`.
    Raster {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// No content: the layer is skipped.
    None,
}

/// SVG export options.
#[derive(Clone, Debug)]
pub struct SvgOptions {
    /// Per-top-level-layer content; see [`SvgContent`].
    pub contents: Vec<SvgContent>,
    /// Decimal places for coordinates, clamped to `0..=6`.
    pub precision: u8,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            contents: Vec::new(),
            precision: 3,
        }
    }
}

/// Serialize `doc` to SVG text. `options.contents` supplies each top-level
/// layer's content; see [`SvgOptions`].
pub fn encode_svg(doc: &Document, options: &SvgOptions) -> String {
    let precision = options.precision.min(6);
    let mut defs = Vec::new();
    let mut body = Vec::new();
    for (index, layer) in doc.layers.iter().enumerate() {
        if !layer.visible || layer.opacity == 0 {
            continue;
        }
        let element = match options.contents.get(index) {
            Some(SvgContent::Shape {
                subpaths,
                fill,
                live,
            }) => match shape_element(subpaths, *fill, live.as_ref(), precision) {
                Some(element) => element,
                None => continue,
            },
            Some(SvgContent::Raster {
                rgba,
                width,
                height,
            }) => {
                let Some(href) = crate::encode_png_base64(rgba, *width, *height) else {
                    continue;
                };
                let clip = clip_path(layer.vector_mask.as_ref(), index, precision);
                if let Some((id, path)) = &clip {
                    defs.push(format!(
                        "  <clipPath id=\"{id}\">\n    <path d=\"{}\"/>\n  </clipPath>",
                        escape_xml(path)
                    ));
                }
                image_element(&href, *width, *height, clip.as_ref().map(|c| c.0.as_str()))
            }
            _ => continue,
        };
        body.push(group_layer(&element, layer));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" preserveAspectRatio=\"xMidYMid\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        doc.width, doc.height, doc.width, doc.height
    ));
    if !defs.is_empty() {
        out.push_str("  <defs>\n");
        out.push_str(&defs.join("\n"));
        out.push_str("\n  </defs>\n");
    }
    for element in body {
        out.push_str(&indent_lines(&element, 1));
        out.push('\n');
    }
    out.push_str("</svg>\n");
    out
}

/// The primitive for a shape layer: a live shape's `rect` / rounded `rect` /
/// `ellipse` when still parametric, else the outline as a `<path>`. `None` when
/// the outline is empty and there is no live form.
fn shape_element(
    subpaths: &[Subpath],
    fill: [u8; 4],
    live: Option<&LiveShape>,
    precision: u8,
) -> Option<String> {
    let base = match live {
        Some(shape) if shape.origin_type == crate::ORIGIN_ELLIPSE => {
            let (l, t, r, b) = shape.bounds;
            ellipse_element(
                (l + r) / 2.0,
                (t + b) / 2.0,
                (r - l) / 2.0,
                (b - t) / 2.0,
                precision,
            )
        }
        Some(shape) if shape.origin_type == crate::ORIGIN_ROUNDED_RECTANGLE => {
            let (l, t, r, b) = shape.bounds;
            // SVG has one corner radius; use the largest so no corner is
            // over-rounded. ponytail: per-corner `rx`/`ry` is unsupported.
            let radius = shape.radii.iter().copied().fold(0.0_f64, f64::max);
            rounded_rect_element(l, t, r - l, b - t, radius, precision)
        }
        Some(shape) if shape.origin_type == crate::ORIGIN_RECTANGLE => {
            let (l, t, r, b) = shape.bounds;
            rect_element(l, t, r - l, b - t, precision)
        }
        _ => {
            let data = path_data(subpaths, precision);
            if data.is_empty() {
                return None;
            }
            path_element(&data, VectorFillRule::EvenOdd)
        }
    };
    Some(with_fill(&base, fill))
}

/// Insert the fill attributes into a self-closing primitive.
fn with_fill(element: &str, fill: [u8; 4]) -> String {
    let attrs = if fill[3] == 255 {
        format!(" fill=\"#{:02x}{:02x}{:02x}\"", fill[0], fill[1], fill[2])
    } else {
        format!(
            " fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{}\"",
            fill[0],
            fill[1],
            fill[2],
            format_coord(f64::from(fill[3]) / 255.0, 3)
        )
    };
    match element.strip_suffix("/>") {
        Some(head) => format!("{head}{attrs}/>"),
        None => element.to_string(),
    }
}

/// Wrap `element` in a `<g>` when the layer carries a non-default opacity or a
/// CSS-representable blend mode.
fn group_layer(element: &str, layer: &Layer) -> String {
    let mut attrs = Vec::new();
    if layer.opacity != 255 {
        attrs.push(format!(
            "opacity=\"{}\"",
            format_coord(f64::from(layer.opacity) / 255.0, 3)
        ));
    }
    if let Some(css) = blend_css(layer.blend) {
        attrs.push(format!("style=\"mix-blend-mode:{css}\""));
    }
    if attrs.is_empty() {
        return element.to_string();
    }
    format!(
        "<g {}>\n{}\n</g>",
        attrs.join(" "),
        indent_lines(element, 1)
    )
}

/// The `d` data for a run of editable subpaths (the Work Path model).
pub fn path_data(subpaths: &[Subpath], precision: u8) -> String {
    let precision = precision.min(6);
    let mut out = String::new();
    for subpath in subpaths {
        let data = subpath_data(subpath, precision);
        if data.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&data);
    }
    out
}

/// The `d` data for one subpath: `M`, then a `C` (or `L` for a straight segment)
/// per segment, and `Z` when closed.
pub fn subpath_data(subpath: &Subpath, precision: u8) -> String {
    let precision = precision.min(6);
    let Some(first) = subpath.points.first() else {
        return String::new();
    };
    let mut out = format!("M {}", point(first.anchor, precision));
    for i in 0..subpath.segment_count() {
        let Some([p0, c1, c2, p3]) = subpath.segment(i) else {
            continue;
        };
        if c1 == p0 && c2 == p3 {
            out.push_str(&format!(" L {}", point(p3, precision)));
        } else {
            out.push_str(&format!(
                " C {} {} {}",
                point(c1, precision),
                point(c2, precision),
                point(p3, precision)
            ));
        }
    }
    if subpath.closed {
        out.push_str(" Z");
    }
    out
}

/// A filled/stroked `<path>` element. A `fill_rule` of [`VectorFillRule::EvenOdd`]
/// is emitted explicitly.
pub fn path_element(data: &str, fill_rule: VectorFillRule) -> String {
    match fill_rule {
        VectorFillRule::EvenOdd => {
            format!("<path d=\"{}\" fill-rule=\"evenodd\"/>", escape_xml(data))
        }
        VectorFillRule::NonZero => format!("<path d=\"{}\"/>", escape_xml(data)),
    }
}

/// A base64 `<image>` at the document size, optionally clipped. The MIME
/// subtype is `img/png` to match the CC sample.
pub fn image_element(href: &str, width: u32, height: u32, clip_id: Option<&str>) -> String {
    let image = format!(
        "<image xlink:href=\"data:img/png;base64,{}\" width=\"{}\" height=\"{}\"/>",
        escape_xml(href),
        width,
        height
    );
    match clip_id {
        Some(id) => format!(
            "<g clip-path=\"url(#{})\">\n{}\n</g>",
            escape_xml(id),
            indent_lines(&image, 1)
        ),
        None => image,
    }
}

/// A live-shape rectangle primitive.
pub fn rect_element(x: f64, y: f64, width: f64, height: f64, precision: u8) -> String {
    let p = precision.min(6);
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
        format_coord(x, p),
        format_coord(y, p),
        format_coord(width, p),
        format_coord(height, p)
    )
}

/// A live-shape rounded rectangle primitive (a single uniform corner radius).
pub fn rounded_rect_element(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    precision: u8,
) -> String {
    let p = precision.min(6);
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\"/>",
        format_coord(x, p),
        format_coord(y, p),
        format_coord(width, p),
        format_coord(height, p),
        format_coord(radius, p),
        format_coord(radius, p)
    )
}

/// A live-shape ellipse primitive.
pub fn ellipse_element(cx: f64, cy: f64, rx: f64, ry: f64, precision: u8) -> String {
    let p = precision.min(6);
    format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"/>",
        format_coord(cx, p),
        format_coord(cy, p),
        format_coord(rx, p),
        format_coord(ry, p)
    )
}

/// A `<text>` element. The fill is a straight RGBA in `0..=255`.
pub fn text_element(
    x: f64,
    y: f64,
    text: &str,
    font: &str,
    size: f64,
    fill: [u8; 4],
    precision: u8,
) -> String {
    let p = precision.min(6);
    format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{}\">{}</text>",
        format_coord(x, p),
        format_coord(y, p),
        escape_xml(font),
        format_coord(size, p),
        fill[0],
        fill[1],
        fill[2],
        format_coord(f64::from(fill[3]) / 255.0, 3),
        escape_xml(text)
    )
}

/// The CSS `mix-blend-mode` value for a blend mode, `None` when CSS cannot
/// represent it faithfully.
pub fn blend_css(blend: BlendMode) -> Option<&'static str> {
    Some(match blend {
        BlendMode::Normal | BlendMode::PassThrough => return None,
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color-dodge",
        BlendMode::ColorBurn => "color-burn",
        BlendMode::HardLight => "hard-light",
        BlendMode::SoftLight => "soft-light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hue",
        BlendMode::Saturation => "saturation",
        BlendMode::Color => "color",
        BlendMode::Luminosity => "luminosity",
        _ => return None,
    })
}

/// A `<clipPath>` for a layer's vector mask as `(id, d)`, `None` when the layer
/// has no usable mask. The mask is flattened to a polyline (1/256-pixel units).
fn clip_path(mask: Option<&VectorMask>, index: usize, precision: u8) -> Option<(String, String)> {
    let mask = mask?;
    if mask.disabled || mask.subpaths.is_empty() {
        return None;
    }
    let data = vector_mask_data(mask, precision);
    (!data.is_empty()).then(|| (format!("clip{index}"), data))
}

/// The `d` data for a decoded vector mask's flattened subpaths.
fn vector_mask_data(mask: &VectorMask, precision: u8) -> String {
    let precision = precision.min(6);
    let mut out = String::new();
    for subpath in &mask.subpaths {
        if subpath.points.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        let mut it = subpath.points.iter();
        let first = it.next().unwrap();
        out.push_str(&format!("M {}", point_i(first, precision)));
        for p in it {
            out.push_str(&format!(" L {}", point_i(p, precision)));
        }
        if subpath.closed {
            out.push_str(" Z");
        }
    }
    out
}

fn point(p: (f64, f64), precision: u8) -> String {
    format!(
        "{} {}",
        format_coord(p.0, precision),
        format_coord(p.1, precision)
    )
}

/// A mask vertex, stored in 1/256-pixel units.
fn point_i(p: &[i32; 2], precision: u8) -> String {
    let x = f64::from(p[0]) / 256.0;
    let y = f64::from(p[1]) / 256.0;
    format!(
        "{} {}",
        format_coord(x, precision),
        format_coord(y, precision)
    )
}

/// Round to `precision` decimals and trim trailing zeros (and a trailing dot).
fn format_coord(value: f64, precision: u8) -> String {
    let mut s = format!("{:.*}", usize::from(precision), value);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

/// Escape the five XML metacharacters for text and attribute values.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Indent every line of `s` by `level` two-space steps.
fn indent_lines(s: &str, level: usize) -> String {
    let pad = "  ".repeat(level);
    s.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::PathPoint;
    use pictura_core::VectorSubpath;

    fn doc(width: u32, height: u32, layers: Vec<Layer>) -> Document {
        let mut d = Document::new(
            width,
            height,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        d.layers = layers;
        d
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
        let layer = Layer {
            visible: true,
            ..Default::default()
        };
        let d = doc(1000, 652, vec![layer]);
        let options = SvgOptions {
            contents: vec![raster(1000, 652)],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
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
    fn hidden_and_zero_opacity_layers_are_skipped() {
        let hidden = Layer {
            visible: false,
            ..Default::default()
        };
        let transparent = Layer {
            opacity: 0,
            ..Default::default()
        };
        let d = doc(4, 4, vec![hidden, transparent]);
        let options = SvgOptions {
            contents: vec![raster(4, 4), raster(4, 4)],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(!svg.contains("image"), "{svg}");
    }

    #[test]
    fn opacity_wraps_in_a_group() {
        let layer = Layer {
            opacity: 128,
            blend: BlendMode::Multiply,
            ..Default::default()
        };
        let d = doc(4, 4, vec![layer]);
        let options = SvgOptions {
            contents: vec![raster(4, 4)],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(
            svg.contains("<g opacity=\"0.502\" style=\"mix-blend-mode:multiply\">"),
            "{svg}"
        );
    }

    #[test]
    fn a_vector_mask_becomes_a_clip_path() {
        let layer = Layer {
            vector_mask: Some(VectorMask {
                subpaths: vec![VectorSubpath {
                    closed: true,
                    operation: 1,
                    fill_rule: VectorFillRule::EvenOdd,
                    points: vec![[0, 0], [2560, 0], [2560, 2560], [0, 2560]],
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let d = doc(10, 10, vec![layer]);
        let options = SvgOptions {
            contents: vec![raster(10, 10)],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(svg.contains("<clipPath id=\"clip0\">"), "{svg}");
        assert!(svg.contains("clip-path=\"url(#clip0)\""), "{svg}");
        assert!(svg.contains("M 0 0 L 10 0 L 10 10 L 0 10 Z"), "{svg}");
    }

    #[test]
    fn a_shape_content_emits_a_filled_path() {
        let layer = Layer::default();
        let d = doc(10, 10, vec![layer]);
        let options = SvgOptions {
            contents: vec![SvgContent::Shape {
                subpaths: vec![straight_subpath()],
                fill: [10, 20, 30, 255],
                live: None,
            }],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(svg.contains("<path d=\"M 0 0 L 10 0\""), "{svg}");
        assert!(svg.contains("fill=\"#0a141e\""), "{svg}");
    }

    #[test]
    fn a_live_shape_emits_its_parametric_primitive() {
        let layer = Layer::default();
        let d = doc(10, 10, vec![layer]);
        let rect = LiveShape {
            origin_type: crate::ORIGIN_RECTANGLE,
            bounds: (1.0, 2.0, 6.0, 8.0),
            radii: [0.0; 4],
        };
        let options = SvgOptions {
            contents: vec![SvgContent::Shape {
                subpaths: Vec::new(),
                fill: [255, 0, 0, 128],
                live: Some(rect),
            }],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(
            svg.contains("<rect x=\"1\" y=\"2\" width=\"5\" height=\"6\""),
            "{svg}"
        );
        assert!(
            svg.contains("fill=\"#ff0000\" fill-opacity=\"0.502\""),
            "{svg}"
        );

        let ellipse = LiveShape {
            origin_type: crate::ORIGIN_ELLIPSE,
            bounds: (0.0, 0.0, 10.0, 4.0),
            radii: [0.0; 4],
        };
        let options = SvgOptions {
            contents: vec![SvgContent::Shape {
                subpaths: Vec::new(),
                fill: [0, 0, 0, 255],
                live: Some(ellipse),
            }],
            precision: 3,
        };
        let svg = encode_svg(&d, &options);
        assert!(
            svg.contains("<ellipse cx=\"5\" cy=\"2\" rx=\"5\" ry=\"2\""),
            "{svg}"
        );
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
}
