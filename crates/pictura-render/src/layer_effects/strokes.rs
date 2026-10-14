use pictura_adjust::{GradientFillParams, PatternFillParams};
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer};

use crate::composite::{blend_parts, desc_item, mask_alpha, Canvas};

use super::{
    bool_or, clamp_finite, clip_rect, content_matte, decode_color, dilate_matte, effect_blend_mode,
    erode_matte, num_clamped, num_or, pad_rect, rect_empty, with_gradient_defaults, MAX_OPACITY,
    MAX_SIZE,
};

/// The stroke position stored in `Styl` (typeID `FStl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokePosition {
    /// `OutF`: the band is built outside the content edge.
    Outside,
    /// `InsF`: the band is built inside the content edge.
    Inside,
    /// `CtrF`: the band straddles the content edge.
    Center,
}

/// The fill source of a stroke, from `PntT` (typeID `FrFl`).
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeFill {
    /// `SClr`: one flat colour on the `0..=255` scale (default black).
    Solid([u8; 3]),
    /// `GrFl`: the gradient content under `Grad`.
    Gradient {
        params: GradientFillParams,
        /// `Algn`, default true.
        align_with_layer: bool,
    },
    /// `Ptrn`: the pattern content under `Ptrn`.
    Pattern {
        params: PatternFillParams,
        /// `Angl`, decoded for symmetry; not applied.
        angle_deg: f32,
    },
}

/// The typed stroke decoded from a layer's `lfx2` block.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub enabled: bool,
    pub present: bool,
    pub blend_mode: BlendMode,
    pub fill: StrokeFill,
    /// Percent, `0..=100`.
    pub opacity: f32,
    /// Pixels, integer `1..=250`.
    pub size: u32,
    /// `Styl` (typeID `FStl`), default Outside.
    pub position: StrokePosition,
}

/// The stroke solid colour from `Clr ` (absent → black), or `None` for a
/// non-`RGBC` payload.
fn solid_color(frfx: &DescValue) -> Option<[u8; 3]> {
    match desc_item(frfx, b"Clr ") {
        None => Some([0, 0, 0]),
        Some(value) => decode_color(value),
    }
}

/// Decode a layer's `lfx2` Stroke (`FrFX`).
///
/// `PntT` (typeID `FrFl`) selects the fill: absent or `SClr` is the solid
/// `Clr ` (default black), `GrFl` decodes the `Grad` content via the shared
/// `fill::gradient_params_from_desc` with `with_gradient_defaults` supplying an
/// absent `Angl`/`Type`, and `Ptrn` decodes the `Ptrn` content via the shared
/// `fill::pattern_params_from_desc_with_link` with the link read authoritatively
/// from `Lnkd` (falling back to `Algn` only when `Lnkd` is absent). An unknown
/// `PntT`, a missing or malformed `Grad`/`Ptrn`, and a wrongly-typed key are
/// `None`.
///
/// ponytail: gradient noise, `Dither`, `Ofst` and stop midpoints are not
/// modelled, pattern `Angl` rotation is decoded but not applied, the aligned
/// gradient clamps at the layer-rect edge and the pattern is anchored to the
/// layer rect (design D4), the solid colour defaults to black rather than the
/// docs' "foreground `(inferred)`", and contour (`TrnS`), anti-alias (`AntA`)
/// and `overprint` are ignored. `size` is rounded to the nearest integer and
/// clamped to `1..=250`.
///
/// A missing `lfx2`, a missing or wrongly-typed `FrFX`, an unknown data
/// version, a wrong-typed or non-finite value, or a parse error is `None`.
/// Never panics.
pub fn decode_stroke(layer: &Layer) -> Option<Stroke> {
    let data = &layer.extra_block(b"lfx2")?.data;
    if data.len() < 8 {
        return None;
    }
    let body = data.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    let frfx = desc_item(&obj, b"FrFX")?;
    let DescValue::Object { class_id, .. } = frfx else {
        return None;
    };
    if class_id.as_slice() != b"FrFX" {
        return None;
    }
    let blend_mode = match desc_item(frfx, b"Md  ") {
        None => BlendMode::Normal,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"BlnM" => {
            effect_blend_mode(value, BlendMode::Normal)
        }
        Some(_) => return None,
    };
    let position = match desc_item(frfx, b"Styl") {
        None => StrokePosition::Outside,
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"FStl" => {
            match value.as_slice() {
                b"InsF" => StrokePosition::Inside,
                b"CtrF" => StrokePosition::Center,
                // `OutF`, or an unknown value, is Outside.
                _ => StrokePosition::Outside,
            }
        }
        Some(_) => return None,
    };
    let fill = match desc_item(frfx, b"PntT") {
        None => StrokeFill::Solid(solid_color(frfx)?),
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"FrFl" => {
            match value.as_slice() {
                b"SClr" => StrokeFill::Solid(solid_color(frfx)?),
                b"GrFl" => {
                    let params =
                        crate::fill::gradient_params_from_desc(&with_gradient_defaults(frfx))?;
                    StrokeFill::Gradient {
                        params,
                        align_with_layer: bool_or(frfx, b"Algn", true)?,
                    }
                }
                b"Ptrn" => {
                    // `Lnkd` is the stroke pattern's link key (the overlay uses
                    // `Algn`); decode it first so a valid `Lnkd` wins even when a
                    // present-but-wrongly-typed `Algn` would reject the shared
                    // helper, and fall back to `Algn` when `Lnkd` is absent.
                    let link = match desc_item(frfx, b"Lnkd") {
                        Some(DescValue::Bool(linked)) => Some(*linked),
                        Some(_) => return None,
                        None => None,
                    };
                    let params = crate::fill::pattern_params_from_desc_with_link(frfx, link)?;
                    StrokeFill::Pattern {
                        params,
                        angle_deg: num_or(frfx, b"Angl", 0.0)?,
                    }
                }
                _ => return None,
            }
        }
        Some(_) => return None,
    };
    Some(Stroke {
        enabled: bool_or(frfx, b"enab", false)?,
        present: bool_or(frfx, b"present", false)?,
        blend_mode,
        fill,
        opacity: num_clamped(frfx, b"Opct", 100.0, 0.0, MAX_OPACITY)?,
        size: num_clamped(frfx, b"Sz  ", 3.0, 1.0, MAX_SIZE)?.round() as u32,
        position,
    })
}

/// Build the masked content matte `M` over the padded source region, form the
/// band from exact integer max/min filters, and composite
/// `band · source_alpha · opacity/100` above the content over `padded` only,
/// where the source is the stroke fill: the flat `Solid` colour (alpha 1), the
/// `Gradient` sampled over the layer rect when aligned (out-of-rect samples
/// clamped to the nearest edge) else over the canvas, or the `Pattern` tiled
/// from `padded` and anchored to the layer rect when linked (else the origin)
/// with the tile alpha.
///
/// ponytail: the band is composited after the interior effects, so it sits on
/// top of them; the exact inter-effect order and the isolated `Blend
/// Interior Effects As Group` composite are not modelled. The max/min filters
/// are separable, so a canvas-filling layer at the maximum size costs
/// `O(canvas · size)` — the existing compositor ceiling, plus one
/// gradient/pattern buffer.
pub(super) fn composite_stroke(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    stroke: &Stroke,
) {
    let (w, h) = (canvas.w as i32, canvas.h as i32);
    if w == 0 || h == 0 {
        return;
    }
    let source = clip_rect(layer, w, h);
    if rect_empty(source) {
        return;
    }
    // Clamp again: a hand-built `Stroke` may not have gone through decode.
    let size = clamp_finite(stroke.size as f32, MAX_SIZE) as i64;
    let opacity = clamp_finite(stroke.opacity, MAX_OPACITY) / 100.0;
    if size == 0 || opacity == 0.0 {
        return;
    }
    let (out_r, in_r) = match stroke.position {
        StrokePosition::Outside => (size, 0),
        StrokePosition::Inside => (0, size),
        StrokePosition::Center => ((size + 1) / 2, size / 2),
    };
    // The matte is built over `source` padded by `size` on every side: the pad
    // bounds the outside half of the band and supplies the zero content beyond
    // the source edge that the erode min filter needs.
    let padded = pad_rect(source, size, w, h);
    if rect_empty(padded) {
        return;
    }
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded);
    for y in py0..py1 {
        for x in px0..px1 {
            matte[(y - py0) as usize * pw + (x - px0) as usize] *=
                mask_alpha(layer, crate::mask_refine::canvas(doc), x, y) as f32 / 255.0;
        }
    }
    if matte.iter().all(|&v| v <= 0.0) {
        return;
    }
    let dilate = |r: i64| {
        if r > 0 {
            dilate_matte(&matte, pw, ph, r)
        } else {
            matte.clone()
        }
    };
    let erode = |r: i64| {
        if r > 0 {
            erode_matte(&matte, pw, ph, r)
        } else {
            matte.clone()
        }
    };
    let band: Vec<f32> = match stroke.position {
        StrokePosition::Outside => dilate(out_r)
            .iter()
            .zip(&matte)
            .map(|(d, m)| (d - m).clamp(0.0, 1.0))
            .collect(),
        StrokePosition::Inside => matte
            .iter()
            .zip(&erode(in_r))
            .map(|(m, e)| (m - e).clamp(0.0, 1.0))
            .collect(),
        StrokePosition::Center => {
            let d = dilate(out_r);
            let e = erode(in_r);
            d.iter()
                .zip(&e)
                .map(|(a, b)| (a - b).clamp(0.0, 1.0))
                .collect()
        }
    };
    // The gradient buffer is generated over the layer rect (aligned) or the
    // canvas (unaligned); both leave every `padded` sample in range, and the
    // aligned out-of-rect samples are clamped to the nearest edge (design D4).
    let gradient = match &stroke.fill {
        StrokeFill::Gradient {
            params,
            align_with_layer,
        } => {
            let (gw, gh, left, top) = if *align_with_layer {
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
            Some((
                crate::fill::gradient_rgba(gw, gh, params),
                gw,
                gh,
                left,
                top,
            ))
        }
        _ => None,
    };
    // The pattern tile is baked once over `padded`, anchored to the layer rect
    // when linked (matching the pattern overlay) else the canvas origin; the
    // shipped `Tile` wraps both axes, so the pattern repeats beyond the content.
    let pattern = match &stroke.fill {
        StrokeFill::Pattern { params, .. } => {
            let anchor = if params.link_with_layer {
                (layer.rect.left, layer.rect.top)
            } else {
                (0, 0)
            };
            let patterns = pictura_codec::decode_patterns(doc);
            Some(crate::fill::pattern_tile_region(
                &patterns, params, anchor, padded,
            ))
        }
        _ => None,
    };
    for y in py0..py1 {
        for x in px0..px1 {
            let i = (y - py0) as usize * pw + (x - px0) as usize;
            let (rgb, src_a) = match &stroke.fill {
                StrokeFill::Solid(c) => (rgb_f32(*c), 1.0),
                StrokeFill::Gradient { .. } => {
                    let Some((grad, gw, gh, left, top)) = gradient.as_ref() else {
                        continue;
                    };
                    let (gw, gh, left, top) = (*gw, *gh, *left, *top);
                    let gx = (x - left).clamp(0, (gw - 1).max(0)) as usize;
                    let gy = (y - top).clamp(0, (gh - 1).max(0)) as usize;
                    let c = grad
                        .get(gy * gw as usize + gx)
                        .copied()
                        .unwrap_or([0, 0, 0, 255]);
                    (rgb4_f32(c), 1.0)
                }
                StrokeFill::Pattern { .. } => {
                    let Some(tile) = pattern.as_ref() else {
                        continue;
                    };
                    let c = tile.get(i).copied().unwrap_or([0, 0, 0, 0]);
                    (rgb4_f32(c), c[3] as f32 / 255.0)
                }
            };
            let alpha = band[i] * src_a * opacity;
            if alpha > 0.0 {
                blend_parts(
                    canvas,
                    x as usize,
                    y as usize,
                    rgb,
                    alpha,
                    stroke.blend_mode,
                );
            }
        }
    }
}

fn rgb_f32(c: [u8; 3]) -> [f32; 3] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    ]
}

fn rgb4_f32(c: [u8; 4]) -> [f32; 3] {
    rgb_f32([c[0], c[1], c[2]])
}
