use pictura_adjust::{GradientFillParams, PatternFillParams};
use pictura_codec::DescValue;
use pictura_core::{BlendMode, Document, Layer, PsdRect};

use crate::composite::desc_item;

use super::apply_mask;
use super::planes::{EffectPass, PassWriter};
use super::BuildRegion;
use super::{
    bool_or, clamp_finite, content_matte, decode_color, dilate_matte, effect_blend_mode,
    effect_source_rect, erode_matte, num_clamped, num_or, pad_rect, rect_empty,
    with_gradient_defaults, MAX_OPACITY, MAX_SIZE,
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

/// The authored stroke line type (`Lntp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeLine {
    Solid,
    Dashed,
    Dotted,
}

/// An authored dash pattern in pixels (`DshP`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashPattern {
    pub on: f64,
    pub off: f64,
}

impl DashPattern {
    /// The pattern a line type implies for a `size`-px stroke.
    pub fn for_line(line: StrokeLine, size: u32) -> Option<Self> {
        let width = f64::from(size.max(1));
        match line {
            StrokeLine::Solid => None,
            StrokeLine::Dashed => Some(Self {
                on: width * 2.0,
                off: width * 2.0,
            }),
            StrokeLine::Dotted => Some(Self {
                on: (width * 0.5).max(1.0),
                off: (width * 1.5).max(2.0),
            }),
        }
    }
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
    /// `Lntp`; Solid when absent.
    pub line: StrokeLine,
    /// `DshP`; derived from `line` and `size` when the line is not solid.
    pub dash: Option<DashPattern>,
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
fn decode_line(frfx: &DescValue) -> Option<StrokeLine> {
    match desc_item(frfx, b"Lntp") {
        None => Some(StrokeLine::Solid),
        Some(DescValue::Text(value)) => match value.as_str() {
            "solid" => Some(StrokeLine::Solid),
            "dashed" => Some(StrokeLine::Dashed),
            "dotted" => Some(StrokeLine::Dotted),
            _ => None,
        },
        Some(_) => None,
    }
}

fn decode_dash(frfx: &DescValue) -> Option<Option<DashPattern>> {
    match desc_item(frfx, b"DshP") {
        None => Some(None),
        Some(DescValue::Text(value)) => {
            let (on, off) = value.split_once(',')?;
            let on: f64 = on.trim().parse().ok()?;
            let off: f64 = off.trim().parse().ok()?;
            (on.is_finite() && off.is_finite() && on > 0.0 && off > 0.0)
                .then_some(Some(DashPattern { on, off }))
        }
        Some(_) => None,
    }
}

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
    let line = decode_line(frfx)?;
    let size = num_clamped(frfx, b"Sz  ", 3.0, 1.0, MAX_SIZE)?.round() as u32;
    let dash = decode_dash(frfx)?.or_else(|| DashPattern::for_line(line, size));
    Some(Stroke {
        enabled: bool_or(frfx, b"enab", false)?,
        present: bool_or(frfx, b"present", false)?,
        blend_mode,
        fill,
        opacity: num_clamped(frfx, b"Opct", 100.0, 0.0, MAX_OPACITY)?,
        size,
        position,
        line,
        dash,
    })
}

pub(super) fn build_stroke(
    layer: &Layer,
    doc: &Document,
    stroke: &Stroke,
    build: BuildRegion,
    fill: Option<&pictura_core::FillContent>,
) -> Vec<EffectPass> {
    let (w, h) = (
        doc.width.min(i32::MAX as u32) as i32,
        doc.height.min(i32::MAX as u32) as i32,
    );
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let source = effect_source_rect(layer, w, h);
    if rect_empty(source) {
        return Vec::new();
    }
    // Clamp again: a hand-built `Stroke` may not have gone through decode.
    let size = clamp_finite(stroke.size as f32, MAX_SIZE) as i64;
    let opacity = clamp_finite(stroke.opacity, MAX_OPACITY) / 100.0;
    if size == 0 || opacity == 0.0 {
        return Vec::new();
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
    let Some(padded) = build.buffer(padded, size) else {
        return Vec::new();
    };
    let (px0, py0, px1, py1) = padded;
    let pw = (px1 - px0) as usize;
    let ph = (py1 - py0) as usize;
    let mut matte = content_matte(layer, doc, padded, fill);
    apply_mask(&mut matte, pw, px0, py0, layer, build.mask_clip);
    if matte.iter().all(|&v| v <= 0.0) {
        return Vec::new();
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
    let band = match stroke.dash {
        Some(dash) => apply_dash(layer, &band, (px0, py0, pw, ph), size as f64 + 1.0, dash),
        None => band,
    };
    // The gradient buffer is generated over the layer rect (aligned) or the
    // document (unaligned); both leave every `padded` sample in range, and the
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
                (
                    doc.width.min(i32::MAX as u32) as i32,
                    doc.height.min(i32::MAX as u32) as i32,
                    0,
                    0,
                )
            };
            if gw <= 0 || gh <= 0 {
                return Vec::new();
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
    // when linked (matching the pattern overlay) else the document origin; the
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
    let rect = PsdRect {
        top: py0,
        left: px0,
        bottom: py1,
        right: px1,
    };
    let Some(rect) = build.output(rect) else {
        return Vec::new();
    };
    let mut writer = match &stroke.fill {
        StrokeFill::Solid(color) => PassWriter::constant(rect, stroke.blend_mode, rgb_f32(*color)),
        _ => PassWriter::per_pixel(rect, stroke.blend_mode),
    };
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
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
                writer.emit(x, y, rgb, alpha);
            }
        }
    }
    vec![writer.finish()]
}

/// Cut `band` into `dash` gaps along the layer's vector-mask subpaths. A layer
/// without a vector mask keeps the solid band.
fn apply_dash(
    layer: &Layer,
    band: &[f32],
    (px0, py0, pw, ph): (i32, i32, usize, usize),
    radius: f64,
    dash: DashPattern,
) -> Vec<f32> {
    let Some(mask) = layer.vector_mask.as_ref().filter(|mask| !mask.disabled) else {
        return band.to_vec();
    };
    let mut hit = vec![false; pw * ph];
    let mut any = false;
    let period = dash.on + dash.off;
    if period <= 0.0 {
        return band.to_vec();
    }
    for subpath in &mask.subpaths {
        if subpath.points.len() < 2 {
            continue;
        }
        // `VectorSubpath::points` is already flattened through the Bezier
        // handles by the codec, so walking consecutive points follows the curve.
        let mut points: Vec<(f64, f64)> = subpath
            .points
            .iter()
            .map(|p| (p[0] as f64 / 256.0, p[1] as f64 / 256.0))
            .collect();
        if subpath.closed && points.first() != points.last() {
            points.push(points[0]);
        }
        let total: f64 = points
            .windows(2)
            .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
            .sum();
        if total <= 0.0 {
            continue;
        }
        let mut arc = 0.0;
        while arc < total {
            let end = (arc + dash.on).min(total);
            draw_dash_interval(&points, arc, end, radius, (px0, py0, pw, ph), &mut hit);
            any = true;
            arc += period;
        }
    }
    if !any {
        return band.to_vec();
    }
    band.iter()
        .zip(hit)
        .map(|(&value, on)| if on { value } else { 0.0 })
        .collect()
}

fn draw_dash_interval(
    points: &[(f64, f64)],
    start: f64,
    end: f64,
    radius: f64,
    (px0, py0, pw, ph): (i32, i32, usize, usize),
    hit: &mut [bool],
) {
    let mut arc = 0.0;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        if length <= 0.0 {
            continue;
        }
        let seg_start = arc;
        arc += length;
        let lo = start.max(seg_start);
        let hi = end.min(arc);
        if hi <= lo {
            continue;
        }
        let t0 = (lo - seg_start) / length;
        let t1 = (hi - seg_start) / length;
        let from = (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0);
        let to = (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1);
        draw_capsule(from, to, radius, (px0, py0, pw, ph), hit);
    }
}

fn draw_capsule(
    from: (f64, f64),
    to: (f64, f64),
    radius: f64,
    (px0, py0, pw, ph): (i32, i32, usize, usize),
    hit: &mut [bool],
) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length2 = dx * dx + dy * dy;
    if length2 <= 0.0 {
        return;
    }
    let (px1, py1) = (px0 + pw as i32, py0 + ph as i32);
    let min_x = (from.0.min(to.0) - radius).floor() as i32;
    let max_x = (from.0.max(to.0) + radius).ceil() as i32;
    let min_y = (from.1.min(to.1) - radius).floor() as i32;
    let max_y = (from.1.max(to.1) + radius).ceil() as i32;
    for y in min_y.max(py0)..=max_y.min(py1 - 1) {
        for x in min_x.max(px0)..=max_x.min(px1 - 1) {
            let point = (x as f64 + 0.5, y as f64 + 0.5);
            let t = ((point.0 - from.0) * dx + (point.1 - from.1) * dy) / length2;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let distance = (point.0 - (from.0 + dx * t)).hypot(point.1 - (from.1 + dy * t));
            if distance <= radius {
                hit[(y - py0) as usize * pw + (x - px0) as usize] = true;
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
