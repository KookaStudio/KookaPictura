//! Materialise an effect pass into its own RGBA layer for Create Layers.

use pictura_core::{BlendMode, Document, Layer, PixelBuffer, PsdRect};

use super::planes::{self, EffectPass};
use super::{light, BuildRegion};
use crate::composite::Canvas;

/// One pass turned into a layer-sized buffer. `below` says where it belongs
/// relative to the base layer.
pub(crate) struct MaterializedPass {
    pub(crate) name: &'static str,
    pub(crate) below: bool,
    pub(crate) rect: PsdRect,
    pub(crate) blend: BlendMode,
    pub(crate) buffer: PixelBuffer,
}

/// Materialise every enabled and present effect pass in compositor draw order.
/// The document global light is resolved first, so a globally-lit effect
/// explodes at the same light as the canvas.
pub(crate) fn style_passes(
    layer: &Layer,
    doc: &Document,
    effects: &super::LayerEffects,
) -> Vec<MaterializedPass> {
    let effects = light::effective(effects, doc);
    let build = BuildRegion {
        region: None,
        mask_clip: super::build_mask_clip(None, &effects, doc),
    };
    let on = |enabled: bool, present: bool| enabled && present;
    let mut out = Vec::new();
    let mut push = |name: &'static str, below: bool, passes: Vec<EffectPass>| {
        for pass in passes {
            let rect = pass.rect;
            let width = rect.width().max(0) as usize;
            let height = rect.height().max(0) as usize;
            if width == 0 || height == 0 {
                continue;
            }
            let mode = pass.mode;
            let mut canvas = Canvas::new_region(rect.left, rect.top, width, height);
            planes::apply(&mut canvas, &[pass]);
            out.push(MaterializedPass {
                name,
                below,
                rect,
                blend: mode,
                buffer: canvas.into_pixel_buffer(),
            });
        }
    };
    if let Some(shadow) = effects
        .drop_shadow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Drop Shadow",
            true,
            super::shadows::build_drop_shadow(layer, doc, shadow, build, None),
        );
    }
    if let Some(glow) = effects
        .outer_glow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Outer Glow",
            true,
            super::glows::build_outer_glow(layer, doc, glow, build, None),
        );
    }
    if let Some(overlay) = effects
        .pattern_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Pattern Overlay",
            false,
            super::overlays::build_pattern_overlay(layer, doc, overlay, build, None),
        );
    }
    if let Some(overlay) = effects
        .gradient_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Gradient Overlay",
            false,
            super::overlays::build_gradient_overlay(layer, doc, overlay, build, None),
        );
    }
    if let Some(overlay) = effects
        .color_overlay
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Color Overlay",
            false,
            super::overlays::build_color_overlay(layer, doc, overlay, build, None),
        );
    }
    if let Some(satin) = effects
        .satin
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Satin",
            false,
            super::satin::build_satin(layer, doc, satin, build, None),
        );
    }
    if let Some(glow) = effects
        .inner_glow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Inner Glow",
            false,
            super::glows::build_inner_glow(layer, doc, glow, build, None),
        );
    }
    if let Some(shadow) = effects
        .inner_shadow
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Inner Shadow",
            false,
            super::shadows::build_inner_shadow(layer, doc, shadow, build, None),
        );
    }
    if let Some(stroke) = effects
        .stroke
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Stroke",
            false,
            super::strokes::build_stroke(layer, doc, stroke, build, None),
        );
    }
    if let Some(bevel) = effects
        .bevel
        .as_ref()
        .filter(|effect| on(effect.enabled, effect.present))
    {
        push(
            "Bevel & Emboss",
            false,
            super::bevel::build_bevel_emboss(layer, doc, bevel, build, None),
        );
    }
    out
}
