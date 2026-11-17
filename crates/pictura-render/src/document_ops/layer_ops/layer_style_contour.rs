//! Structured layer-style values: the bevel contours and the Gradient Overlay
//! stop list, which do not fit the scalar `"<effect>.<field>"` table.

use pictura_codec::DescValue;
use pictura_core::Layer;

use crate::composite::desc_item;
use crate::layer_effects::contour::Contour;

use super::Effect;
use super::{
    default_effect, effect_named, lfx2_items, object, read_lfx2, rgbc, set_item, styleable,
    write_lfx2,
};

/// The bevel field a contour key maps onto.
fn contour_target(key: &str) -> Option<(&'static Effect, &'static [u8])> {
    let key: &'static [u8] = match key {
        "bevel.glossContour" => b"TrnS",
        "bevel.edgeContour" => b"MpgS",
        _ => return None,
    };
    Some((effect_named("bevel")?, key))
}

/// The contour at `key` as `"x,y x,y …"`, or the linear list when absent.
pub fn layer_style_contour(layer: &Layer, key: &str) -> Option<String> {
    let (effect, contour_key) = contour_target(key)?;
    let top = read_lfx2(layer);
    let contour = top
        .as_ref()
        .and_then(|top| desc_item(top, effect.key))
        .and_then(|obj| desc_item(obj, contour_key))
        .and_then(Contour::decode)
        .unwrap_or_default();
    Some(format_contour(&contour))
}

/// Write `key`'s contour from `"x,y x,y …"`. False for a malformed list, an
/// unknown key, a group, the Background, or an unchanged value.
pub fn set_layer_style_contour(layer: &mut Layer, key: &str, text: &str) -> bool {
    let Some((effect, contour_key)) = contour_target(key) else {
        return false;
    };
    if !styleable(layer) {
        return false;
    }
    let Some(contour) = parse_contour(text).and_then(Contour::from_points) else {
        return false;
    };
    let next_text = format_contour(&contour);
    let stored = read_lfx2(layer)
        .as_ref()
        .and_then(|top| desc_item(top, effect.key))
        .and_then(|obj| desc_item(obj, contour_key))
        .and_then(Contour::decode);
    if stored
        .as_ref()
        .is_some_and(|stored| format_contour(stored) == next_text)
    {
        return false;
    }
    let mut items = lfx2_items(layer);
    let mut obj = items
        .iter()
        .find(|(k, _)| k.as_slice() == effect.key)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| default_effect(effect));
    let DescValue::Object { items: fields, .. } = &mut obj else {
        return false;
    };
    let existing = fields
        .iter()
        .find(|(k, _)| k.as_slice() == contour_key)
        .map(|(_, v)| v.clone());
    let next = match existing {
        Some(existing) if matches!(&existing, DescValue::Object { .. }) => {
            let mut existing = existing;
            apply_contour_points(&mut existing, &contour);
            existing
        }
        _ => contour.descriptor("Custom"),
    };
    set_item(fields, contour_key, next);
    set_item(&mut items, effect.key, obj);
    write_lfx2(layer, items);
    true
}

fn parse_contour(text: &str) -> Option<Vec<(f32, f32)>> {
    let mut points = Vec::new();
    for item in text.split_whitespace() {
        let (x, y) = item.split_once(',')?;
        let x: i32 = x.trim().parse().ok()?;
        let y: i32 = y.trim().parse().ok()?;
        if !(0..=255).contains(&x) || !(0..=255).contains(&y) {
            return None;
        }
        points.push((x as f32, y as f32));
    }
    if points.len() < 2 || !points.windows(2).all(|pair| pair[0].0 < pair[1].0) {
        return None;
    }
    Some(points)
}

fn format_contour(contour: &Contour) -> String {
    contour
        .points()
        .iter()
        .map(|&(x, y)| format!("{},{}", x.round() as i32, y.round() as i32))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Replace the `Hrzn`/`Vrtc` of each new point, reusing the existing `CrPt`
/// object for an unchanged input so its other items survive.
fn apply_contour_points(existing: &mut DescValue, contour: &Contour) {
    let DescValue::Object { items, .. } = existing else {
        return;
    };
    let mut old = match items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Crv ")
        .map(|(_, v)| v)
    {
        Some(DescValue::List(list)) => list.clone(),
        _ => Vec::new(),
    };
    let mut points = Vec::with_capacity(contour.points().len());
    for &(x, y) in contour.points() {
        let index = old
            .iter()
            .position(|point| match desc_item(point, b"Hrzn") {
                Some(DescValue::Double(v)) => v.round() as i32 == x.round() as i32,
                Some(DescValue::Long(v)) => *v == x.round() as i32,
                Some(DescValue::UnitFloat { value, .. }) => {
                    value.round() as i32 == x.round() as i32
                }
                _ => false,
            });
        let mut point = index
            .map(|i| old.remove(i))
            .unwrap_or_else(|| DescValue::Object {
                name: String::new(),
                class_id: b"CrPt".to_vec(),
                items: Vec::new(),
            });
        if let DescValue::Object { items, .. } = &mut point {
            set_item(items, b"Hrzn", DescValue::Double(f64::from(x)));
            set_item(items, b"Vrtc", DescValue::Double(f64::from(y)));
        }
        points.push(point);
    }
    set_item(items, b"Crv ", DescValue::List(points));
}

/// The Gradient Overlay field a gradient key maps onto.
fn gradient_target(key: &str) -> Option<&'static Effect> {
    (key == "gradientOverlay.gradient")
        .then(|| effect_named("gradientOverlay"))
        .flatten()
}

/// The Gradient Overlay's colour stops as `"loc:rrggbb …"`.
pub fn layer_style_gradient(layer: &Layer, key: &str) -> Option<String> {
    let effect = gradient_target(key)?;
    let top = read_lfx2(layer);
    let stops = top
        .as_ref()
        .and_then(|top| desc_item(top, effect.key))
        .and_then(|obj| desc_item(obj, b"Grad"))
        .and_then(gradient_stops)
        .unwrap_or_else(default_stops);
    Some(format_stops(&stops))
}

/// Write `key`'s colour stops from `"loc:rrggbb …"`, preserving `Trns`, `GrdF`,
/// `Intr` and the midpoint of every stop whose location is unchanged.
pub fn set_layer_style_gradient(layer: &mut Layer, key: &str, text: &str) -> bool {
    let Some(effect) = gradient_target(key) else {
        return false;
    };
    if !styleable(layer) {
        return false;
    }
    let Some(stops) = parse_stops(text) else {
        return false;
    };
    let next_text = format_stops(&stops);
    let stored = read_lfx2(layer)
        .as_ref()
        .and_then(|top| desc_item(top, effect.key))
        .and_then(|obj| desc_item(obj, b"Grad"))
        .and_then(gradient_stops);
    if stored
        .as_ref()
        .is_some_and(|stored| format_stops(stored) == next_text)
    {
        return false;
    }
    let mut items = lfx2_items(layer);
    let mut obj = items
        .iter()
        .find(|(k, _)| k.as_slice() == effect.key)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| default_effect(effect));
    let DescValue::Object { items: fields, .. } = &mut obj else {
        return false;
    };
    let existing = fields
        .iter()
        .find(|(k, _)| k.as_slice() == b"Grad")
        .map(|(_, v)| v.clone());
    let grad = match existing {
        Some(existing) if matches!(&existing, DescValue::Object { .. }) => {
            let mut existing = existing;
            apply_gradient_stops(&mut existing, &stops);
            existing
        }
        _ => gradient_descriptor(&stops),
    };
    set_item(fields, b"Grad", grad);
    set_item(&mut items, effect.key, obj);
    write_lfx2(layer, items);
    true
}

fn default_stops() -> Vec<(u16, [u8; 3])> {
    vec![(0, [0, 0, 0]), (4096, [255, 255, 255])]
}

fn gradient_stops(grad: &DescValue) -> Option<Vec<(u16, [u8; 3])>> {
    let DescValue::List(items) = desc_item(grad, b"Clrs")? else {
        return None;
    };
    let mut stops = Vec::with_capacity(items.len());
    let mut previous: Option<u16> = None;
    for item in items {
        let location = match desc_item(item, b"Lctn")? {
            DescValue::Double(v) if v.is_finite() && (0.0..=4096.0).contains(v) => v.round() as u16,
            DescValue::Long(v) if (0..=4096).contains(v) => *v as u16,
            _ => return None,
        };
        if previous.is_some_and(|p| location <= p) {
            return None;
        }
        previous = Some(location);
        let color = desc_item(item, b"Clr ")?;
        let component = |key: &[u8]| match desc_item(color, key) {
            Some(DescValue::Double(v)) if v.is_finite() => Some(v.round().clamp(0.0, 255.0) as u8),
            Some(DescValue::UnitFloat { value, .. }) if value.is_finite() => {
                Some(value.round().clamp(0.0, 255.0) as u8)
            }
            _ => None,
        };
        stops.push((
            location,
            [
                component(b"Rd  ")?,
                component(b"Grn ")?,
                component(b"Bl  ")?,
            ],
        ));
    }
    (stops.len() >= 2).then_some(stops)
}

fn parse_stops(text: &str) -> Option<Vec<(u16, [u8; 3])>> {
    let mut stops = Vec::new();
    for item in text.split_whitespace() {
        let (location, hex) = item.split_once(':')?;
        let location: u16 = location.parse().ok()?;
        if location > 4096 || hex.len() != 6 {
            return None;
        }
        let value = u32::from_str_radix(hex, 16).ok()?;
        stops.push((
            location,
            [(value >> 16) as u8, (value >> 8) as u8, value as u8],
        ));
    }
    if stops.len() < 2 || !stops.windows(2).all(|pair| pair[0].0 < pair[1].0) {
        return None;
    }
    Some(stops)
}

fn format_stops(stops: &[(u16, [u8; 3])]) -> String {
    stops
        .iter()
        .map(|(location, color)| {
            format!(
                "{}:{:02x}{:02x}{:02x}",
                location, color[0], color[1], color[2]
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Replace only `Clrs`; the existing `Clrt` object is reused for an unchanged
/// location so its `Mdpn` and `Type` survive.
fn apply_gradient_stops(grad: &mut DescValue, stops: &[(u16, [u8; 3])]) {
    let DescValue::Object { items, .. } = grad else {
        return;
    };
    let mut old = match items
        .iter()
        .find(|(k, _)| k.as_slice() == b"Clrs")
        .map(|(_, v)| v)
    {
        Some(DescValue::List(list)) => list.clone(),
        _ => Vec::new(),
    };
    let mut list = Vec::with_capacity(stops.len());
    for &(location, color) in stops {
        let index = old.iter().position(|stop| match desc_item(stop, b"Lctn") {
            Some(DescValue::Double(v)) => v.round() as i32 == location as i32,
            Some(DescValue::Long(v)) => *v == location as i32,
            _ => false,
        });
        let mut stop = index.map(|i| old.remove(i)).unwrap_or_else(|| {
            object(
                b"Clrt",
                vec![
                    (b"Clr ".to_vec(), rgbc(0.0)),
                    (
                        b"Type".to_vec(),
                        DescValue::Enum {
                            kind: b"Clry".to_vec(),
                            value: b"UsrS".to_vec(),
                        },
                    ),
                    (b"Lctn".to_vec(), DescValue::Long(location as i32)),
                    (b"Mdpn".to_vec(), DescValue::Long(50)),
                ],
            )
        });
        if let DescValue::Object { items, .. } = &mut stop {
            set_item(items, b"Clr ", rgbc(packed(color)));
            set_item(items, b"Lctn", DescValue::Long(location as i32));
        }
        list.push(stop);
    }
    set_item(items, b"Clrs", DescValue::List(list));
}

fn gradient_descriptor(stops: &[(u16, [u8; 3])]) -> DescValue {
    let clrt = |location: u16, color: [u8; 3]| {
        object(
            b"Clrt",
            vec![
                (b"Clr ".to_vec(), rgbc(packed(color))),
                (
                    b"Type".to_vec(),
                    DescValue::Enum {
                        kind: b"Clry".to_vec(),
                        value: b"UsrS".to_vec(),
                    },
                ),
                (b"Lctn".to_vec(), DescValue::Long(location as i32)),
                (b"Mdpn".to_vec(), DescValue::Long(50)),
            ],
        )
    };
    let opacity = |location: i32| {
        object(
            b"TrnS",
            vec![
                (
                    b"Opct".to_vec(),
                    DescValue::UnitFloat {
                        unit: *b"#Prc",
                        value: 100.0,
                    },
                ),
                (b"Lctn".to_vec(), DescValue::Long(location)),
                (b"Mdpn".to_vec(), DescValue::Long(50)),
            ],
        )
    };
    object(
        b"Grdn",
        vec![
            (b"Nm  ".to_vec(), DescValue::Text("Custom".into())),
            (
                b"GrdF".to_vec(),
                DescValue::Enum {
                    kind: b"GrdF".to_vec(),
                    value: b"CstS".to_vec(),
                },
            ),
            (b"Intr".to_vec(), DescValue::Double(4096.0)),
            (
                b"Clrs".to_vec(),
                DescValue::List(stops.iter().map(|&(l, c)| clrt(l, c)).collect()),
            ),
            (
                b"Trns".to_vec(),
                DescValue::List(vec![opacity(0), opacity(4096)]),
            ),
        ],
    )
}

fn packed(color: [u8; 3]) -> f64 {
    f64::from((u32::from(color[0]) << 16) | (u32::from(color[1]) << 8) | u32::from(color[2]))
}
