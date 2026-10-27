//! The Rasterize subset (design D8).
//!
//! Fill content is a layer whose opaque adjustment block decodes through
//! [`crate::decode_adjustment`] to [`Adjustment::SolidFill`] (a `SoCo` block, in
//! either the 4-byte in-house form or the standard PSD descriptor),
//! [`Adjustment::GradientFill`] (a `GdFl` descriptor), or
//! [`Adjustment::PatternFill`] (a `PtFl` descriptor). Type/Shape/Vector
//! Mask/Smart Object/Video/3D layers do not exist in the model, so those
//! commands refuse without mutating.

use pictura_adjust::{Adjustment, GradientFillParams, PatternFillParams};
use pictura_codec::PatternPixels;
use pictura_core::{Channel, Document, Layer, PixelBuffer, PsdRect};

use super::paths::{flatten_rows, resolve_path_mut};
use super::shape_layer::is_shape_layer;

/// A decodable fill-content layer: a `SoCo`/`GdFl`/`PtFl` block whose payload
/// decodes to a fill [`Adjustment`]. Routing both this predicate and the bakers
/// through [`crate::decode_adjustment`] keeps them from disagreeing.
pub fn is_fill_content_layer(layer: &Layer) -> bool {
    matches!(
        layer.adjustment.as_ref().and_then(crate::decode_adjustment),
        Some(Adjustment::SolidFill(_) | Adjustment::GradientFill(_) | Adjustment::PatternFill(_))
    )
}

/// Rasterize the fill content at `path`: bake the decoded fill into the layer's
/// `0/1/2/-1` channels over its rect, clear the adjustment, and keep the name,
/// rect, blend, opacity, fill, and mask. Returns false (no mutation) when
/// `path` does not resolve to a decodable fill-content layer.
pub fn rasterize_fill_content(doc: &mut Document, path: &str) -> bool {
    // Decode the pattern library once, before taking the mutable layer borrow.
    let patterns = pictura_codec::decode_patterns(doc);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(adjustment) = layer.adjustment.as_ref().and_then(crate::decode_adjustment) else {
        return false;
    };
    match adjustment {
        Adjustment::SolidFill(rgba) => bake_solid(layer, rgba),
        Adjustment::GradientFill(params) => bake_gradient(layer, &params),
        Adjustment::PatternFill(params) => bake_pattern(layer, &params, &patterns),
        _ => return false,
    }
    layer.adjustment = None;
    true
}

/// Rasterize every fill-content layer in the tree. Returns how many were
/// rasterized; a nil result leaves the document untouched. Rasterizing keeps
/// the tree shape, so the pre-collected paths stay valid.
/// Rasterize every flattened layer that can be baked: fill content through
/// [`rasterize_fill_content`], else a type layer through the bundled text
/// renderer. Neither operation adds or removes layers, so the path list stays
/// valid. Returns how many were rasterized.
pub fn rasterize_all_layers(doc: &mut Document) -> usize {
    let paths: Vec<String> = flatten_rows(doc)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    paths
        .iter()
        .filter(|path| {
            rasterize_shape(doc, path)
                || rasterize_fill_content(doc, path)
                || crate::render_text_layer(doc, path)
        })
        .count()
}

/// Rasterize the shape layer at `path`: bake its fill cut to the outline, plus
/// its stroke, into the layer's `0/1/2/-1` channels, then drop the shape/vector
/// definition — the solid-fill adjustment, the `vmsk` vector mask, the
/// live-shape `vogk` block, and the stroke/effects `lfx2` block. The name, rect,
/// opacity, fill, blend mode, and layer mask are kept. Returns false (no
/// mutation) when `path` does not resolve to a shape layer.
pub fn rasterize_shape(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = super::paths::resolve_path(doc, path) else {
        return false;
    };
    if !is_shape_layer(layer) {
        return false;
    }
    // The render rect grows to the effects' outward reach: an outside or
    // centered stroke paints past the layer rect, so baking into the rect alone
    // would clip it and change the recomposited appearance.
    let render_rect = effects_bounds(layer);
    // Render the layer's intrinsic appearance through the real compositor.
    // Opacity, blend, and the layer mask stay live on the raster result, so the
    // scratch copy neutralizes them; a shape's fill is binary (`0` or `255`), so
    // leaving it as-is bakes the fill on/off correctly.
    let mut scratch = Document::new(doc.width, doc.height, doc.mode, doc.depth);
    let mut source = layer.clone();
    source.opacity = 255;
    source.mask = None;
    source.blend = pictura_core::BlendMode::Normal;
    scratch.layers = pictura_core::LayerStack::from_layers(vec![source]);
    let rendered = crate::composite_rgba(&scratch);

    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    layer.rect = render_rect;
    bake_from_composite(layer, &rendered);
    layer.adjustment = None;
    layer.vector_mask = None;
    layer.extra_blocks.retain(|block| {
        let key = block.key.as_slice();
        key != b"vmsk" && key != b"vogk" && key != b"lfx2" && key != b"lrFX"
    });
    true
}

/// The union of `layer.rect` and the document-space box its effects can paint,
/// so a bake covers an outside/centered stroke that reaches past the rect.
fn effects_bounds(layer: &Layer) -> PsdRect {
    let reach = crate::layer_effects::effects_reach(layer);
    PsdRect {
        left: layer.rect.left.min(reach.left),
        top: layer.rect.top.min(reach.top),
        right: layer.rect.right.max(reach.right),
        bottom: layer.rect.bottom.max(reach.bottom),
    }
}

/// Copy the four planes of a document-sized `out` within `layer.rect` into the
/// layer's color and alpha channels; out-of-canvas samples read as 0.
fn bake_from_composite(layer: &mut Layer, out: &PixelBuffer) {
    let rect = layer.rect;
    let w = rect.width().max(0) as usize;
    let h = rect.height().max(0) as usize;
    if w == 0 || h == 0 {
        layer.channels = Vec::new();
        return;
    }
    let plane = out.width as usize * out.height as usize;
    let mut planes = [
        vec![0u8; w * h],
        vec![0u8; w * h],
        vec![0u8; w * h],
        vec![0u8; w * h],
    ];
    for y in 0..h {
        for x in 0..w {
            let dx = rect.left + x as i32;
            let dy = rect.top + y as i32;
            if dx < 0 || dy < 0 || dx >= out.width as i32 || dy >= out.height as i32 {
                continue;
            }
            let dst = y * w + x;
            let src = dy as usize * out.width as usize + dx as usize;
            for (index, data) in planes.iter_mut().enumerate() {
                data[dst] = out.data[index * plane + src];
            }
        }
    }
    let [red, green, blue, alpha] = planes;
    layer.channels = vec![
        Channel {
            id: 0,
            data: red.into(),
        },
        Channel {
            id: 1,
            data: green.into(),
        },
        Channel {
            id: 2,
            data: blue.into(),
        },
        Channel {
            id: -1,
            data: alpha.into(),
        },
    ];
}

/// Overwrite the layer's color channels with `rgba` over its rect.
fn bake_solid(layer: &mut Layer, rgba: [u8; 4]) {
    let w = layer.rect.width();
    let h = layer.rect.height();
    if w <= 0 || h <= 0 {
        layer.channels = Vec::new();
        return;
    }
    let n = w as usize * h as usize;
    layer.channels = vec![
        Channel {
            id: 0,
            data: vec![rgba[0]; n].into(),
        },
        Channel {
            id: 1,
            data: vec![rgba[1]; n].into(),
        },
        Channel {
            id: 2,
            data: vec![rgba[2]; n].into(),
        },
        Channel {
            id: -1,
            data: vec![rgba[3]; n].into(),
        },
    ];
}

/// Overwrite the layer's color channels with a generated gradient over its
/// rect. Alpha is opaque; an empty rect leaves no channels.
fn bake_gradient(layer: &mut Layer, params: &GradientFillParams) {
    let w = layer.rect.width();
    let h = layer.rect.height();
    if w <= 0 || h <= 0 {
        layer.channels = Vec::new();
        return;
    }
    let px = crate::fill::gradient_rgba(w, h, params);
    layer.channels = vec![
        Channel {
            id: 0,
            data: px.iter().map(|p| p[0]).collect(),
        },
        Channel {
            id: 1,
            data: px.iter().map(|p| p[1]).collect(),
        },
        Channel {
            id: 2,
            data: px.iter().map(|p| p[2]).collect(),
        },
        Channel {
            id: -1,
            data: vec![255; px.len()].into(),
        },
    ];
}

/// Overwrite the layer's color channels with the tiled pattern (or the
/// placeholder when `params.pattern_id` is absent) over its rect. The pattern's
/// own alpha bakes into `-1`; an empty rect leaves no channels.
fn bake_pattern(layer: &mut Layer, params: &PatternFillParams, patterns: &[PatternPixels]) {
    let w = layer.rect.width();
    let h = layer.rect.height();
    if w <= 0 || h <= 0 {
        layer.channels = Vec::new();
        return;
    }
    let px =
        crate::fill::pattern_tile_rgba(patterns, params, layer.rect.left, layer.rect.top, w, h);
    layer.channels = vec![
        Channel {
            id: 0,
            data: px.iter().map(|p| p[0]).collect(),
        },
        Channel {
            id: 1,
            data: px.iter().map(|p| p[1]).collect(),
        },
        Channel {
            id: 2,
            data: px.iter().map(|p| p[2]).collect(),
        },
        Channel {
            id: -1,
            data: px.iter().map(|p| p[3]).collect(),
        },
    ];
}
