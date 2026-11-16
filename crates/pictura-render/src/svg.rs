//! Map a document to the [`SvgNode`] tree the SVG exporter serializes.
//!
//! A shape layer (a `vmsk` outline plus a solid `vscg` fill, or an app-authored
//! adjustment fill) is emitted as a filled primitive, a group as a nested `<g>`,
//! and every other layer as a raster fallback. Content SVG cannot express —
//! an adjustment layer, a blend mode with no CSS form, a clipping stack that is
//! not plain source-over — flattens with everything beneath it in its container
//! into one image, so the output stays faithful at the cost of editability. The
//! actual SVG text is built by `pictura_codec::svg`; this module only decodes
//! the engine views the codec cannot reach.

use pictura_adjust::{Adjustment, GradientFillParams, GradientKind, PatternFillParams};
use pictura_codec::{
    encode_svg, SvgContent, SvgGradient, SvgGradientStop, SvgNode, SvgOptions, SvgPaint,
    SvgPattern, SvgStroke, SvgStrokePosition,
};
use pictura_core::{BlendMode, Document, Layer, PsdRect};

/// Composites a scratch document (the source document's settings with only
/// the given layers) to straight-alpha sRGB RGBA8888 at the document size.
type Rasterize<'a> = dyn FnMut(&Document) -> Vec<u8> + 'a;

/// Serialize `doc` to SVG text; `raster` composites a scratch document as in
/// [`svg_nodes`].
pub fn encode_document_svg(doc: &Document, raster: impl FnMut(&Document) -> Vec<u8>) -> String {
    encode_svg(
        doc,
        &SvgOptions {
            nodes: svg_nodes(doc, raster),
            precision: 3,
        },
    )
}

/// The SVG node tree for `doc`. `raster` composites a scratch document — `doc`
/// with its layers replaced — to straight-alpha sRGB RGBA8888
/// (`width * height * 4` bytes); it is called once per raster fallback.
pub fn svg_nodes(doc: &Document, mut raster: impl FnMut(&Document) -> Vec<u8>) -> Vec<SvgNode> {
    // The document is an isolated container, so the walk always succeeds.
    walk(doc, &doc.layers, true, &mut raster).unwrap_or_default()
}

/// How one stack (a layer plus the layers clipped to it) is emitted.
enum Plan {
    Skip,
    Node(Box<SvgNode>),
    /// The stack reads the content beneath it: flatten the container up to and
    /// including it.
    Backdrop,
}

/// The nodes for one container's `layers`. `None` when a stack needs the
/// backdrop and the container is a pass-through group, whose backdrop lies
/// outside it; the caller then flattens at its own level.
fn walk(
    doc: &Document,
    layers: &[Layer],
    isolated: bool,
    raster: &mut Rasterize,
) -> Option<Vec<SvgNode>> {
    let mut nodes = Vec::new();
    let mut start = 0;
    while start < layers.len() {
        let mut end = start + 1;
        while end < layers.len() && layers[end].clipping {
            end += 1;
        }
        let plan = plan_stack(doc, &layers[start..end], raster);
        start = end;
        match plan {
            Plan::Skip => {}
            Plan::Node(node) => nodes.push(*node),
            Plan::Backdrop if isolated => {
                let content = raster_content(doc, &layers[..end], raster);
                nodes = vec![SvgNode::plain(content)];
            }
            Plan::Backdrop => return None,
        }
    }
    Some(nodes)
}

/// The plan for `stack`: its base layer, then the layers clipped to it.
fn plan_stack(doc: &Document, stack: &[Layer], raster: &mut Rasterize) -> Plan {
    let base = &stack[0];
    if !base.visible || base.opacity == 0 {
        return Plan::Skip;
    }
    if is_adjustment_layer(base) || !svg_blend(base.blend) {
        return Plan::Backdrop;
    }
    if stack.len() > 1 {
        // A clipping stack composited alone equals its in-place result only
        // under plain source-over; anything else reads the backdrop.
        let plain = base.opacity == 255
            && base.blend == BlendMode::Normal
            && stack[1..].iter().all(|l| l.blend == BlendMode::Normal);
        if !plain {
            return Plan::Backdrop;
        }
        return Plan::Node(Box::new(SvgNode::plain(raster_content(doc, stack, raster))));
    }
    if base.is_group {
        return plan_group(doc, base, raster);
    }
    Plan::Node(Box::new(leaf_node(doc, base, raster)))
}

fn plan_group(doc: &Document, group: &Layer, raster: &mut Rasterize) -> Plan {
    let masked = group.mask.is_some() || group.vector_mask.is_some();
    if group.blend == BlendMode::PassThrough {
        // ponytail: a faded or masked pass-through group has no SVG form (a
        // `<g opacity>` isolates its children), so it flattens.
        if group.opacity != 255 || masked {
            return Plan::Backdrop;
        }
        return match walk(doc, &group.children, false, raster) {
            Some(children) => Plan::Node(Box::new(SvgNode::plain(SvgContent::Group {
                children,
                isolated: false,
            }))),
            None => Plan::Backdrop,
        };
    }
    let content = if masked {
        raster_content(doc, &[normalized(group)], raster)
    } else {
        SvgContent::Group {
            children: walk(doc, &group.children, true, raster).unwrap_or_default(),
            isolated: true,
        }
    };
    Plan::Node(Box::new(styled(group, content)))
}

/// A non-group layer: a filled primitive when its fill and effects are
/// expressible as SVG, else the layer's own raster.
fn leaf_node(doc: &Document, layer: &Layer, raster: &mut Rasterize) -> SvgNode {
    if let Some(content) = shape_content(doc, layer) {
        return styled(layer, content);
    }
    SvgNode {
        clip: layer.vector_mask.clone(),
        ..styled(layer, raster_content(doc, &[normalized(layer)], raster))
    }
}

/// The vector primitive for `layer`, or `None` when it needs the raster
/// fallback.
fn shape_content(doc: &Document, layer: &Layer) -> Option<SvgContent> {
    if layer.mask.as_ref().is_some_and(|mask| !mask_is_noop(mask)) {
        return None;
    }
    let fill = crate::fill::decode_layer_fill(layer)?;
    let subpaths = match crate::layer_shape_paths(layer, doc.width, doc.height) {
        Some(subpaths) => subpaths,
        None => match &fill {
            Adjustment::GradientFill(_) | Adjustment::PatternFill(_)
                if crate::is_fill_content_layer(layer) =>
            {
                vec![rect_outline(layer.rect)?]
            }
            _ => return None,
        },
    };
    let paint = fill_paint(&fill, layer.rect, doc)?;
    let stroke = match effects_plan(layer) {
        EffectsPlan::None => None,
        EffectsPlan::Stroke(stroke) => Some(stroke),
        EffectsPlan::Raster => return None,
    };
    Some(SvgContent::Shape {
        subpaths,
        paint,
        fill_opacity: layer.fill,
        stroke,
        live: crate::layer_live_shape(layer),
    })
}

/// The fill paint for a decoded fill, or `None` when SVG cannot express it.
fn fill_paint(fill: &Adjustment, rect: PsdRect, doc: &Document) -> Option<SvgPaint> {
    match fill {
        Adjustment::SolidFill(rgba) => Some(SvgPaint::Solid(*rgba)),
        Adjustment::GradientFill(params) => gradient_paint(params, rect),
        Adjustment::PatternFill(params) => {
            Some(SvgPaint::Pattern(pattern_paint(doc, params, rect)))
        }
        _ => None,
    }
}

/// Whether a layer mask leaves the layer fully visible: disabled, or a full
/// white mask at full density with no feather. The app gives every new fill
/// layer such a mask, so treating it as a real mask would rasterize the
/// gradient and pattern fills the SVG writer can express.
fn mask_is_noop(mask: &pictura_core::LayerMask) -> bool {
    if mask.disabled {
        return true;
    }
    if mask.density != 255 || mask.feather() > 0.0 || mask.default_color != 255 {
        return false;
    }
    mask.data
        .as_ref()
        .is_none_or(|data| data.iter().all(|&v| v == 255))
}

/// A full-rectangle outline for a fill layer with no vector mask.
fn rect_outline(rect: PsdRect) -> Option<pictura_core::path::Subpath> {
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    pictura_core::shape::outline_in_box(
        pictura_core::shape::ShapeOptions::new(pictura_core::shape::ShapeKind::Rectangle, 0.0, 3),
        (
            f64::from(rect.left),
            f64::from(rect.top),
            f64::from(rect.width()),
            f64::from(rect.height()),
        ),
    )
}

/// Map a gradient fill to an SVG paint server, or a solid colour for a
/// degenerate gradient, or `None` for a kind SVG cannot represent.
fn gradient_paint(params: &GradientFillParams, rect: PsdRect) -> Option<SvgPaint> {
    let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let angle = f64::from(params.angle_deg);
    let ratio = angle.rem_euclid(90.0);
    let s = (f64::from(params.scale) / 100.0) * ((90.0 - ratio) / 90.0 * w + (ratio / 90.0) * h);
    if !s.is_finite() || s <= 0.0 {
        return None;
    }
    let theta = angle.rem_euclid(360.0).to_radians();
    let (sin_t, cos_t) = (theta.sin(), theta.cos());
    let ax = if w > 1.0 {
        2.0 * w / s / (w - 1.0)
    } else {
        0.0
    };
    let ay = if h > 1.0 {
        2.0 * h / s / (h - 1.0)
    } else {
        0.0
    };
    let bx = -w / s;
    let by = -h / s;
    let stops = gradient_stops(params);
    match params.kind {
        GradientKind::Linear => {
            let gx = 0.5 * cos_t * ax;
            let gy = -0.5 * sin_t * ay;
            let c = 0.5 * (cos_t * bx - sin_t * by + 1.0);
            let g2 = gx * gx + gy * gy;
            if g2 <= f64::EPSILON {
                return Some(SvgPaint::Solid(crate::fill::sample_stops(
                    &params.stops,
                    c.clamp(0.0, 1.0),
                )));
            }
            // The compositor samples the gradient at integer pixel origins;
            // SVG rasterizers sample at pixel centers, so shift by half a pixel.
            let c = c - 0.5 * (gx + gy);
            let p1 = (-c / g2 * gx, -c / g2 * gy);
            let p2 = (p1.0 + gx / g2, p1.1 + gy / g2);
            let (dx, dy) = (f64::from(rect.left), f64::from(rect.top));
            Some(SvgPaint::Gradient(SvgGradient::Linear {
                x1: p1.0 + dx,
                y1: p1.1 + dy,
                x2: p2.0 + dx,
                y2: p2.1 + dy,
                stops,
            }))
        }
        GradientKind::Radial => {
            if ax.abs() <= f64::EPSILON || ay.abs() <= f64::EPSILON {
                return None;
            }
            // Pixel centers: the compositor samples at integer origins.
            let cx = -bx / ax + f64::from(rect.left) + 0.5;
            let cy = -by / ay + f64::from(rect.top) + 0.5;
            let rx = 1.0 / ax.abs();
            let ry = 1.0 / ay.abs();
            if !cx.is_finite() || !cy.is_finite() || !rx.is_finite() || !ry.is_finite() {
                return None;
            }
            Some(SvgPaint::Gradient(SvgGradient::Radial {
                cx,
                cy,
                rx,
                ry,
                stops,
            }))
        }
        _ => None,
    }
}

/// The gradient stops in SVG order, reversed when the PSD fill is reversed.
fn gradient_stops(params: &GradientFillParams) -> Vec<SvgGradientStop> {
    let mut stops: Vec<SvgGradientStop> = params
        .stops
        .iter()
        .map(|stop| SvgGradientStop {
            offset: f64::from(stop.location) / 4096.0,
            rgba: [stop.color[0], stop.color[1], stop.color[2], 255],
        })
        .collect();
    if params.reverse {
        stops.reverse();
        for stop in &mut stops {
            stop.offset = 1.0 - stop.offset;
        }
    }
    stops
}

const PATTERN_PLACEHOLDER: [u8; 4] = [128, 128, 128, 255];

/// Map a pattern fill to a nearest-neighbour scaled tile and its user-space
/// origin, using the compositor's placeholder when the pattern is missing.
fn pattern_paint(doc: &Document, params: &PatternFillParams, rect: PsdRect) -> SvgPattern {
    let patterns = pictura_codec::decode_patterns(doc);
    let pattern = patterns
        .iter()
        .find(|pattern| pattern.pattern_id == params.pattern_id)
        .filter(|pattern| {
            pattern.width > 0
                && pattern.height > 0
                && pattern.rgba.len() >= pattern.width as usize * pattern.height as usize * 4
        });
    let (pw, ph, rgba) = match pattern {
        Some(pattern) => (
            i64::from(pattern.width),
            i64::from(pattern.height),
            pattern.rgba.as_slice(),
        ),
        None => (1, 1, PATTERN_PLACEHOLDER.as_slice()),
    };
    let scale = if params.scale.is_finite() {
        f64::from(params.scale)
    } else {
        100.0
    };
    // ponytail: cap the embedded tile; a huge scale would otherwise allocate a
    // multi-gigabyte PNG. Normal pattern scales are far below this.
    const MAX_TILE: f64 = 4096.0;
    let tw = (pw as f64 * scale / 100.0).round().clamp(1.0, MAX_TILE) as u32;
    let th = (ph as f64 * scale / 100.0).round().clamp(1.0, MAX_TILE) as u32;
    let mut tile = Vec::with_capacity((tw * th * 4) as usize);
    for ty in 0..th {
        let py = ((f64::from(ty) / f64::from(th)) * ph as f64).floor() as i64;
        let py = py.clamp(0, ph - 1) as usize;
        for tx in 0..tw {
            let px = ((f64::from(tx) / f64::from(tw)) * pw as f64).floor() as i64;
            let px = px.clamp(0, pw - 1) as usize;
            let i = (py * pw as usize + px) * 4;
            tile.extend_from_slice(&rgba[i..i + 4]);
        }
    }
    let x = if params.link_with_layer {
        f64::from(rect.left)
    } else {
        0.0
    } + f64::from(params.origin.0);
    let y = if params.link_with_layer {
        f64::from(rect.top)
    } else {
        0.0
    } + f64::from(params.origin.1);
    SvgPattern {
        rgba: tile,
        width: tw,
        height: th,
        x,
        y,
    }
}

/// How a layer's effects map to SVG.
enum EffectsPlan {
    None,
    Stroke(SvgStroke),
    Raster,
}

/// A layer is stroke-vectorizable only when a solid, Normal stroke is its sole
/// active effect.
fn effects_plan(layer: &Layer) -> EffectsPlan {
    use crate::layer_effects::{decode_layer_effects, StrokeFill, StrokePosition};

    let effects = decode_layer_effects(layer);
    let on = |enabled: bool, present: bool| enabled && present;
    let other_active = effects
        .drop_shadow
        .as_ref()
        .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .outer_glow
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .inner_shadow
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .inner_glow
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .bevel
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .satin
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .color_overlay
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .gradient_overlay
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present))
        || effects
            .pattern_overlay
            .as_ref()
            .is_some_and(|e| on(e.enabled, e.present));
    if other_active {
        return EffectsPlan::Raster;
    }
    let Some(stroke) = effects.stroke else {
        return EffectsPlan::None;
    };
    if !on(stroke.enabled, stroke.present) {
        return EffectsPlan::None;
    }
    let StrokeFill::Solid(rgb) = stroke.fill else {
        return EffectsPlan::Raster;
    };
    if stroke.blend_mode != BlendMode::Normal {
        return EffectsPlan::Raster;
    }
    EffectsPlan::Stroke(SvgStroke {
        rgb,
        width: f64::from(stroke.size),
        opacity: f64::from(stroke.opacity) / 100.0,
        position: match stroke.position {
            StrokePosition::Inside => SvgStrokePosition::Inside,
            StrokePosition::Center => SvgStrokePosition::Center,
            StrokePosition::Outside => SvgStrokePosition::Outside,
        },
    })
}

/// `content` carrying `layer`'s opacity and (CSS) blend.
fn styled(layer: &Layer, content: SvgContent) -> SvgNode {
    SvgNode {
        content,
        opacity: layer.opacity,
        blend: layer.blend,
        clip: None,
    }
}

/// `layer` at full opacity and Normal blend, so its raster does not bake in
/// what its node's `<g>` applies.
fn normalized(layer: &Layer) -> Layer {
    Layer {
        opacity: 255,
        blend: BlendMode::Normal,
        ..layer.clone()
    }
}

fn raster_content(doc: &Document, layers: &[Layer], raster: &mut Rasterize) -> SvgContent {
    let mut scratch = doc.clone();
    scratch.layers = pictura_core::LayerStack::from_layers(layers.to_vec());
    SvgContent::Raster {
        rgba: raster(&scratch),
        width: doc.width,
        height: doc.height,
    }
}

/// A true adjustment layer, which recolors what lies beneath it. Fill layers
/// also carry an adjustment block but paint their own pixels.
fn is_adjustment_layer(layer: &Layer) -> bool {
    layer.adjustment.is_some() && !crate::is_fill_content_layer(layer)
}

/// Whether SVG can express `blend`: Normal, pass-through (handled by the group
/// walk), or a CSS `mix-blend-mode`.
fn svg_blend(blend: BlendMode) -> bool {
    matches!(blend, BlendMode::Normal | BlendMode::PassThrough)
        || pictura_codec::blend_css(blend).is_some()
}

#[cfg(test)]
mod tests;
