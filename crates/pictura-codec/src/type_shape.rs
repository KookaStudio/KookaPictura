//! The EngineData `/Rendered` shape: whether a type layer is point type
//! (`ShapeType 0`) or paragraph type (`ShapeType 1` with its `BoxBounds`).
//!
//! Photoshop keys box text on `/EngineDict /Rendered /Shapes /Children[0]
//! /Cookie /Photoshop`; without it a box layer reopens as point type. The
//! line cache (`/Lines`) is written empty, for Photoshop to rebuild.

use crate::engine_data::{as_list, get, path, EngineValue};

fn dict(items: Vec<(&str, EngineValue)>) -> EngineValue {
    EngineValue::Dict(items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn doubles(values: &[f64]) -> EngineValue {
    EngineValue::List(values.iter().map(|v| EngineValue::Double(*v)).collect())
}

/// The `/Rendered` value for point type (`None`) or a paragraph box (left,
/// top, right, bottom relative to the origin).
pub(crate) fn rendered(box_bounds: Option<[f64; 4]>) -> EngineValue {
    let shape_type = i64::from(box_bounds.is_some());
    let mut photoshop = vec![("ShapeType", EngineValue::Int(shape_type))];
    match box_bounds {
        Some(b) => photoshop.push(("BoxBounds", doubles(&b))),
        None => photoshop.push(("PointBase", doubles(&[0.0, 0.0]))),
    }
    photoshop.push((
        "Base",
        dict(vec![
            ("ShapeType", EngineValue::Int(shape_type)),
            ("TransformPoint0", doubles(&[1.0, 0.0])),
            ("TransformPoint1", doubles(&[0.0, 1.0])),
            ("TransformPoint2", doubles(&[0.0, 0.0])),
        ]),
    ));
    let child = dict(vec![
        ("ShapeType", EngineValue::Int(shape_type)),
        ("Procession", EngineValue::Int(0)),
        (
            "Lines",
            dict(vec![
                ("WritingDirection", EngineValue::Int(0)),
                ("Children", EngineValue::List(Vec::new())),
            ]),
        ),
        ("Cookie", dict(vec![("Photoshop", dict(photoshop))])),
    ]);
    dict(vec![
        ("Version", EngineValue::Int(1)),
        (
            "Shapes",
            dict(vec![
                ("WritingDirection", EngineValue::Int(0)),
                ("Children", EngineValue::List(vec![child])),
            ]),
        ),
    ])
}

fn number(value: &EngineValue) -> Option<f64> {
    match value {
        EngineValue::Double(d) => Some(*d),
        EngineValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}

/// The first shape's `BoxBounds` when it is paragraph type; `None` for point
/// type, a missing shape, or a malformed box.
pub(crate) fn extract_box(root: &EngineValue) -> Option<[f64; 4]> {
    let shape = path(root, &["EngineDict", "Rendered", "Shapes", "Children"])
        .map(as_list)?
        .first()?;
    let photoshop = path(shape, &["Cookie", "Photoshop"])?;
    if get(photoshop, "ShapeType").and_then(number)? != 1.0 {
        return None;
    }
    let values = get(photoshop, "BoxBounds").map(as_list)?;
    if values.len() != 4 {
        return None;
    }
    let mut out = [0.0; 4];
    for (slot, value) in out.iter_mut().zip(values) {
        *slot = number(value).filter(|v| v.is_finite())?;
    }
    (out[2] > out[0] && out[3] > out[1]).then_some(out)
}
