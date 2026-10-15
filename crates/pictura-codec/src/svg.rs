//! SVG export: serialize a document to Photoshop-CC-shaped SVG text.
//!
//! Photoshop's SVG export is not a traced raster. It walks the document's
//! vector content and emits SVG primitives, embedding raster and effect content
//! as a base64 PNG fallback. The header and the raster fallback shape are taken
//! from a shipped CC sample; the primitives are inferred and marked below.
//!
//! This module is Qt-free and does no compositing or fill decoding: the caller
//! supplies the document as a tree of [`SvgNode`]s — a decoded shape (outline,
//! fill, live form), a raster fallback as RGBA8888 (encoded here to a base64
//! PNG), or a group of nodes. `pictura-render` builds that tree
//! (`pictura_render::svg_nodes`), because it owns the `vscg` fill decode and the
//! compositor, and it already depends on this crate.

use pictura_core::path::Subpath;
use pictura_core::{BlendMode, Document, VectorFillRule, VectorMask};

use crate::LiveShape;

/// One emitted element with the opacity and blend of the layer (or flattened
/// run of layers) it stands for. The tree is bottom-first, like
/// `Document::layers`.
#[derive(Clone, Debug)]
pub struct SvgNode {
    pub content: SvgContent,
    pub opacity: u8,
    pub blend: BlendMode,
    /// A vector mask clipping a raster fallback.
    pub clip: Option<VectorMask>,
}

impl SvgNode {
    /// `content` at full opacity, Normal blend, unclipped.
    pub fn plain(content: SvgContent) -> Self {
        Self {
            content,
            opacity: 255,
            blend: BlendMode::Normal,
            clip: None,
        }
    }
}

/// The content of an [`SvgNode`], decoded by the caller.
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
    /// A layer group. An `isolated` group composites its children on
    /// transparency first (any non-pass-through group); a pass-through group
    /// lets its children blend with what lies beneath it.
    Group {
        children: Vec<SvgNode>,
        isolated: bool,
    },
}

/// SVG export options.
#[derive(Clone, Debug)]
pub struct SvgOptions {
    /// The document's top-level nodes, bottom-first; see [`SvgNode`].
    pub nodes: Vec<SvgNode>,
    /// Decimal places for coordinates, clamped to `0..=6`.
    pub precision: u8,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            precision: 3,
        }
    }
}

/// Serialize `doc` to SVG text. The document supplies only the canvas size;
/// `options.nodes` supplies the content.
pub fn encode_svg(doc: &Document, options: &SvgOptions) -> String {
    let mut writer = Writer {
        precision: options.precision.min(6),
        defs: Vec::new(),
        clips: 0,
    };
    let body: Vec<String> = options
        .nodes
        .iter()
        .filter_map(|node| writer.node(node))
        .collect();

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" preserveAspectRatio=\"xMidYMid\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        doc.width, doc.height, doc.width, doc.height
    ));
    if !writer.defs.is_empty() {
        out.push_str("  <defs>\n");
        out.push_str(&writer.defs.join("\n"));
        out.push_str("\n  </defs>\n");
    }
    for element in body {
        out.push_str(&indent_lines(&element, 1));
        out.push('\n');
    }
    out.push_str("</svg>\n");
    out
}

/// The tree walk's state: clip paths collect into one `<defs>` with
/// document-unique ids.
struct Writer {
    precision: u8,
    defs: Vec<String>,
    clips: usize,
}

impl Writer {
    /// The element for `node`, `None` when it draws nothing.
    fn node(&mut self, node: &SvgNode) -> Option<String> {
        if node.opacity == 0 {
            return None;
        }
        let attrs = group_attrs(node.opacity, node.blend);
        match &node.content {
            SvgContent::Shape {
                subpaths,
                fill,
                live,
            } => {
                let element = shape_element(subpaths, *fill, live.as_ref(), self.precision)?;
                Some(wrap(&element, &attrs))
            }
            SvgContent::Raster {
                rgba,
                width,
                height,
            } => {
                let href = crate::encode_png_base64(rgba, *width, *height)?;
                let clip = self.clip_path(node.clip.as_ref());
                let element = image_element(&href, *width, *height, clip.as_deref());
                Some(wrap(&element, &attrs))
            }
            SvgContent::Group { children, isolated } => {
                let inner: Vec<String> = children.iter().filter_map(|c| self.node(c)).collect();
                if inner.is_empty() {
                    return None;
                }
                let mut attrs = attrs;
                // Opacity and a blend already isolate; a bare `<g>` does not.
                if *isolated && attrs.is_empty() {
                    attrs.push("style=\"isolation:isolate\"".to_string());
                }
                let open = if attrs.is_empty() {
                    "<g>".to_string()
                } else {
                    format!("<g {}>", attrs.join(" "))
                };
                Some(format!(
                    "{open}\n{}\n</g>",
                    indent_lines(&inner.join("\n"), 1)
                ))
            }
        }
    }

    /// Register a `<clipPath>` for a vector mask and return its id, `None`
    /// when the mask is absent, disabled, or empty.
    fn clip_path(&mut self, mask: Option<&VectorMask>) -> Option<String> {
        let mask = mask?;
        if mask.disabled || mask.subpaths.is_empty() {
            return None;
        }
        let data = vector_mask_data(mask, self.precision);
        if data.is_empty() {
            return None;
        }
        let id = format!("clip{}", self.clips);
        self.clips += 1;
        self.defs.push(format!(
            "  <clipPath id=\"{id}\">\n    <path d=\"{}\"/>\n  </clipPath>",
            escape_xml(&data)
        ));
        Some(id)
    }
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

/// The `<g>` attributes for a non-default opacity or a CSS-representable blend.
fn group_attrs(opacity: u8, blend: BlendMode) -> Vec<String> {
    let mut attrs = Vec::new();
    if opacity != 255 {
        attrs.push(format!(
            "opacity=\"{}\"",
            format_coord(f64::from(opacity) / 255.0, 3)
        ));
    }
    if let Some(css) = blend_css(blend) {
        attrs.push(format!("style=\"mix-blend-mode:{css}\""));
    }
    attrs
}

/// Wrap `element` in a `<g>` carrying `attrs`, or return it as is without any.
fn wrap(element: &str, attrs: &[String]) -> String {
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

    #[test]
    fn a_shape_content_emits_a_filled_path() {
        let shape = SvgContent::Shape {
            subpaths: vec![straight_subpath()],
            fill: [10, 20, 30, 255],
            live: None,
        };
        let svg = svg(10, 10, vec![SvgNode::plain(shape)]);
        assert!(svg.contains("<path d=\"M 0 0 L 10 0\""), "{svg}");
        assert!(svg.contains("fill=\"#0a141e\""), "{svg}");
    }

    #[test]
    fn a_live_shape_emits_its_parametric_primitive() {
        let shape = |fill, live| {
            SvgNode::plain(SvgContent::Shape {
                subpaths: Vec::new(),
                fill,
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
}
