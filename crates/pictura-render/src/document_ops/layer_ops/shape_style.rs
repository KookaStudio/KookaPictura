//! A shape layer's appearance and size as the shape tools' options bar edits
//! them: the Fill (a solid colour, or None), the Stroke, and the outline's
//! width and height.
//!
//! Approximations (`ponytail:`): CS6 keeps a vector shape's fill and stroke in
//! a `vstk` block; here Fill None is the layer's Fill opacity at 0 and the
//! Stroke is a solid Stroke layer effect (`lfx2` `FrFX`), which the compositor
//! already draws along the vector mask. Dashed and dotted strokes, and a
//! gradient or pattern fill or stroke, are not modelled.

use pictura_codec::DescValue;
use pictura_core::path::{PathPoint, Subpath, VectorPath};
use pictura_core::Layer;

use super::shape_layer::{
    is_shape_layer, layer_live_shape, layer_shape_paths, set_layer_live_shape,
    set_layer_shape_paths, shape_fill_color,
};
use crate::layer_effects::{decode_stroke, StrokeFill, StrokePosition};

/// A solid stroke: its colour, width in pixels, and where it sits on the
/// outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeStroke {
    pub color: [u8; 3],
    pub width: u32,
    pub position: StrokePosition,
}

/// A shape layer's copyable appearance: the fill (or `None` when unfilled) and
/// the stroke (or `None` when unstroked).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeAttributes {
    pub fill: Option<[u8; 3]>,
    pub stroke: Option<ShapeStroke>,
}

/// The shape layer's copyable fill and stroke; `None` when the layer is not a
/// shape layer.
pub fn copy_shape_attributes(layer: &Layer) -> Option<ShapeAttributes> {
    is_shape_layer(layer).then(|| ShapeAttributes {
        fill: shape_fill(layer),
        stroke: shape_stroke(layer),
    })
}

/// Give the shape layer `attributes`, replacing its fill and stroke. False when
/// the layer is not a shape layer; otherwise whether anything changed.
pub fn paste_shape_attributes(layer: &mut Layer, attributes: &ShapeAttributes) -> bool {
    if !is_shape_layer(layer) {
        return false;
    }
    let fill = set_shape_fill(layer, attributes.fill);
    let stroke = set_shape_stroke(layer, attributes.stroke.as_ref());
    fill | stroke
}

/// The shape layer's fill colour, or `None` when it has no fill (Fill
/// opacity 0) or is not a shape layer.
pub fn shape_fill(layer: &Layer) -> Option<[u8; 3]> {
    let [r, g, b, _] = shape_fill_color(layer)?;
    (layer.fill > 0).then_some([r, g, b])
}

/// Fill the shape layer with `fill`, or (`None`) leave it unfilled. False when
/// the layer is not a shape layer or nothing changed.
pub fn set_shape_fill(layer: &mut Layer, fill: Option<[u8; 3]>) -> bool {
    if !is_shape_layer(layer) || shape_fill(layer) == fill {
        return false;
    }
    match fill {
        Some(color) => {
            layer.adjustment = Some(crate::encode_solid_color_fill(color));
            layer.fill = 255;
        }
        None => layer.fill = 0,
    }
    true
}

/// The shape layer's solid stroke, when it has an enabled one.
pub fn shape_stroke(layer: &Layer) -> Option<ShapeStroke> {
    let stroke = decode_stroke(layer).filter(|s| s.enabled && s.present)?;
    let StrokeFill::Solid(color) = stroke.fill else {
        return None;
    };
    Some(ShapeStroke {
        color,
        width: stroke.size,
        position: stroke.position,
    })
}

fn object(class: &[u8], items: Vec<(&[u8], DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class.to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    }
}

fn enumerated(kind: &[u8], value: &[u8]) -> DescValue {
    DescValue::Enum {
        kind: kind.to_vec(),
        value: value.to_vec(),
    }
}

fn unit(unit: &[u8; 4], value: f64) -> DescValue {
    DescValue::UnitFloat { unit: *unit, value }
}

fn frfx(stroke: &ShapeStroke) -> DescValue {
    let position: &[u8] = match stroke.position {
        StrokePosition::Inside => b"InsF",
        StrokePosition::Center => b"CtrF",
        StrokePosition::Outside => b"OutF",
    };
    let [r, g, b] = stroke.color.map(f64::from);
    object(
        b"FrFX",
        vec![
            (b"enab", DescValue::Bool(true)),
            (b"present", DescValue::Bool(true)),
            (b"showInDialog", DescValue::Bool(true)),
            (b"Styl", enumerated(b"FStl", position)),
            (b"PntT", enumerated(b"FrFl", b"SClr")),
            (b"Md  ", enumerated(b"BlnM", b"Nrml")),
            (b"Opct", unit(b"#Prc", 100.0)),
            (
                b"Sz  ",
                unit(b"#Pxl", f64::from(stroke.width.clamp(1, 250))),
            ),
            (
                b"Clr ",
                object(
                    b"RGBC",
                    vec![
                        (b"Rd  ", DescValue::Double(r)),
                        (b"Grn ", DescValue::Double(g)),
                        (b"Bl  ", DescValue::Double(b)),
                    ],
                ),
            ),
        ],
    )
}

/// Give the shape layer `stroke`, or (`None`) remove its stroke, keeping any
/// other layer effects. False when the layer is not a shape layer or nothing
/// changed.
pub fn set_shape_stroke(layer: &mut Layer, stroke: Option<&ShapeStroke>) -> bool {
    if !is_shape_layer(layer) || shape_stroke(layer).as_ref() == stroke {
        return false;
    }
    let existing = layer
        .extra_block(b"lfx2")
        .and_then(|b| pictura_codec::read_descriptor(b.data.get(4..)?).ok());
    let mut items = match existing {
        Some(DescValue::Object { items, .. }) => items,
        _ => vec![
            (b"Scl ".to_vec(), unit(b"#Prc", 100.0)),
            (b"masterFXSwitch".to_vec(), DescValue::Bool(true)),
        ],
    };
    items.retain(|(k, _)| k.as_slice() != b"FrFX");
    if let Some(stroke) = stroke {
        items.push((b"FrFX".to_vec(), frfx(stroke)));
    }
    let has_effects = items
        .iter()
        .any(|(k, _)| !matches!(k.as_slice(), b"Scl " | b"masterFXSwitch"));
    layer.extra_blocks.retain(|b| &b.key != b"lfx2");
    if has_effects {
        let mut data = 0u32.to_be_bytes().to_vec();
        data.extend_from_slice(&pictura_codec::write_descriptor(&object(
            b"null",
            items
                .iter()
                .map(|(k, v)| (k.as_slice(), v.clone()))
                .collect(),
        )));
        layer.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"lfx2",
            data: data.into(),
        });
    }
    true
}

/// The shape layer's outline bounds as `(left, top, right, bottom)`.
pub fn shape_bounds(layer: &Layer, width: u32, height: u32) -> Option<(f64, f64, f64, f64)> {
    let mut path = VectorPath::default();
    for subpath in layer_shape_paths(layer, width, height)? {
        path.add_subpath(subpath);
    }
    (0..path.subpaths.len())
        .filter_map(|i| path.subpath_bounds(i))
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
}

/// Scale the shape layer's outline about its top-left corner to `new_width` x
/// `new_height`; a live shape stays live with its box resized. False when the
/// layer is not a shape layer, has no extent, or the size is unchanged.
pub fn resize_shape(
    layer: &mut Layer,
    width: u32,
    height: u32,
    new_width: f64,
    new_height: f64,
) -> bool {
    let Some((l, t, r, b)) = shape_bounds(layer, width, height) else {
        return false;
    };
    let (w, h) = (r - l, b - t);
    if !(w > 0.0 && h > 0.0 && new_width > 0.0 && new_height > 0.0)
        || ((new_width - w).abs() < 1e-3 && (new_height - h).abs() < 1e-3)
    {
        return false;
    }
    let (sx, sy) = (new_width / w, new_height / h);
    let scale = |(x, y): (f64, f64)| (l + (x - l) * sx, t + (y - t) * sy);
    let Some(paths) = layer_shape_paths(layer, width, height) else {
        return false;
    };
    let scaled: Vec<Subpath> = paths
        .into_iter()
        .map(|s| Subpath {
            points: s
                .points
                .into_iter()
                .map(|p| PathPoint {
                    anchor: scale(p.anchor),
                    in_handle: p.in_handle.map(scale),
                    out_handle: p.out_handle.map(scale),
                    smooth: p.smooth,
                })
                .collect(),
            closed: s.closed,
        })
        .collect();
    set_layer_shape_paths(layer, &scaled, width, height);
    if let Some(mut live) = layer_live_shape(layer) {
        let (ll, lt, lr, lb) = live.bounds;
        let (a, c) = (scale((ll, lt)), scale((lr, lb)));
        live.bounds = (a.0, a.1, c.0, c.1);
        set_layer_live_shape(layer, Some(&live));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_ops::layer_ops::{add_shape_layer, resolve_path, resolve_path_mut};
    use pictura_core::shape::{outline_in_box, ShapeKind, ShapeOptions};
    use pictura_core::Document;

    fn doc_with_square() -> (Document, String) {
        let mut doc = Document::from_rgba("Background", 20, 20, &[255; 20 * 20 * 4]);
        let square = outline_in_box(
            ShapeOptions::new(ShapeKind::Rectangle, 0.0, 3),
            (5.0, 5.0, 10.0, 10.0),
        )
        .unwrap();
        let path = add_shape_layer(&mut doc, "", [0, 0, 255, 255], "Rectangle", &square, None);
        (doc, path)
    }

    fn rgb(doc: &Document, x: usize, y: usize) -> [u8; 3] {
        // The composite is planar: R, G, B, then A planes.
        let out = crate::composite_rgba(doc);
        let plane = (out.width * out.height) as usize;
        let i = y * out.width as usize + x;
        [out.data[i], out.data[plane + i], out.data[2 * plane + i]]
    }

    #[test]
    fn fill_none_hides_the_fill_and_a_stroke_draws_the_outline() {
        let (mut doc, path) = doc_with_square();
        let layer = resolve_path_mut(&mut doc, &path).unwrap();
        assert_eq!(shape_fill(layer), Some([0, 0, 255]));
        assert!(set_shape_fill(layer, Some([255, 0, 0])));
        assert!(!set_shape_fill(layer, Some([255, 0, 0])), "unchanged");
        assert!(set_shape_fill(layer, None));
        assert_eq!(shape_fill(layer), None);
        let stroke = ShapeStroke {
            color: [0, 255, 0],
            width: 2,
            position: StrokePosition::Inside,
        };
        assert!(set_shape_stroke(layer, Some(&stroke)));
        assert_eq!(shape_stroke(layer), Some(stroke));
        assert_eq!(rgb(&doc, 10, 10), [255, 255, 255], "no fill inside");
        assert_eq!(
            rgb(&doc, 5, 10),
            [0, 255, 0],
            "the stroke on the inner edge"
        );
        assert_eq!(rgb(&doc, 3, 10), [255, 255, 255], "nothing outside");

        let layer = resolve_path_mut(&mut doc, &path).unwrap();
        assert!(set_shape_stroke(layer, None));
        assert!(
            layer.extra_block(b"lfx2").is_none(),
            "the empty effects block is dropped"
        );
        assert_eq!(rgb(&doc, 5, 10), [255, 255, 255]);
    }

    #[test]
    fn copy_and_paste_attributes_transfer_fill_and_stroke() {
        let (mut doc, path) = doc_with_square();
        let stroke = ShapeStroke {
            color: [0, 255, 0],
            width: 2,
            position: StrokePosition::Inside,
        };
        {
            let layer = resolve_path_mut(&mut doc, &path).unwrap();
            assert!(set_shape_fill(layer, Some([10, 20, 30])));
            assert!(set_shape_stroke(layer, Some(&stroke)));
        }
        let attrs = copy_shape_attributes(resolve_path(&doc, &path).unwrap()).unwrap();
        assert_eq!(
            attrs,
            ShapeAttributes {
                fill: Some([10, 20, 30]),
                stroke: Some(stroke),
            }
        );

        // Change the target, then paste the captured appearance back.
        {
            let layer = resolve_path_mut(&mut doc, &path).unwrap();
            assert!(set_shape_fill(layer, Some([1, 1, 1])));
            assert!(set_shape_stroke(layer, None));
            assert!(paste_shape_attributes(layer, &attrs));
            assert_eq!(shape_fill(layer), Some([10, 20, 30]));
            assert_eq!(shape_stroke(layer), Some(stroke));
            assert!(
                !paste_shape_attributes(layer, &attrs),
                "an identical paste changes nothing"
            );
        }
    }

    #[test]
    fn attributes_refuse_a_non_shape_layer() {
        let (mut doc, _path) = doc_with_square();
        let background = resolve_path_mut(&mut doc, "0").unwrap();
        assert!(!is_shape_layer(background));
        assert_eq!(copy_shape_attributes(background), None);
        let attrs = ShapeAttributes {
            fill: Some([1, 2, 3]),
            stroke: None,
        };
        assert!(!paste_shape_attributes(background, &attrs));
    }

    #[test]
    fn a_shape_resizes_about_its_top_left() {
        let (mut doc, path) = doc_with_square();
        let layer = resolve_path_mut(&mut doc, &path).unwrap();
        assert_eq!(shape_bounds(layer, 20, 20), Some((5.0, 5.0, 15.0, 15.0)));
        assert!(resize_shape(layer, 20, 20, 4.0, 12.0));
        assert!(!resize_shape(layer, 20, 20, 4.0, 12.0), "unchanged");
        let layer = resolve_path(&doc, &path).unwrap();
        let (l, t, r, b) = shape_bounds(layer, 20, 20).unwrap();
        assert!((l - 5.0).abs() < 1e-3 && (t - 5.0).abs() < 1e-3);
        assert!((r - 9.0).abs() < 1e-3 && (b - 17.0).abs() < 1e-3);
        assert_eq!(rgb(&doc, 7, 16), [0, 0, 255]);
        assert_eq!(rgb(&doc, 12, 7), [255, 255, 255]);
    }
}
