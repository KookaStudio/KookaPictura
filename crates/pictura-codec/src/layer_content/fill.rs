use pictura_core::{AdjustmentData, FillContent, GradientFillParams, GradientKind, GradientStop, PatternFillParams};
use crate::DescValue;
use crate::layer_content_budget::ContentAlloc;
use super::basic::desc_item;

pub fn decode_gradient_fill(d: &[u8]) -> Option<FillContent> {
    let obj = crate::read_descriptor(d).ok()?;
    gradient_params_from_desc(&obj).map(FillContent::Gradient)
}

/// The gradient fields of an already-read `GdFl`/`GrFl` object. Shared by the
/// public fill decoder and the gradient-overlay decoder. A `doub` or a unit
/// float is accepted for the numerics (a `GdFl` carries `doub`, an effect
/// `GrFl` carries unit floats); a finite `f64` that overflows `f32` rejects.
pub fn gradient_params_from_desc(obj: &DescValue) -> Option<GradientFillParams> {
    gradient_params_from_desc_using(obj, &mut ContentAlloc::new(None))
}

pub(crate) fn gradient_params_from_desc_using(obj: &DescValue, alloc: &mut ContentAlloc) -> Option<GradientFillParams> {
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
    let stops = decode_stops(desc_item(grad, b"Clrs")?, alloc)?;
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
pub fn decode_pattern_fill(d: &[u8]) -> Option<FillContent> {
    let obj = crate::read_descriptor(d).ok()?;
    Some(FillContent::Pattern(pattern_params_from_desc(&obj)?))
}

/// The pattern fields of an already-read `PtFl`/`patternFill` object. Shared by
/// the public fill decoder and the pattern-overlay decoder (a pure move; the
/// strict `PtFl` contract is unchanged).
pub fn pattern_params_from_desc(obj: &DescValue) -> Option<PatternFillParams> {
    pattern_params_from_desc_with_link(obj, None)
}

/// [`pattern_params_from_desc`] with the link flag forced when `Some`.
///
/// The overlay pattern links via `Algn`; the stroke pattern links via `Lnkd`
/// (psd-tools `_PatternMixin`), so the stroke passes its decoded `Lnkd` here.
/// Forcing it bypasses the helper's `Algn` requirement, so a valid `Lnkd` wins
/// even when a present-but-wrongly-typed `Algn` would otherwise reject.
pub fn pattern_params_from_desc_with_link(
    obj: &DescValue,
    link_override: Option<bool>,
) -> Option<PatternFillParams> {
    pattern_params_from_desc_using(obj, link_override, &mut ContentAlloc::new(None))
}

pub(crate) fn pattern_params_from_desc_using(
    obj: &DescValue,
    link_override: Option<bool>,
    alloc: &mut ContentAlloc,
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
        Some(DescValue::Text(id)) => alloc.string(id.trim_end_matches('\0'), "pattern fill id")?,
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

fn decode_stops(value: &DescValue, alloc: &mut ContentAlloc) -> Option<Vec<GradientStop>> {
    let DescValue::List(items) = value else {
        return None;
    };
    if items.len() < 2 {
        return None;
    }
    let mut stops = alloc.vec(items.len(), "gradient fill stops")?;
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
        data: crate::write_descriptor(&desc).into(),
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
        data: crate::write_descriptor(&DescValue::Object {
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

