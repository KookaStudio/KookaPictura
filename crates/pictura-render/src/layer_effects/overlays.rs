//! Object-based overlay layer effects: `SoFi` Color Overlay, `GrFl` Gradient
//! Overlay and `patternFill` Pattern Overlay.
//!
//! An overlay fills the layer's own content coverage with a source colour,
//! composited **above** the layer content. The masked content matte `M` supplies
//! the coverage and the source alpha is `M · source_alpha · opacity/100`, with
//! the overlay's own blend mode.
//!
//! ponytail: the effect object class ids are `SoFi`/`GrFl`/`patternFill`, not
//! the `SoCo`/`PtFl` fill-layer ids (two independent implementations agree).
//! Gradient `Ofst`, noise and CS6 `Dither` are not modelled; pattern `Angl`
//! rotation is decoded but not applied; the unaligned gradient buffer is
//! canvas-sized; the exact inter-effect order among the overlays and
//! Stroke is not modelled.

use pictura_adjust::{GradientFillParams, GradientKind, GradientStop, PatternFillParams};
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer};

use crate::composite::{blend_parts, desc_item, mask_alpha, Canvas};

use super::{
    bool_or, clamp_finite, clip_rect, content_matte, effect_blend_mode, num_clamped, num_or,
    rect_empty, MAX_OPACITY,
};

/// The typed color overlay decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorOverlay {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    /// Red, green, blue on the `0..=255` scale; default red `(inferred)`.
    pub color: [u8; 3],
    /// Percent, `0..=100`.
    pub opacity: f32,
}

/// The typed gradient overlay decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientOverlay {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    /// Percent, `0..=100`.
    pub opacity: f32,
    pub stops: Vec<GradientStop>,
    pub reverse: bool,
    pub kind: GradientKind,
    pub angle_deg: f32,
    /// Percent.
    pub scale: f32,
    pub align_with_layer: bool,
}

/// The typed pattern overlay decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternOverlay {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    /// Percent, `0..=100`.
    pub opacity: f32,
    pub pattern_id: String,
    /// Percent.
    pub scale: f32,
    /// Decoded for symmetry; not applied.
    pub angle_deg: f32,
    pub align_with_layer: bool,
    /// The optional `phase` `Pnt ` tile origin, default `(0, 0)`.
    pub origin: (i32, i32),
}

/// Read a layer's `lfx2` block into its top-level descriptor object. A missing
/// block, a too-short payload, a malformed descriptor or a non-object top level
/// is `None`.
fn read_effect(layer: &Layer) -> Option<DescValue> {
    let data = &layer.extra_block(b"lfx2")?.data;
    if data.len() < 8 {
        return None;
    }
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    matches!(obj, DescValue::Object { .. }).then_some(obj)
}

/// The effect object under `key` whose class id is `class_id`; a missing or
/// wrongly-classed object is `None`.
fn effect_object<'a>(top: &'a DescValue, key: &[u8], class_id: &[u8]) -> Option<&'a DescValue> {
    let value = desc_item(top, key)?;
    let DescValue::Object { class_id: id, .. } = value else {
        return None;
    };
    (id.as_slice() == class_id).then_some(value)
}

/// The blend mode stored in `Md  ` (typeID `BlnM`); absent or an unknown value
/// is `Normal`, a wrong typeID or wrong-typed item rejects.
fn decode_blend_mode(obj: &DescValue) -> Option<BlendMode> {
    match desc_item(obj, b"Md  ") {
        None => Some(BlendMode::Normal),
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            Some(effect_blend_mode(value, BlendMode::Normal))
        }
        Some(_) => None,
    }
}

/// The four fields every overlay shares: `enab`, `present`, `Md  ` and `Opct`.
fn common(obj: &DescValue) -> Option<(bool, bool, BlendMode, f32)> {
    Some((
        bool_or(obj, b"enab", false)?,
        bool_or(obj, b"present", false)?,
        decode_blend_mode(obj)?,
        num_clamped(obj, b"Opct", 100.0, 0.0, MAX_OPACITY)?,
    ))
}

/// Decode a layer's `lfx2` Color Overlay (`SoFi`). Missing keys default to
/// `Normal`, red and opacity 100; malformed input is `None`. Never panics.
pub fn decode_color_overlay(layer: &Layer) -> Option<ColorOverlay> {
    let top = read_effect(layer)?;
    let obj = effect_object(&top, b"SoFi", b"SoFi")?;
    let (enabled, present, blend_mode, opacity) = common(obj)?;
    let color = match desc_item(obj, b"Clr ") {
        None => [255, 0, 0],
        Some(value) => super::decode_color(value)?,
    };
    Some(ColorOverlay {
        enabled,
        present,
        blend_mode,
        color,
        opacity,
    })
}

/// Decode a layer's `lfx2` Gradient Overlay (`GrFl`). The gradient fields are
/// decoded by the reused `fill::gradient_params_from_desc`; an absent `Angl`
/// defaults to 0 and an absent `Type` to `Linear` (the reference's defaults, which
/// the strict `GdFl` helper does not supply). Missing `Rvrs`, `Scl `, `Algn`,
/// `Md  ` and `Opct` likewise take the reference defaults. Malformed input is
/// `None`. Never panics.
pub fn decode_gradient_overlay(layer: &Layer) -> Option<GradientOverlay> {
    let top = read_effect(layer)?;
    let obj = effect_object(&top, b"GrFl", b"GrFl")?;
    let (enabled, present, blend_mode, opacity) = common(obj)?;
    let params = crate::fill::gradient_params_from_desc(&super::with_gradient_defaults(obj))?;
    Some(GradientOverlay {
        enabled,
        present,
        blend_mode,
        opacity,
        stops: params.stops,
        reverse: params.reverse,
        kind: params.kind,
        angle_deg: params.angle_deg,
        scale: params.scale,
        align_with_layer: bool_or(obj, b"Algn", true)?,
    })
}

/// Decode a layer's `lfx2` Pattern Overlay (`patternFill`). The pattern fields
/// are decoded by the reused `fill::pattern_params_from_desc`; `Angl` is
/// decoded for symmetry but not applied, and the optional `phase` origin is
/// carried for the render path. Missing `Scl `, `Algn`, `Md  ` and `Opct` take
/// the reference defaults. Malformed input is `None`. Never panics.
pub fn decode_pattern_overlay(layer: &Layer) -> Option<PatternOverlay> {
    let top = read_effect(layer)?;
    let obj = effect_object(&top, b"patternFill", b"patternFill")?;
    let (enabled, present, blend_mode, opacity) = common(obj)?;
    let params = crate::fill::pattern_params_from_desc(obj)?;
    Some(PatternOverlay {
        enabled,
        present,
        blend_mode,
        opacity,
        pattern_id: params.pattern_id,
        scale: params.scale,
        angle_deg: num_or(obj, b"Angl", 0.0)?,
        align_with_layer: params.link_with_layer,
        origin: params.origin,
    })
}

/// A canvas region `(x0, y0, x1, y1)`.
type Region = (i32, i32, i32, i32);

/// The content region, its masked coverage matte, and the clamped opacity.
type Coverage = (Region, Vec<f32>, f32);

/// The layer's content rect clamped to the canvas, the masked coverage matte
/// `M` over it, and the clamped `opacity/100`. `None` when the rect is empty,
/// `opacity` is 0 or the matte is all zero.
fn coverage(canvas: &Canvas, layer: &Layer, doc: &Document, opacity: f32) -> Option<Coverage> {
    let (w, h) = (canvas.w as i32, canvas.h as i32);
    if w == 0 || h == 0 {
        return None;
    }
    let region = clip_rect(layer, w, h);
    if rect_empty(region) {
        return None;
    }
    let opacity = clamp_finite(opacity, MAX_OPACITY) / 100.0;
    if opacity == 0.0 {
        return None;
    }
    let (x0, y0, x1, y1) = region;
    let rw = (x1 - x0) as usize;
    let mut matte = content_matte(layer, doc, region);
    for y in y0..y1 {
        for x in x0..x1 {
            matte[(y - y0) as usize * rw + (x - x0) as usize] *=
                mask_alpha(layer, crate::mask_refine::canvas(doc), x, y) as f32 / 255.0;
        }
    }
    if matte.iter().all(|&v| v <= 0.0) {
        return None;
    }
    Some((region, matte, opacity))
}

fn rgb(c: [u8; 4]) -> [f32; 3] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    ]
}

/// Composite a color overlay above the content: for each pixel the flat colour,
/// alpha `M · opacity/100`, in the overlay's own blend mode.
pub(super) fn composite_color_overlay(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    overlay: &ColorOverlay,
) {
    let Some((region, matte, opacity)) = coverage(canvas, layer, doc, overlay.opacity) else {
        return;
    };
    let (x0, y0, x1, y1) = region;
    let w = (x1 - x0) as usize;
    let color = [
        overlay.color[0] as f32 / 255.0,
        overlay.color[1] as f32 / 255.0,
        overlay.color[2] as f32 / 255.0,
    ];
    for y in y0..y1 {
        for x in x0..x1 {
            let m = matte[(y - y0) as usize * w + (x - x0) as usize];
            if m > 0.0 {
                blend_parts(
                    canvas,
                    x as usize,
                    y as usize,
                    color,
                    m * opacity,
                    overlay.blend_mode,
                );
            }
        }
    }
}

/// Composite a gradient overlay above the content. The gradient is generated
/// over the layer rect when `align_with_layer` (sampling local `(x-left,
/// y-top)`), else over the canvas (sampling `(x, y)`); alpha `M · opacity/100`.
pub(super) fn composite_gradient_overlay(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    overlay: &GradientOverlay,
) {
    let Some((region, matte, opacity)) = coverage(canvas, layer, doc, overlay.opacity) else {
        return;
    };
    let params = GradientFillParams {
        stops: overlay.stops.clone(),
        reverse: overlay.reverse,
        kind: overlay.kind,
        angle_deg: overlay.angle_deg,
        scale: overlay.scale,
    };
    let (gw, gh, left, top) = if overlay.align_with_layer {
        (
            layer.rect.width(),
            layer.rect.height(),
            layer.rect.left,
            layer.rect.top,
        )
    } else {
        (canvas.w as i32, canvas.h as i32, 0, 0)
    };
    if gw <= 0 || gh <= 0 {
        return;
    }
    let grad = crate::fill::gradient_rgba(gw, gh, &params);
    let grow = gw as usize;
    let (x0, y0, x1, y1) = region;
    let w = (x1 - x0) as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let m = matte[(y - y0) as usize * w + (x - x0) as usize];
            if m <= 0.0 {
                continue;
            }
            // The sample is in range for both rects; the guard makes a craft
            // struct's non-finite `scale` (an all-zero gradient) a bounded no-op.
            let Some(c) = grad.get((y - top).max(0) as usize * grow + (x - left).max(0) as usize)
            else {
                continue;
            };
            blend_parts(
                canvas,
                x as usize,
                y as usize,
                rgb(*c),
                m * opacity,
                overlay.blend_mode,
            );
        }
    }
}

/// Composite a pattern overlay above the content, tiling the document pattern
/// (the grey placeholder when the id is absent) over the layer rect, anchored
/// by `align_with_layer` and offset by the `phase` origin; alpha
/// `M · pattern_alpha/255 · opacity/100`.
pub(super) fn composite_pattern_overlay(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    overlay: &PatternOverlay,
) {
    let Some((region, matte, opacity)) = coverage(canvas, layer, doc, overlay.opacity) else {
        return;
    };
    let (lw, lh) = (layer.rect.width(), layer.rect.height());
    if lw <= 0 || lh <= 0 {
        return;
    }
    let origin = overlay.origin;
    let params = PatternFillParams {
        pattern_id: overlay.pattern_id.clone(),
        scale: overlay.scale,
        link_with_layer: overlay.align_with_layer,
        origin,
    };
    let patterns = pictura_codec::decode_patterns(doc);
    let tile =
        crate::fill::pattern_tile_rgba(&patterns, &params, layer.rect.left, layer.rect.top, lw, lh);
    let trow = lw as usize;
    let (x0, y0, x1, y1) = region;
    let w = (x1 - x0) as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let m = matte[(y - y0) as usize * w + (x - x0) as usize];
            if m <= 0.0 {
                continue;
            }
            let li =
                (y - layer.rect.top).max(0) as usize * trow + (x - layer.rect.left).max(0) as usize;
            let Some(c) = tile.get(li) else {
                continue;
            };
            let alpha = m * (c[3] as f32 / 255.0) * opacity;
            if alpha > 0.0 {
                blend_parts(
                    canvas,
                    x as usize,
                    y as usize,
                    rgb(*c),
                    alpha,
                    overlay.blend_mode,
                );
            }
        }
    }
}
