//! Clipping-group dispatch: build the group with the CPU algorithm over the
//! region's intersection with the base/clipped bounds, upload it, and blend it
//! on the GPU with the base's opacity and mode.

use pictura_core::{Document, Layer, PsdRect};

use super::backend::{Gpu, SrcLayout};
use super::effect::pack_word;
use super::NO_ADJ;

impl Gpu {
    pub(super) fn composite_clip_group(
        &self,
        canvas: &wgpu::Buffer,
        base: &Layer,
        clipped: &[&Layer],
        doc: &Document,
    ) {
        let region = self.region();
        let clip = region.rect();
        let Some(bounds) = group_bounds(base, clipped, clip) else {
            return;
        };
        let effect_region = (!self.is_document(doc)).then_some(clip);
        let clipped: Vec<(usize, &Layer)> = clipped
            .iter()
            .enumerate()
            .map(|(i, layer)| (i + 1, *layer))
            .collect();
        let mut members: Vec<&Layer> = Vec::with_capacity(1 + clipped.len());
        members.push(base);
        members.extend(clipped.iter().map(|(_, layer)| *layer));
        let plan = crate::composite_plan::CompositePlan::build_for_layers(
            &members,
            doc,
            clip,
            effect_region,
        );
        let group = crate::composite_clipping::build_sibling_group(
            bounds,
            clip,
            effect_region,
            false,
            0,
            &clipped,
            doc,
            &plan,
            |canvas, _, node, over| {
                crate::composite::composite_layer_inner(canvas, node, doc, None, None, over);
            },
        );
        let w = bounds.width().max(0) as usize;
        let h = bounds.height().max(0) as usize;
        let n = w * h;
        if n == 0 {
            return;
        }
        let words: Vec<u32> = group
            .px
            .iter()
            .map(|p| pack_word([p.r, p.g, p.b], p.a))
            .collect();
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let src = self.upload_with("pictura-clip", n * 4, move |start, mut out| {
            out.copy_from_slice(&bytes[start..start + out.len()]);
        });
        let proxy = Layer {
            is_group: true,
            opacity: base.opacity,
            blend: base.blend,
            fill: 255,
            mask: None,
            vector_mask: None,
            blend_if: None,
            ..Default::default()
        };
        let mask = self.build_mask(&proxy, doc);
        let layout = SrcLayout {
            x0: bounds.left as u32,
            y0: bounds.top as u32,
            w: w as u32,
            h: h as u32,
            gray: false,
            packed: true,
        };
        self.dispatch(
            canvas,
            &src,
            &mask,
            layout,
            base.blend,
            base.opacity,
            255,
            false,
            NO_ADJ,
            None,
        );
    }
}

/// The union of the base and clipped content bounds, grown by each layer's
/// effect reach and clipped to the composite region. A destructive adjustment
/// adds no content; a fill adjustment is bounded by its layer rect.
fn group_bounds(base: &Layer, clipped: &[&Layer], region: PsdRect) -> Option<PsdRect> {
    // A base with no content has zero coverage, so every clipped layer lerps
    // to nothing; skip the group entirely.
    let base_bounds = layer_bounds(base)?;
    let mut bounds = Some(base_bounds);
    for layer in clipped {
        bounds = union(bounds, layer_bounds(layer));
    }
    let bounds = bounds?;
    let clipped = PsdRect {
        top: bounds.top.max(region.top),
        left: bounds.left.max(region.left),
        bottom: bounds.bottom.min(region.bottom),
        right: bounds.right.min(region.right),
    };
    (clipped.width() > 0 && clipped.height() > 0).then_some(clipped)
}

fn layer_bounds(layer: &Layer) -> Option<PsdRect> {
    if layer.is_group {
        let mut bounds = None;
        for child in &layer.children {
            if child.visible {
                bounds = union(bounds, layer_bounds(child));
            }
        }
        return bounds.map(|rect| grow(rect, crate::layer_effects::paint_reach(layer)));
    }
    if layer.adjustment.is_some() && crate::fill::decode_layer_fill(layer).is_none() {
        return None;
    }
    Some(grow(layer.rect, crate::layer_effects::paint_reach(layer)))
}

fn union(a: Option<PsdRect>, b: Option<PsdRect>) -> Option<PsdRect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(PsdRect {
            top: a.top.min(b.top),
            left: a.left.min(b.left),
            bottom: a.bottom.max(b.bottom),
            right: a.right.max(b.right),
        }),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

fn grow(rect: PsdRect, pad: i32) -> PsdRect {
    PsdRect {
        top: rect.top.saturating_sub(pad),
        left: rect.left.saturating_sub(pad),
        bottom: rect.bottom.saturating_add(pad),
        right: rect.right.saturating_add(pad),
    }
}
