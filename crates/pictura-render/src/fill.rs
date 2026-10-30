//! Generative fill content: solid (`SoCo`) and gradient (`GdFl`).
//!
//! A fill layer owns no pixels; it is a descriptor that the compositor paints
//! over the layer's rect. This module is the single decoder for both fill keys
//! and the geometry that turns a gradient descriptor into pixels, following
//! psd-tools' `composite/paint.py::draw_gradient_fill`.
//!
//! ponytail: noise gradients (`ClNs`), transparency/opacity stops, stop
//! midpoints (`Mdpn`), non-linear interpolation (`Intr`), and non-RGB colour
//! models are not modelled; a fill that needs them renders as the nearest
//! custom-stop approximation. Raise these one at a time when a fixture shows a
//! real gap.

use pictura_adjust::{
    Adjustment, GradientFillParams, GradientKind, GradientStop, PatternFillParams,
};
use pictura_codec::{DescValue, PatternPixels};
use pictura_core::{AdjustmentData, Document, Layer, PixelBuffer};

use crate::composite::{blend_into_ctx, desc_item, Canvas};

/// The decoded `GdFl` descriptor, or `None` when it is not a custom-stop
/// gradient this crate understands. Never panics; every failure is a no-op.
pub fn decode_gradient_fill(d: &[u8]) -> Option<Adjustment> {
    let obj = pictura_codec::read_descriptor(d).ok()?;
    Some(Adjustment::GradientFill(gradient_params_from_desc(&obj)?))
}

/// The decoded `vscg` vector-fill block, or `None` when it is malformed or an
/// unknown fill kind. Never panics.
///
/// The block is `[4-byte fill key][version-16 descriptor]`. A `Grad` object is a
/// gradient, a `Ptrn` object a pattern, a `Clr ` object a solid fill; anything
/// else is `None`, and a version other than 16 is rejected by
/// [`pictura_codec::read_descriptor`]. The fill decoders are the same ones the
/// top-level `GdFl`/`PtFl`/`SoCo` blocks use, so the layouts cannot drift.
///
/// ponytail: the 4-byte key is consumed but ignored; the descriptor's content is
/// the dispatch authority, mirroring ag-psd `parseVectorContent`.
pub fn decode_vector_fill(d: &[u8]) -> Option<Adjustment> {
    let body = d.get(4..)?;
    let obj = pictura_codec::read_descriptor(body).ok()?;
    if desc_item(&obj, b"Grad").is_some() {
        gradient_params_from_desc(&obj).map(Adjustment::GradientFill)
    } else if desc_item(&obj, b"Ptrn").is_some() {
        pattern_params_from_desc(&obj).map(Adjustment::PatternFill)
    } else if desc_item(&obj, b"Clr ").is_some() {
        crate::composite::decode_solid_fill(body)
    } else {
        None
    }
}

/// The layer's decoded fill content: its modeled adjustment block first, else
/// its preserved `vscg` vector fill. Shared by the compositor dispatch and the
/// layer-effects coverage matte, so a shape layer's fill and its effects are
/// gated by the same content.
///
/// The choice is strict: an adjustment block is consulted on its own, so an
/// undecodable key is a no-op and never falls through to the `vscg` fill; the
/// `vscg` block is decoded only when the layer has no adjustment block.
pub fn decode_layer_fill(layer: &Layer) -> Option<Adjustment> {
    match layer.adjustment.as_ref() {
        Some(data) => crate::decode_adjustment(data),
        None => layer
            .extra_block(b"vscg")
            .and_then(|b| decode_vector_fill(&b.data)),
    }
}

/// The gradient fields of an already-read `GdFl`/`GrFl` object. Shared by the
/// public fill decoder and the gradient-overlay decoder. A `doub` or a unit
/// float is accepted for the numerics (a `GdFl` carries `doub`, an effect
/// `GrFl` carries unit floats); a finite `f64` that overflows `f32` rejects.
pub(crate) fn gradient_params_from_desc(obj: &DescValue) -> Option<GradientFillParams> {
    if !matches!(obj, DescValue::Object { .. }) {
        return None;
    }
    let angle_deg = match desc_item(obj, b"Angl") {
        Some(DescValue::Double(v)) if v.is_finite() => as_f32_finite(*v)?,
        Some(DescValue::UnitFloat { value, .. }) if value.is_finite() => as_f32_finite(*value)?,
        _ => return None,
    };
    let kind = match desc_item(obj, b"Type") {
        Some(DescValue::Enum { kind, value }) if kind.as_slice() == b"GrdT" => decode_kind(value)?,
        _ => return None,
    };
    let grad = desc_item(obj, b"Grad")?;
    if !matches!(grad, DescValue::Object { .. }) {
        return None;
    }
    match desc_item(grad, b"GrdF") {
        Some(DescValue::Enum { kind, value })
            if kind.as_slice() == b"GrdF" && value.as_slice() == b"CstS" => {}
        _ => return None,
    }
    let stops = decode_stops(desc_item(grad, b"Clrs")?)?;
    let reverse = match desc_item(obj, b"Rvrs") {
        Some(DescValue::Bool(b)) => *b,
        Some(_) => return None,
        None => false,
    };
    let scale = match desc_item(obj, b"Scl ") {
        Some(DescValue::Double(v)) if v.is_finite() => as_f32_finite(*v)?,
        Some(DescValue::UnitFloat { value, .. }) if value.is_finite() => as_f32_finite(*value)?,
        Some(_) => return None,
        None => 100.0,
    };
    Some(GradientFillParams {
        stops,
        reverse,
        kind,
        angle_deg,
        scale,
    })
}

/// A finite `f32` from an `f64`, or `None` when the cast overflows.
fn as_f32_finite(value: f64) -> Option<f32> {
    let v = value as f32;
    v.is_finite().then_some(v)
}

/// The decoded `PtFl` descriptor, or `None` when it is not a pattern fill this
/// crate understands. Never panics; every failure is a no-op.
pub fn decode_pattern_fill(d: &[u8]) -> Option<Adjustment> {
    let obj = pictura_codec::read_descriptor(d).ok()?;
    Some(Adjustment::PatternFill(pattern_params_from_desc(&obj)?))
}

/// The pattern fields of an already-read `PtFl`/`patternFill` object. Shared by
/// the public fill decoder and the pattern-overlay decoder (a pure move; the
/// strict `PtFl` contract is unchanged).
pub(crate) fn pattern_params_from_desc(obj: &DescValue) -> Option<PatternFillParams> {
    pattern_params_from_desc_with_link(obj, None)
}

/// [`pattern_params_from_desc`] with the link flag forced when `Some`.
///
/// The overlay pattern links via `Algn`; the stroke pattern links via `Lnkd`
/// (psd-tools `_PatternMixin`), so the stroke passes its decoded `Lnkd` here.
/// Forcing it bypasses the helper's `Algn` requirement, so a valid `Lnkd` wins
/// even when a present-but-wrongly-typed `Algn` would otherwise reject.
pub(crate) fn pattern_params_from_desc_with_link(
    obj: &DescValue,
    link_override: Option<bool>,
) -> Option<PatternFillParams> {
    if !matches!(obj, DescValue::Object { .. }) {
        return None;
    }
    let ptrn = desc_item(obj, b"Ptrn")?;
    let DescValue::Object { class_id, .. } = ptrn else {
        return None;
    };
    if class_id.as_slice() != b"Ptrn" {
        return None;
    }
    let pattern_id = match desc_item(ptrn, b"Idnt") {
        Some(DescValue::Text(id)) => id.trim_end_matches('\0').to_string(),
        _ => return None,
    };
    let scale = match desc_item(obj, b"Scl ") {
        Some(DescValue::Double(v)) => *v as f32,
        Some(DescValue::UnitFloat { value, .. }) => *value as f32,
        Some(_) => return None,
        None => 100.0,
    };
    if !scale.is_finite() {
        return None;
    }
    let link_with_layer = match link_override {
        Some(linked) => linked,
        None => match desc_item(obj, b"Algn") {
            Some(DescValue::Bool(b)) => *b,
            Some(_) => return None,
            None => true,
        },
    };
    Some(PatternFillParams {
        pattern_id,
        scale,
        link_with_layer,
        origin: decode_origin(obj)?,
    })
}

/// The optional `phase` `Pnt ` object's `Hrzn`/`Vrtc` doubles as an integer
/// pixel origin; absent is `(0, 0)`.
fn decode_origin(obj: &DescValue) -> Option<(i32, i32)> {
    let Some(phase) = desc_item(obj, b"phase") else {
        return Some((0, 0));
    };
    if !matches!(phase, DescValue::Object { .. }) {
        return None;
    }
    let axis = |key: &[u8]| -> Option<i32> {
        match desc_item(phase, key) {
            None => Some(0),
            Some(DescValue::Double(v)) if v.is_finite() => Some(v.round() as i32),
            Some(_) => None,
        }
    };
    Some((axis(b"Hrzn")?, axis(b"Vrtc")?))
}

fn decode_kind(value: &[u8]) -> Option<GradientKind> {
    Some(match value {
        b"Lnr " => GradientKind::Linear,
        b"Rdl " => GradientKind::Radial,
        b"Angl" => GradientKind::Angle,
        b"Rflc" => GradientKind::Reflected,
        b"Dmnd" => GradientKind::Diamond,
        _ => return None,
    })
}

fn kind_enum(kind: GradientKind) -> &'static [u8] {
    match kind {
        GradientKind::Linear => b"Lnr ",
        GradientKind::Radial => b"Rdl ",
        GradientKind::Angle => b"Angl",
        GradientKind::Reflected => b"Rflc",
        GradientKind::Diamond => b"Dmnd",
    }
}

fn decode_stops(value: &DescValue) -> Option<Vec<GradientStop>> {
    let DescValue::List(items) = value else {
        return None;
    };
    if items.len() < 2 {
        return None;
    }
    let mut stops = Vec::with_capacity(items.len());
    let mut previous: Option<u16> = None;
    for item in items {
        if !matches!(item, DescValue::Object { .. }) {
            return None;
        }
        let clr = desc_item(item, b"Clr ")?;
        if !matches!(clr, DescValue::Object { .. }) {
            return None;
        }
        let component = |key: &[u8]| match desc_item(clr, key) {
            Some(DescValue::Double(c)) if c.is_finite() => Some(c.round().clamp(0.0, 255.0) as u8),
            _ => None,
        };
        let color = [
            component(b"Rd  ")?,
            component(b"Grn ")?,
            component(b"Bl  ")?,
        ];
        let location = match desc_item(item, b"Lctn") {
            Some(DescValue::Double(v)) if v.is_finite() && (0.0..=4096.0).contains(v) => {
                v.round() as u16
            }
            Some(DescValue::Long(v)) if (0..=4096).contains(v) => *v as u16,
            _ => return None,
        };
        if previous.is_some_and(|p| location <= p) {
            return None;
        }
        previous = Some(location);
        stops.push(GradientStop { location, color });
    }
    Some(stops)
}

/// Build a standard PSD `GdFl` descriptor with only custom stops.
pub fn encode_gradient_fill(
    kind: GradientKind,
    stops: &[GradientStop],
    angle_deg: f32,
) -> AdjustmentData {
    encode_gradient_fill_params(&GradientFillParams {
        stops: stops.to_vec(),
        reverse: false,
        kind,
        angle_deg,
        scale: 100.0,
    })
}

/// Build a standard PSD `GdFl` descriptor carrying every field the decoder
/// reads. `Rvrs` and `Scl ` are written only when they differ from the decode
/// defaults, so the default output is byte-identical to
/// [`encode_gradient_fill`].
pub fn encode_gradient_fill_params(params: &GradientFillParams) -> AdjustmentData {
    let grad = DescValue::Object {
        name: String::new(),
        class_id: b"Grdn".to_vec(),
        items: vec![
            (b"Nm  ".to_vec(), DescValue::Text("Gradient".into())),
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (
                b"Intr".to_vec(),
                DescValue::Enum {
                    kind: b"Intp".to_vec(),
                    value: b"Lnr ".to_vec(),
                },
            ),
            (
                b"Clrs".to_vec(),
                DescValue::List(params.stops.iter().map(encode_stop).collect()),
            ),
        ],
    };
    let mut items = vec![
        (b"Angl".to_vec(), DescValue::Double(params.angle_deg as f64)),
        (
            b"Type".to_vec(),
            DescValue::Enum {
                kind: b"GrdT".to_vec(),
                value: kind_enum(params.kind).to_vec(),
            },
        ),
        (b"Grad".to_vec(), grad),
    ];
    if params.reverse {
        items.push((b"Rvrs".to_vec(), DescValue::Bool(true)));
    }
    if params.scale != 100.0 {
        items.push((b"Scl ".to_vec(), DescValue::Double(params.scale as f64)));
    }
    let desc = DescValue::Object {
        name: String::new(),
        class_id: b"GdFl".to_vec(),
        items,
    };
    AdjustmentData {
        key: *b"GdFl",
        data: pictura_codec::write_descriptor(&desc).into(),
    }
}

/// Build a standard PSD `PtFl` descriptor from `params`. The `phase` point is
/// written only for a non-zero origin, matching the decoder's default.
pub fn encode_pattern_fill(params: &PatternFillParams) -> AdjustmentData {
    let ptrn = DescValue::Object {
        name: String::new(),
        class_id: b"Ptrn".to_vec(),
        items: vec![(
            b"Idnt".to_vec(),
            DescValue::Text(format!("{}\0", params.pattern_id)),
        )],
    };
    let mut items = vec![
        (b"Ptrn".to_vec(), ptrn),
        (b"Scl ".to_vec(), DescValue::Double(params.scale as f64)),
        (b"Algn".to_vec(), DescValue::Bool(params.link_with_layer)),
    ];
    if params.origin != (0, 0) {
        items.push((
            b"phase".to_vec(),
            DescValue::Object {
                name: String::new(),
                class_id: b"Pnt ".to_vec(),
                items: vec![
                    (b"Hrzn".to_vec(), DescValue::Double(params.origin.0 as f64)),
                    (b"Vrtc".to_vec(), DescValue::Double(params.origin.1 as f64)),
                ],
            },
        ));
    }
    AdjustmentData {
        key: *b"PtFl",
        data: pictura_codec::write_descriptor(&DescValue::Object {
            name: String::new(),
            class_id: b"PtFl".to_vec(),
            items,
        })
        .into(),
    }
}

fn encode_stop(stop: &GradientStop) -> DescValue {
    let clr = DescValue::Object {
        name: String::new(),
        class_id: b"RGBC".to_vec(),
        items: [b"Rd  ", b"Grn ", b"Bl  "]
            .into_iter()
            .zip(stop.color)
            .map(|(key, c)| (key.to_vec(), DescValue::Double(c as f64)))
            .collect(),
    };
    DescValue::Object {
        name: String::new(),
        class_id: b"Clrt".to_vec(),
        items: vec![
            (b"Clr ".to_vec(), clr),
            (
                b"Type".to_vec(),
                DescValue::Enum {
                    kind: b"Clry".to_vec(),
                    value: b"UsrS".to_vec(),
                },
            ),
            (b"Lctn".to_vec(), DescValue::Double(stop.location as f64)),
            (b"Mdpn".to_vec(), DescValue::Double(50.0)),
        ],
    }
}

/// Generate the opaque RGBA gradient over a `w × h` layer rect (row-major).
///
/// Mirrors psd-tools' `draw_gradient_fill` coordinate convention and the five
/// kind geometries; alpha is always 255.
pub fn gradient_rgba(w: i32, h: i32, params: &GradientFillParams) -> Vec<[u8; 4]> {
    gradient_rgba_region(w, h, (0, 0, w.max(0), h.max(0)), params)
}

/// Generate the same gradient as [`gradient_rgba`] for only the local
/// sub-rectangle `region`, in `w × h` coordinates. The full extent still sets
/// the geometry, so every sample matches the full buffer's slice.
pub fn gradient_rgba_region(
    w: i32,
    h: i32,
    region: (i32, i32, i32, i32),
    params: &GradientFillParams,
) -> Vec<[u8; 4]> {
    let (sx0, sy0, sx1, sy1) = region;
    let sw = (sx1 - sx0).max(0) as usize;
    let sh = (sy1 - sy0).max(0) as usize;
    let mut out = vec![[0u8, 0, 0, 255]; sw * sh];
    if w <= 0 || h <= 0 || sw == 0 || sh == 0 {
        return out;
    }
    let wf = w as f64;
    let hf = h as f64;
    let angle = params.angle_deg as f64;
    let ratio = angle.rem_euclid(90.0);
    let s = (params.scale as f64 / 100.0) * ((90.0 - ratio) / 90.0 * wf + (ratio / 90.0) * hf);
    if !s.is_finite() || s <= 0.0 {
        return out;
    }
    let theta = angle.rem_euclid(360.0).to_radians();
    let (sin_t, cos_t) = (theta.sin(), theta.cos());
    let x0 = -wf / s;
    let x_step = if w > 1 {
        2.0 * wf / s / (wf - 1.0)
    } else {
        0.0
    };
    let y0 = -hf / s;
    let y_step = if h > 1 {
        2.0 * hf / s / (hf - 1.0)
    } else {
        0.0
    };
    for j in sy0..sy1 {
        let y = y0 + j as f64 * y_step;
        for i in sx0..sx1 {
            let x = x0 + i as f64 * x_step;
            let mut z = match params.kind {
                GradientKind::Linear => 0.5 * (cos_t * x - sin_t * y + 1.0),
                GradientKind::Radial => (x * x + y * y).sqrt(),
                GradientKind::Angle => {
                    let a = (180.0 * y.atan2(x) / std::f64::consts::PI) + angle;
                    a.rem_euclid(360.0) / 360.0
                }
                GradientKind::Reflected => (cos_t * x - sin_t * y).abs(),
                GradientKind::Diamond => {
                    (cos_t * x - sin_t * y).abs() + (sin_t * x + cos_t * y).abs()
                }
            };
            z = z.clamp(0.0, 1.0);
            if params.reverse {
                z = 1.0 - z;
            }
            out[(j - sy0) as usize * sw + (i - sx0) as usize] = sample_stops(&params.stops, z);
        }
    }
    out
}

/// Linearly sample `stops` at index `z ∈ [0, 1]` (`z * 4096`), clamping outside
/// the stop range. Stops are strictly increasing in location.
fn sample_stops(stops: &[GradientStop], z: f64) -> [u8; 4] {
    let Some(first) = stops.first() else {
        return [0, 0, 0, 255];
    };
    let last = stops.last().unwrap_or(first);
    let pos = z * 4096.0;
    if stops.len() == 1 || pos <= first.location as f64 {
        return [first.color[0], first.color[1], first.color[2], 255];
    }
    if pos >= last.location as f64 {
        return [last.color[0], last.color[1], last.color[2], 255];
    }
    for pair in stops.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if pos >= a.location as f64 && pos <= b.location as f64 {
            let span = (b.location - a.location) as f64;
            let t = if span <= 0.0 {
                0.0
            } else {
                (pos - a.location as f64) / span
            };
            let lerp = |i: usize| {
                (a.color[i] as f64 + (b.color[i] as f64 - a.color[i] as f64) * t)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            return [lerp(0), lerp(1), lerp(2), 255];
        }
    }
    [last.color[0], last.color[1], last.color[2], 255]
}

/// Composite a solid fill across the layer's rect (clamped to the canvas).
///
/// Unlike a destructive adjustment, every in-rect pixel is source content at
/// the payload's alpha; the layer's mask, opacity, fill, and blend still apply
/// through [`blend_into_ctx`]. Pixels outside the rect are untouched.
pub(crate) fn composite_solid_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    rgba: [u8; 4],
) {
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let cs = [
        rgba[0] as f32 / 255.0,
        rgba[1] as f32 / 255.0,
        rgba[2] as f32 / 255.0,
    ];
    let src_a = rgba[3] as f32 / 255.0;
    let clip = canvas.clip();
    let ctx = crate::composite_blend::BlendCtx::new(layer, doc, clip);
    for y in y0..y1 {
        for x in x0..x1 {
            blend_into_ctx(
                canvas, &ctx, layer, doc, clip, x as usize, y as usize, cs, src_a,
            );
        }
    }
}

/// Composite a gradient fill across the layer's rect (clamped to the canvas),
/// generating an opaque colour per pixel and blending it through [`blend_into_ctx`].
pub(crate) fn composite_gradient_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    params: &GradientFillParams,
) {
    let w = layer.rect.width();
    let h = layer.rect.height();
    if w <= 0 || h <= 0 {
        return;
    }
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let px = gradient_rgba(w, h, params);
    let row = w as usize;
    let clip = canvas.clip();
    let ctx = crate::composite_blend::BlendCtx::new(layer, doc, clip);
    for y in y0..y1 {
        for x in x0..x1 {
            let li = (y - layer.rect.top) as usize * row + (x - layer.rect.left) as usize;
            let c = px[li];
            blend_into_ctx(
                canvas,
                &ctx,
                layer,
                doc,
                clip,
                x as usize,
                y as usize,
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ],
                1.0,
            );
        }
    }
}

/// Fallback when a pattern id is not in the document's decoded pattern library:
/// a missing pattern, or one the decoder skipped (a non-RGB mode, a non-8-bit
/// plane, or a malformed rectangle). It composites an opaque 50 %-grey tile so
/// the layer is visible rather than a no-op. The docs describe a
/// placeholder-or-last-known plus a warning; this uses the grey placeholder and
/// has no warning surface.
/// ponytail: no warning surface in the render crate, and no last-known pattern
/// retained; add them when one exists. Ceiling, not an intended parity claim.
const PATTERN_PLACEHOLDER: [u8; 4] = [128, 128, 128, 255];

/// A tile sampler over a decoded pattern (or the placeholder).
///
/// The tile is `tw × th` pixels; the source is sampled nearest-neighbour, which
/// is exact at scale 100.
///
/// ponytail: scale != 100 uses nearest-neighbour rather than CS6's resample
/// filter; non-RGB pattern modes and 16/32-bit planes never reach here (the
/// codec skips them, so the placeholder renders); `phase`/origin is inferred
/// from the layer-style convention and unverified for `PtFl`.
struct Tile<'a> {
    rgba: &'a [u8],
    pw: i64,
    ph: i64,
    tw: i64,
    th: i64,
    link: bool,
    origin: (i32, i32),
}

impl Tile<'_> {
    /// The pattern pixel for canvas pixel `(cx, cy)` of a rect at
    /// `(rect_left, rect_top)`.
    fn sample(&self, cx: i32, cy: i32, rect_left: i32, rect_top: i32) -> [u8; 4] {
        let sx_base = if self.link { cx - rect_left } else { cx } as i64;
        let sy_base = if self.link { cy - rect_top } else { cy } as i64;
        let sx = (sx_base - self.origin.0 as i64).rem_euclid(self.tw);
        let sy = (sy_base - self.origin.1 as i64).rem_euclid(self.th);
        let px = ((sx as f64 / self.tw as f64) * self.pw as f64).floor() as i64;
        let py = ((sy as f64 / self.th as f64) * self.ph as f64).floor() as i64;
        let px = px.clamp(0, self.pw - 1) as usize;
        let py = py.clamp(0, self.ph - 1) as usize;
        let i = (py * self.pw as usize + px) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

/// Build the tile for `params`, resolving the pattern in the document's decoded
/// pattern set (the placeholder when absent).
fn tile_for<'a>(pattern: Option<&'a PatternPixels>, params: &PatternFillParams) -> Tile<'a> {
    let pattern = pattern.filter(|pattern| {
        if pattern.width == 0 || pattern.height == 0 {
            return false;
        }
        let pixels = (pattern.width as usize).saturating_mul(pattern.height as usize);
        pattern.rgba.len() >= pixels.saturating_mul(4)
    });
    let Some(pattern) = pattern else {
        return Tile {
            rgba: &PATTERN_PLACEHOLDER,
            pw: 1,
            ph: 1,
            tw: 1,
            th: 1,
            link: params.link_with_layer,
            origin: params.origin,
        };
    };
    let scale = if params.scale.is_finite() {
        params.scale as f64
    } else {
        100.0
    };
    let tw = (pattern.width as f64 * scale / 100.0).round().max(1.0) as i64;
    let th = (pattern.height as f64 * scale / 100.0).round().max(1.0) as i64;
    Tile {
        rgba: &pattern.rgba,
        pw: pattern.width as i64,
        ph: pattern.height as i64,
        tw: tw.max(1),
        th: th.max(1),
        link: params.link_with_layer,
        origin: params.origin,
    }
}

/// Composite a pattern fill across the layer's rect (clamped to the canvas),
/// tiling through [`blend_into_ctx`] so the layer's mask, opacity, fill, and blend
/// apply and pixels outside the rect are untouched.
pub(crate) fn composite_pattern_fill(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    params: &PatternFillParams,
) {
    let x0 = layer.rect.left.max(canvas.x0());
    let y0 = layer.rect.top.max(canvas.y0());
    let x1 = layer.rect.right.min(canvas.x1());
    let y1 = layer.rect.bottom.min(canvas.y1());
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // ponytail: the pattern library is decoded once per pattern-fill layer;
    // thread a shared decode through the composite if re-parsing ever shows up.
    let patterns = pictura_codec::decode_patterns(doc);
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let clip = canvas.clip();
    let ctx = crate::composite_blend::BlendCtx::new(layer, doc, clip);
    for y in y0..y1 {
        for x in x0..x1 {
            let c = tile.sample(x, y, layer.rect.left, layer.rect.top);
            blend_into_ctx(
                canvas,
                &ctx,
                layer,
                doc,
                clip,
                x as usize,
                y as usize,
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ],
                c[3] as f32 / 255.0,
            );
        }
    }
}

/// Bake the tiled pattern (or placeholder) for an arbitrary canvas `region`
/// into row-major RGBA. Shares [`Tile`] with the compositor; the caller decodes
/// the pattern library once and passes the anchor the tile is linked to.
pub(crate) fn pattern_tile_region(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    anchor: (i32, i32),
    region: (i32, i32, i32, i32),
) -> Vec<[u8; 4]> {
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let (x0, y0, x1, y1) = region;
    let mut out = Vec::with_capacity(((x1 - x0).max(0) * (y1 - y0).max(0)) as usize);
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(tile.sample(x, y, anchor.0, anchor.1));
        }
    }
    out
}

/// Bake the tiled pattern (or placeholder) for a rect into row-major RGBA, for
/// fill-content rasterization. A thin wrapper over [`pattern_tile_region`]
/// anchored to the rect's top-left.
pub(crate) fn pattern_tile_rgba(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    rect_left: i32,
    rect_top: i32,
    w: i32,
    h: i32,
) -> Vec<[u8; 4]> {
    pattern_tile_region(
        patterns,
        params,
        (rect_left, rect_top),
        (
            rect_left,
            rect_top,
            rect_left + w.max(0),
            rect_top + h.max(0),
        ),
    )
}

/// A `width`x`height` preview of a pattern fill's layer rect, sampling the tile
/// at the scaled document position. Nearest-neighbour; used by the Layers
/// panel thumbnail so a pattern reads at any thumbnail size.
pub fn pattern_preview_rgba(
    patterns: &[PatternPixels],
    params: &PatternFillParams,
    rect: pictura_core::PsdRect,
    width: u32,
    height: u32,
) -> Vec<[u8; 4]> {
    let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
    let tile = tile_for(pattern, params);
    let mut out = Vec::with_capacity((width as usize) * (height as usize));
    let sx = rect.width().max(1) as f64 / width.max(1) as f64;
    let sy = rect.height().max(1) as f64 / height.max(1) as f64;
    for y in 0..height {
        for x in 0..width {
            let px = rect.left + (x as f64 * sx) as i32;
            let py = rect.top + (y as f64 * sy) as i32;
            out.push(tile.sample(px, py, rect.left, rect.top));
        }
    }
    out
}

/// Paint a fill layer's coverage alpha into a region `(x0, y0, x1, y1)` of the
/// canvas (row-major `(x1-x0) × (y1-y0)`), zero outside the layer rect. Returns
/// `false` when `layer` is not a decoded fill, so the caller falls back to the
/// pixel/smart path.
///
/// ponytail: the pattern tile is sampled nearest-neighbour, matching the
/// compositor's [`Tile`].
pub(crate) fn fill_coverage_matte(
    layer: &Layer,
    doc: &Document,
    region: (i32, i32, i32, i32),
    out: &mut [f32],
) -> bool {
    let Some(adjustment) = decode_layer_fill(layer) else {
        return false;
    };
    let (rx0, ry0, rx1, ry1) = region;
    let rw = (rx1 - rx0) as usize;
    let ix0 = layer.rect.left.max(rx0);
    let iy0 = layer.rect.top.max(ry0);
    let ix1 = layer.rect.right.min(rx1);
    let iy1 = layer.rect.bottom.min(ry1);
    if ix1 <= ix0 || iy1 <= iy0 {
        return true;
    }
    let mut put = |x: i32, y: i32, value: f32| {
        out[(y - ry0) as usize * rw + (x - rx0) as usize] = value;
    };
    match adjustment {
        Adjustment::SolidFill(rgba) => {
            let alpha = rgba[3] as f32 / 255.0;
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    put(x, y, alpha);
                }
            }
            true
        }
        Adjustment::GradientFill(_) => {
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    put(x, y, 1.0);
                }
            }
            true
        }
        Adjustment::PatternFill(params) => {
            let patterns = pictura_codec::decode_patterns(doc);
            let pattern = patterns.iter().find(|p| p.pattern_id == params.pattern_id);
            let tile = tile_for(pattern, &params);
            for y in iy0..iy1 {
                for x in ix0..ix1 {
                    let c = tile.sample(x, y, layer.rect.left, layer.rect.top);
                    put(x, y, c[3] as f32 / 255.0);
                }
            }
            true
        }
        _ => false,
    }
}

/// Rasterize a fill layer's content into the straight-alpha RGBA buffer the GPU
/// uploads, or `None` when the layer is not a decoded fill.
pub(crate) fn fill_source(
    layer: &Layer,
    doc: &Document,
) -> Option<crate::layer_source::CachedBuffer> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::Arc;

    let adjustment = decode_layer_fill(layer)?;
    let full_rect = layer.rect;
    let (full_w, full_h) = (full_rect.width(), full_rect.height());
    let rect = crate::layer_source::clip_to_document(full_rect, doc);
    let (w, h) = (rect.width(), rect.height());
    // A fill with no on-canvas area is the CPU's no-op; keep it on the GPU as an
    // empty source rather than declining the whole document.
    let (buffer, retain) = if w <= 0 || h <= 0 || full_w <= 0 || full_h <= 0 {
        (PixelBuffer::new(0, 0, 4), false)
    } else {
        match &adjustment {
            Adjustment::SolidFill(rgba) => {
                let mut buf = PixelBuffer::new(w as u32, h as u32, 4);
                let n = buf.pixel_count();
                for i in 0..n {
                    buf.data[i] = rgba[0];
                    buf.data[n + i] = rgba[1];
                    buf.data[2 * n + i] = rgba[2];
                    buf.data[3 * n + i] = rgba[3];
                }
                (buf, true)
            }
            Adjustment::GradientFill(params) => {
                let px = gradient_rgba_region(
                    full_w,
                    full_h,
                    (
                        rect.left - full_rect.left,
                        rect.top - full_rect.top,
                        rect.right - full_rect.left,
                        rect.bottom - full_rect.top,
                    ),
                    params,
                );
                (rgba_to_planar(&px, w, h), true)
            }
            Adjustment::PatternFill(params) => {
                let patterns = pictura_codec::decode_patterns(doc);
                let px = pattern_tile_region(
                    &patterns,
                    params,
                    (full_rect.left, full_rect.top),
                    (rect.left, rect.top, rect.right, rect.bottom),
                );
                (rgba_to_planar(&px, w, h), false)
            }
            _ => return None,
        }
    };
    let mut hasher = DefaultHasher::new();
    [
        full_rect.left,
        full_rect.top,
        full_rect.right,
        full_rect.bottom,
    ]
    .hash(&mut hasher);
    doc.width.hash(&mut hasher);
    doc.height.hash(&mut hasher);
    doc.mode.hash(&mut hasher);
    if let Some(data) = layer.adjustment.as_ref() {
        data.key.hash(&mut hasher);
        data.data.hash(&mut hasher);
    }
    if let Some(block) = layer.extra_block(b"vscg") {
        block.data.hash(&mut hasher);
    }
    Some(crate::layer_source::CachedBuffer {
        rect,
        buffer: Arc::new(buffer),
        content: hasher.finish(),
        retain,
    })
}

fn rgba_to_planar(px: &[[u8; 4]], w: i32, h: i32) -> PixelBuffer {
    let n = (w.max(0) as usize) * (h.max(0) as usize);
    let mut buf = PixelBuffer::new(w.max(0) as u32, h.max(0) as u32, 4);
    for (i, c) in px.iter().take(n).enumerate() {
        buf.data[i] = c[0];
        buf.data[n + i] = c[1];
        buf.data[2 * n + i] = c[2];
        buf.data[3 * n + i] = c[3];
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_adjust::Adjustment;

    #[test]
    fn pattern_fill_upload_is_not_retained() {
        let bytes = include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd");
        let doc = pictura_codec::read_psd(bytes).expect("fixture parses");
        let layer = doc
            .layers
            .iter()
            .find(|layer| matches!(decode_layer_fill(layer), Some(Adjustment::PatternFill(_))))
            .expect("a pattern fill layer");
        let cached = fill_source(layer, &doc).expect("the fill rasterizes");
        assert!(!cached.retain, "a pattern fill is not retained");
    }

    #[test]
    fn all_fill_fixtures_rasterize() {
        for (name, bytes) in [
            (
                "solid",
                include_bytes!("../../pictura-codec/tests/fixtures/solid_fill.psd").as_slice(),
            ),
            (
                "gradient",
                include_bytes!("../../pictura-codec/tests/fixtures/gradient_fill.psd").as_slice(),
            ),
            (
                "pattern",
                include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd").as_slice(),
            ),
            (
                "vector",
                include_bytes!("../../pictura-codec/tests/fixtures/vector_fill.psd").as_slice(),
            ),
        ] {
            let doc = pictura_codec::read_psd(bytes).expect("fixture parses");
            for layer in doc.layers.iter() {
                if decode_layer_fill(layer).is_some() {
                    assert!(fill_source(layer, &doc).is_some(), "{name}: {}", layer.name);
                }
            }
        }
    }

    #[test]
    fn changing_the_pattern_content_changes_the_raster() {
        let bytes = include_bytes!("../../pictura-codec/tests/fixtures/pattern_fill.psd");
        let mut doc = pictura_codec::read_psd(bytes).expect("fixture parses");
        let mut layer = doc
            .layers
            .iter()
            .find(|layer| matches!(decode_layer_fill(layer), Some(Adjustment::PatternFill(_))))
            .expect("a pattern fill layer")
            .clone();
        let Some(Adjustment::PatternFill(mut params)) = decode_layer_fill(&layer) else {
            unreachable!()
        };
        let first = fill_source(&layer, &doc).expect("first raster");
        params.pattern_id = "pictura-changed".into();
        layer.adjustment = Some(encode_pattern_fill(&params));
        let record = pictura_codec::encode_rgb_pattern(
            "changed",
            "pictura-changed",
            4,
            4,
            &[255u8; 4 * 4 * 4],
        )
        .expect("pattern encodes");
        assert!(pictura_codec::add_document_pattern(
            &mut doc,
            "pictura-changed",
            &record
        ));
        let second = fill_source(&layer, &doc).expect("second raster");
        assert_ne!(first.content, second.content);
        assert_ne!(first.buffer.data.as_ref(), second.buffer.data.as_ref());
    }
}
