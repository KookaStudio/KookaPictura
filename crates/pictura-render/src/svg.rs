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

use pictura_codec::{encode_svg, SvgContent, SvgNode, SvgOptions};
use pictura_core::{BlendMode, Document, Layer};

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

/// A non-group layer: a filled primitive when it is a plain solid shape, else
/// the layer's own raster.
fn leaf_node(doc: &Document, layer: &Layer, raster: &mut Rasterize) -> SvgNode {
    let shape = (layer.mask.is_none() && !crate::composite::has_effect_block(layer))
        .then(|| crate::shape_fill_color(layer))
        .flatten()
        .zip(crate::layer_shape_paths(layer, doc.width, doc.height));
    if let Some((mut fill, subpaths)) = shape {
        fill[3] = ((u16::from(fill[3]) * u16::from(layer.fill) + 127) / 255) as u8;
        return styled(
            layer,
            SvgContent::Shape {
                subpaths,
                fill,
                live: crate::layer_live_shape(layer),
            },
        );
    }
    SvgNode {
        clip: layer.vector_mask.clone(),
        ..styled(layer, raster_content(doc, &[normalized(layer)], raster))
    }
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
