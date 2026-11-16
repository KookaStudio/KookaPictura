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
    /// A shape layer: its outline in document pixels, its paint, and the
    /// live-shape form when the layer is still parametric.
    Shape {
        subpaths: Vec<Subpath>,
        paint: SvgPaint,
        /// The layer's Fill opacity, `0..=255`, scaling the paint.
        fill_opacity: u8,
        stroke: Option<SvgStroke>,
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

/// A shape's fill paint.
#[derive(Clone, Debug)]
pub enum SvgPaint {
    /// A straight RGBA solid colour.
    Solid([u8; 4]),
    /// A linear or radial gradient in document user space.
    Gradient(SvgGradient),
    /// A tiled pattern in document user space.
    Pattern(SvgPattern),
}

/// A gradient paint server; every coordinate is in document user space.
#[derive(Clone, Debug)]
pub enum SvgGradient {
    Linear {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stops: Vec<SvgGradientStop>,
    },
    Radial {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        stops: Vec<SvgGradientStop>,
    },
}

/// One gradient colour stop; `offset` is `0..=1`.
#[derive(Clone, Debug)]
pub struct SvgGradientStop {
    pub offset: f64,
    pub rgba: [u8; 4],
}

/// A pattern paint server: its scaled tile as straight-alpha RGBA8888 and its
/// user-space origin.
#[derive(Clone, Debug)]
pub struct SvgPattern {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub x: f64,
    pub y: f64,
}

/// Where a stroke sits on the outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvgStrokePosition {
    Center,
    Inside,
    Outside,
}

/// A solid vector stroke.
#[derive(Clone, Copy, Debug)]
pub struct SvgStroke {
    pub rgb: [u8; 3],
    pub width: f64,
    pub opacity: f64,
    pub position: SvgStrokePosition,
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
        paints: 0,
        masks: 0,
        width: doc.width,
        height: doc.height,
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
    paints: usize,
    masks: usize,
    width: u32,
    height: u32,
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
                paint,
                fill_opacity,
                stroke,
                live,
            } => {
                let element = self.shape(
                    subpaths,
                    paint,
                    *fill_opacity,
                    stroke.as_ref(),
                    live.as_ref(),
                )?;
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

    /// The element(s) for a shape: its fill primitive, plus a separate
    /// stroke-only element when an inside/outside stroke needs a mask.
    fn shape(
        &mut self,
        subpaths: &[Subpath],
        paint: &SvgPaint,
        fill_opacity: u8,
        stroke: Option<&SvgStroke>,
        live: Option<&LiveShape>,
    ) -> Option<String> {
        let base = shape_base(subpaths, live, self.precision)?;
        let mut fill_attrs = self.paint_attrs(paint, fill_opacity)?;
        let stroke = stroke.map(|stroke| self.stroke_attrs(stroke, subpaths));
        match stroke {
            Some((stroke_attrs, true)) => {
                let fill_element = with_attrs(&base, &fill_attrs);
                let mut stroke_attrs = stroke_attrs;
                stroke_attrs.insert(0, "fill=\"none\"".to_string());
                let stroke_element = with_attrs(&base, &stroke_attrs);
                Some(format!("{fill_element}\n{stroke_element}"))
            }
            Some((stroke_attrs, false)) => {
                fill_attrs.extend(stroke_attrs);
                Some(with_attrs(&base, &fill_attrs))
            }
            None => Some(with_attrs(&base, &fill_attrs)),
        }
    }

    /// The fill attributes for `paint`, registering a paint server when needed.
    fn paint_attrs(&mut self, paint: &SvgPaint, fill_opacity: u8) -> Option<Vec<String>> {
        match paint {
            SvgPaint::Solid(fill) => {
                let alpha = (u16::from(fill[3]) * u16::from(fill_opacity) + 127) / 255;
                let mut attrs = vec![format!(
                    "fill=\"#{:02x}{:02x}{:02x}\"",
                    fill[0], fill[1], fill[2]
                )];
                if alpha != 255 {
                    attrs.push(format!(
                        "fill-opacity=\"{}\"",
                        format_coord(f64::from(alpha) / 255.0, 3)
                    ));
                }
                Some(attrs)
            }
            SvgPaint::Gradient(gradient) => {
                let id = self.gradient_def(gradient);
                Some(self.paint_ref_attrs(&id, fill_opacity))
            }
            SvgPaint::Pattern(pattern) => {
                let id = self.pattern_def(pattern)?;
                Some(self.paint_ref_attrs(&id, fill_opacity))
            }
        }
    }

    /// A `fill="url(#…)"` attribute list, with the fill opacity when not full.
    fn paint_ref_attrs(&self, id: &str, fill_opacity: u8) -> Vec<String> {
        let mut attrs = vec![format!("fill=\"url(#{id})\"")];
        if fill_opacity != 255 {
            attrs.push(format!(
                "fill-opacity=\"{}\"",
                format_coord(f64::from(fill_opacity) / 255.0, 3)
            ));
        }
        attrs
    }

    /// Register a gradient paint server and return its id.
    fn gradient_def(&mut self, gradient: &SvgGradient) -> String {
        let id = format!("grad{}", self.paints);
        self.paints += 1;
        let (open, stops) = match gradient {
            SvgGradient::Linear {
                x1,
                y1,
                x2,
                y2,
                stops,
            } => (
                format!(
                    "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">",
                    format_coord(*x1, self.precision),
                    format_coord(*y1, self.precision),
                    format_coord(*x2, self.precision),
                    format_coord(*y2, self.precision)
                ),
                stops,
            ),
            SvgGradient::Radial {
                cx,
                cy,
                rx,
                ry,
                stops,
            } => (
                format!(
                    "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"0\" cy=\"0\" r=\"1\" gradientTransform=\"translate({} {}) scale({} {})\">",
                    format_coord(*cx, self.precision),
                    format_coord(*cy, self.precision),
                    format_coord(*rx, self.precision),
                    format_coord(*ry, self.precision)
                ),
                stops,
            ),
        };
        let close = if matches!(gradient, SvgGradient::Linear { .. }) {
            "</linearGradient>"
        } else {
            "</radialGradient>"
        };
        let body: Vec<String> = stops.iter().map(gradient_stop).collect();
        self.defs
            .push(format!("  {open}\n{}\n  {close}", body.join("\n")));
        id
    }

    /// Register a pattern paint server and return its id.
    fn pattern_def(&mut self, pattern: &SvgPattern) -> Option<String> {
        let href = crate::encode_png_base64(&pattern.rgba, pattern.width, pattern.height)?;
        let id = format!("pat{}", self.paints);
        self.paints += 1;
        let x = format_coord(pattern.x, self.precision);
        let y = format_coord(pattern.y, self.precision);
        let (w, h) = (pattern.width, pattern.height);
        self.defs.push(format!(
            "  <pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" patternContentUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\">\n    <image xlink:href=\"data:img/png;base64,{}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/>\n  </pattern>",
            escape_xml(&href)
        ));
        Some(id)
    }

    /// The stroke attributes and whether they use an inside/outside mask.
    fn stroke_attrs(&mut self, stroke: &SvgStroke, subpaths: &[Subpath]) -> (Vec<String>, bool) {
        let mut attrs = vec![format!(
            "stroke=\"#{:02x}{:02x}{:02x}\"",
            stroke.rgb[0], stroke.rgb[1], stroke.rgb[2]
        )];
        let mask = self.stroke_mask(stroke.position, subpaths);
        let masked = mask.is_some();
        if let Some(id) = mask {
            attrs.push(format!("mask=\"url(#{id})\""));
        }
        let width = if masked {
            stroke.width * 2.0
        } else {
            stroke.width
        };
        attrs.push(format!(
            "stroke-width=\"{}\"",
            format_coord(width, self.precision)
        ));
        if stroke.opacity < 1.0 {
            attrs.push(format!(
                "stroke-opacity=\"{}\"",
                format_coord(stroke.opacity.clamp(0.0, 1.0), 3)
            ));
        }
        (attrs, masked)
    }

    /// Register a mask that keeps only the interior or exterior of `subpaths`.
    fn stroke_mask(&mut self, position: SvgStrokePosition, subpaths: &[Subpath]) -> Option<String> {
        if position == SvgStrokePosition::Center {
            return None;
        }
        let data = path_data(subpaths, self.precision);
        if data.is_empty() {
            return None;
        }
        let id = format!("mask{}", self.masks);
        self.masks += 1;
        let (backdrop, shape) = match position {
            SvgStrokePosition::Inside => ("black", "white"),
            SvgStrokePosition::Outside => ("white", "black"),
            SvgStrokePosition::Center => unreachable!(),
        };
        let (w, h) = (self.width, self.height);
        self.defs.push(format!(
            "  <mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" maskContentUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\">\n    <rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{backdrop}\"/>\n    <path d=\"{}\" fill=\"{shape}\" fill-rule=\"evenodd\"/>\n  </mask>",
            escape_xml(&data)
        ));
        Some(id)
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
fn shape_base(subpaths: &[Subpath], live: Option<&LiveShape>, precision: u8) -> Option<String> {
    Some(match live {
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
    })
}

/// One `<stop>` element.
fn gradient_stop(stop: &SvgGradientStop) -> String {
    let mut attrs = format!(
        "offset=\"{}\" stop-color=\"#{:02x}{:02x}{:02x}\"",
        format_coord(stop.offset.clamp(0.0, 1.0), 4),
        stop.rgba[0],
        stop.rgba[1],
        stop.rgba[2]
    );
    if stop.rgba[3] != 255 {
        attrs.push_str(&format!(
            " stop-opacity=\"{}\"",
            format_coord(f64::from(stop.rgba[3]) / 255.0, 3)
        ));
    }
    format!("    <stop {attrs}/>")
}

/// Insert `attrs` into a self-closing primitive.
fn with_attrs(element: &str, attrs: &[String]) -> String {
    if attrs.is_empty() {
        return element.to_string();
    }
    match element.strip_suffix("/>") {
        Some(head) => format!("{head} {}/>", attrs.join(" ")),
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
mod tests;
