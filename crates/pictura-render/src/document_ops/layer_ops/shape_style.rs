//! A shape layer's appearance and size as the shape tools' options bar edits
//! them: the Fill (solid, gradient, or pattern, or None), the Stroke (the same
//! sources, a width, an Align, and a line type), and the outline's width and
//! height.
//!
//! The shape layer's fill is the layer's `FillContent`; the stroke is an
//! authored Stroke layer effect (`lfx2` `FrFX`). CS6 keeps a vector shape's
//! stroke in a `vstk` block; the effect approximation is what the compositor
//! already draws along the vector mask. Dashed and dotted strokes are an
//! authored `Lntp`/`DshP` pair the stroke renderer cuts along the mask.

use pictura_adjust::{GradientFillParams, PatternFillParams};
use pictura_codec::DescValue;
use pictura_core::path::{PathPoint, Subpath, VectorPath};
use pictura_core::{FillContent, Layer};

use super::shape_layer::{
    is_shape_layer, layer_live_shape, layer_shape_paths, set_layer_live_shape,
    set_layer_shape_paths,
};
use crate::layer_effects::{decode_stroke, DashPattern, StrokeFill, StrokeLine, StrokePosition};

/// A shape layer's fill source.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeFill {
    Solid([u8; 3]),
    Gradient(GradientFillParams),
    Pattern(PatternFillParams),
}

/// A shape layer's stroke source.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeStrokeFill {
    Solid([u8; 3]),
    Gradient {
        params: GradientFillParams,
        align_with_layer: bool,
    },
    Pattern {
        params: PatternFillParams,
        angle_deg: f32,
    },
}

/// A stroke: its source, width in pixels, where it sits on the outline, and its
/// line type.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeStroke {
    pub fill: ShapeStrokeFill,
    pub width: u32,
    pub position: StrokePosition,
    pub line: StrokeLine,
}

impl ShapeStroke {
    /// A solid stroke of `color`, `width` px, at `position`, with a solid line.
    pub fn solid(color: [u8; 3], width: u32, position: StrokePosition) -> Self {
        Self {
            fill: ShapeStrokeFill::Solid(color),
            width,
            position,
            line: StrokeLine::Solid,
        }
    }
}

/// A shape layer's copyable appearance: the fill (or `None` when unfilled) and
/// the stroke (or `None` when unstroked).
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeAttributes {
    pub fill: Option<ShapeFill>,
    pub stroke: Option<ShapeStroke>,
}

/// The shape layer's copyable fill and stroke; `None` when the layer is not a
/// shape layer.
pub fn copy_shape_attributes(layer: &Layer) -> Option<ShapeAttributes> {
    is_shape_layer(layer).then(|| ShapeAttributes {
        fill: shape_fill_content(layer),
        stroke: shape_stroke(layer),
    })
}

/// Give the shape layer `attributes`, replacing its fill and stroke. False when
/// the layer is not a shape layer; otherwise whether anything changed.
pub fn paste_shape_attributes(layer: &mut Layer, attributes: &ShapeAttributes) -> bool {
    if !is_shape_layer(layer) {
        return false;
    }
    let fill = set_shape_fill_content(layer, attributes.fill.as_ref());
    let stroke = set_shape_stroke(layer, attributes.stroke.as_ref());
    fill | stroke
}

/// The shape layer's solid fill colour, or `None` when it has no solid fill or
/// is not a shape layer. Gradient and pattern fills are [`shape_fill_content`].
pub fn shape_fill(layer: &Layer) -> Option<[u8; 3]> {
    match shape_fill_content(layer)? {
        ShapeFill::Solid(color) => Some(color),
        ShapeFill::Gradient(_) | ShapeFill::Pattern(_) => None,
    }
}

/// The shape layer's full fill source, or `None` when it has no fill (Fill
/// opacity 0) or is not a shape layer.
pub fn shape_fill_content(layer: &Layer) -> Option<ShapeFill> {
    if !is_shape_layer(layer) || layer.fill == 0 {
        return None;
    }
    match crate::fill::decode_layer_fill(layer)? {
        FillContent::Solid([r, g, b, _]) => Some(ShapeFill::Solid([r, g, b])),
        FillContent::Gradient(params) => Some(ShapeFill::Gradient(params)),
        FillContent::Pattern(params) => Some(ShapeFill::Pattern(params)),
    }
}

/// Fill the shape layer with `fill`, or (`None`) leave it unfilled. False when
/// the layer is not a shape layer or nothing changed.
pub fn set_shape_fill(layer: &mut Layer, fill: Option<[u8; 3]>) -> bool {
    set_shape_fill_content(layer, fill.map(ShapeFill::Solid).as_ref())
}

/// Fill the shape layer with `fill`, or (`None`) leave it unfilled. False when
/// the layer is not a shape layer or nothing changed.
pub fn set_shape_fill_content(layer: &mut Layer, fill: Option<&ShapeFill>) -> bool {
    if !is_shape_layer(layer) || shape_fill_content(layer).as_ref() == fill {
        return false;
    }
    match fill {
        Some(ShapeFill::Solid([r, g, b])) => {
            if !layer.set_fill_content(FillContent::Solid([*r, *g, *b, 255])) {
                return false;
            }
            layer.fill = 255;
        }
        Some(ShapeFill::Gradient(params)) => {
            if !layer.set_fill_content(FillContent::Gradient(params.clone())) {
                return false;
            }
            layer.fill = 255;
        }
        Some(ShapeFill::Pattern(params)) => {
            if !layer.set_fill_content(FillContent::Pattern(params.clone())) {
                return false;
            }
            layer.fill = 255;
        }
        None => layer.fill = 0,
    }
    true
}

/// The shape layer's stroke, when it has an enabled one.
pub fn shape_stroke(layer: &Layer) -> Option<ShapeStroke> {
    let stroke = decode_stroke(layer).filter(|s| s.enabled && s.present)?;
    let fill = match stroke.fill {
        StrokeFill::Solid(color) => ShapeStrokeFill::Solid(color),
        StrokeFill::Gradient {
            params,
            align_with_layer,
        } => ShapeStrokeFill::Gradient {
            params,
            align_with_layer,
        },
        StrokeFill::Pattern { params, angle_deg } => ShapeStrokeFill::Pattern { params, angle_deg },
    };
    Some(ShapeStroke {
        fill,
        width: stroke.size,
        position: stroke.position,
        line: stroke.line,
    })
}

fn object(class: &[u8], items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: class.to_vec(),
        items,
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

/// The descriptor items the shared gradient-fill encoder writes, ready to merge
/// into an `FrFX` object.
fn gradient_items(params: &GradientFillParams) -> Option<Vec<(Vec<u8>, DescValue)>> {
    let data = pictura_codec::encode_gradient_fill_params(params);
    match pictura_codec::read_descriptor(&data.data).ok()? {
        DescValue::Object { items, .. } => Some(items),
        _ => None,
    }
}

/// The descriptor items the shared pattern-fill encoder writes, ready to merge
/// into an `FrFX` object.
fn pattern_items(params: &PatternFillParams) -> Option<Vec<(Vec<u8>, DescValue)>> {
    let data = pictura_codec::encode_pattern_fill(params);
    match pictura_codec::read_descriptor(&data.data).ok()? {
        DescValue::Object { items, .. } => Some(items),
        _ => None,
    }
}

fn line_name(line: StrokeLine) -> &'static str {
    match line {
        StrokeLine::Solid => "solid",
        StrokeLine::Dashed => "dashed",
        StrokeLine::Dotted => "dotted",
    }
}

fn frfx(stroke: &ShapeStroke) -> Option<DescValue> {
    let position: &[u8] = match stroke.position {
        StrokePosition::Inside => b"InsF",
        StrokePosition::Center => b"CtrF",
        StrokePosition::Outside => b"OutF",
    };
    let mut items: Vec<(Vec<u8>, DescValue)> = vec![
        (b"enab".to_vec(), DescValue::Bool(true)),
        (b"present".to_vec(), DescValue::Bool(true)),
        (b"showInDialog".to_vec(), DescValue::Bool(true)),
        (b"Styl".to_vec(), enumerated(b"FStl", position)),
        (b"Opct".to_vec(), unit(b"#Prc", 100.0)),
        (
            b"Sz  ".to_vec(),
            unit(b"#Pxl", f64::from(stroke.width.clamp(1, 250))),
        ),
        (
            b"Lntp".to_vec(),
            DescValue::Text(line_name(stroke.line).into()),
        ),
    ];
    if let Some(dash) = DashPattern::for_line(stroke.line, stroke.width) {
        items.push((
            b"DshP".to_vec(),
            DescValue::Text(format!("{},{}", dash.on, dash.off)),
        ));
    }
    match &stroke.fill {
        ShapeStrokeFill::Solid(color) => {
            let [r, g, b] = color.map(f64::from);
            items.push((b"PntT".to_vec(), enumerated(b"FrFl", b"SClr")));
            items.push((b"Md  ".to_vec(), enumerated(b"BlnM", b"Nrml")));
            items.push((
                b"Clr ".to_vec(),
                object(
                    b"RGBC",
                    vec![
                        (b"Rd  ".to_vec(), DescValue::Double(r)),
                        (b"Grn ".to_vec(), DescValue::Double(g)),
                        (b"Bl  ".to_vec(), DescValue::Double(b)),
                    ],
                ),
            ));
        }
        ShapeStrokeFill::Gradient {
            params,
            align_with_layer,
        } => {
            items.push((b"PntT".to_vec(), enumerated(b"FrFl", b"GrFl")));
            items.extend(gradient_items(params)?);
            items.push((b"Algn".to_vec(), DescValue::Bool(*align_with_layer)));
        }
        ShapeStrokeFill::Pattern { params, angle_deg } => {
            items.push((b"PntT".to_vec(), enumerated(b"FrFl", b"Ptrn")));
            items.extend(pattern_items(params)?);
            items.push((b"Angl".to_vec(), DescValue::Double(f64::from(*angle_deg))));
            items.push((b"Lnkd".to_vec(), DescValue::Bool(params.link_with_layer)));
        }
    }
    Some(object(b"FrFX", items))
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
        let Some(stroke) = frfx(stroke) else {
            return false;
        };
        items.push((b"FrFX".to_vec(), stroke));
    }
    let has_effects = items
        .iter()
        .any(|(k, _)| !matches!(k.as_slice(), b"Scl " | b"masterFXSwitch"));
    pictura_codec::edit_layer(layer, |parts| {
        parts.extra_blocks.retain(|b| &b.key != b"lfx2")
    });
    if has_effects {
        let mut data = 0u32.to_be_bytes().to_vec();
        data.extend_from_slice(&pictura_codec::write_descriptor(&object(
            b"null",
            items.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        )));
        pictura_codec::edit_layer(layer, |parts| {
            parts.extra_blocks.push(pictura_core::LayerBlock {
                key: *b"lfx2",
                data: data.into(),
            })
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

    fn gradient() -> GradientFillParams {
        GradientFillParams {
            stops: vec![
                pictura_core::GradientStop {
                    location: 0,
                    color: [255, 0, 0],
                },
                pictura_core::GradientStop {
                    location: 4096,
                    color: [0, 0, 255],
                },
            ],
            reverse: false,
            kind: pictura_adjust::GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        }
    }

    fn pattern() -> PatternFillParams {
        PatternFillParams {
            pattern_id: "pictura-shape".into(),
            scale: 100.0,
            link_with_layer: true,
            origin: (0, 0),
        }
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
        let stroke = ShapeStroke::solid([0, 255, 0], 2, StrokePosition::Inside);
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
    fn gradient_fill_and_pattern_fill_round_trip() {
        let (mut doc, path) = doc_with_square();
        let layer = resolve_path_mut(&mut doc, &path).unwrap();
        assert!(set_shape_fill_content(
            layer,
            Some(&ShapeFill::Gradient(gradient()))
        ));
        assert_eq!(
            shape_fill_content(layer),
            Some(ShapeFill::Gradient(gradient()))
        );
        assert!(set_shape_fill_content(
            layer,
            Some(&ShapeFill::Pattern(pattern()))
        ));
        assert_eq!(
            shape_fill_content(layer),
            Some(ShapeFill::Pattern(pattern()))
        );
        assert_eq!(layer.fill, 255);
    }

    #[test]
    fn dashed_and_dotted_strokes_render_gaps() {
        for (line, expect_gap) in [(StrokeLine::Dashed, true), (StrokeLine::Dotted, true)] {
            let (mut doc, path) = doc_with_square();
            let layer = resolve_path_mut(&mut doc, &path).unwrap();
            assert!(set_shape_fill(layer, None));
            let stroke = ShapeStroke {
                fill: ShapeStrokeFill::Solid([0, 255, 0]),
                width: 4,
                position: StrokePosition::Inside,
                line,
            };
            assert!(set_shape_stroke(layer, Some(&stroke)));
            let mut on = 0;
            let mut off = 0;
            for x in 5..15 {
                if rgb(&doc, x, 6) == [0, 255, 0] {
                    on += 1;
                } else {
                    off += 1;
                }
            }
            assert!(on > 0, "{line:?} has a mark");
            assert_eq!(off > 0, expect_gap, "{line:?} has a gap");
        }
    }

    #[test]
    fn dashes_follow_a_curved_outline() {
        // A circle of radius 20 about (30, 30): the dash walk must follow its
        // flattened arc, not the chords between its four anchors (which come
        // within 20 / sqrt(2) ≈ 14 px of the centre).
        let mut doc = Document::from_rgba("Background", 60, 60, &[255; 60 * 60 * 4]);
        let circle = outline_in_box(
            ShapeOptions::new(ShapeKind::Ellipse, 0.0, 3),
            (10.0, 10.0, 40.0, 40.0),
        )
        .unwrap();
        let path = add_shape_layer(&mut doc, "", [0, 0, 255, 255], "Ellipse", &circle, None);
        let layer = resolve_path_mut(&mut doc, &path).unwrap();
        assert!(set_shape_fill(layer, None));
        let stroke = ShapeStroke {
            fill: ShapeStrokeFill::Solid([0, 255, 0]),
            width: 2,
            position: StrokePosition::Inside,
            line: StrokeLine::Dashed,
        };
        assert!(set_shape_stroke(layer, Some(&stroke)));
        let out = crate::composite_rgba(&doc);
        let plane = (out.width * out.height) as usize;
        let (mut marked, mut gaps) = (0, 0);
        for y in 0..60usize {
            for x in 0..60usize {
                let i = y * 60 + x;
                let green = out.data[i] < 128 && out.data[plane + i] > 128;
                let r = (x as f64 + 0.5 - 30.0).hypot(y as f64 + 0.5 - 30.0);
                if green {
                    marked += 1;
                    assert!(
                        (16.5..=21.0).contains(&r),
                        "dash pixel ({x}, {y}) at radius {r:.1}"
                    );
                } else if (18.5..=19.5).contains(&r) {
                    gaps += 1;
                }
            }
        }
        assert!(marked > 0, "the dashed stroke draws");
        assert!(gaps > 0, "the dashed stroke has gaps along the arc");
    }

    #[test]
    fn copy_and_paste_attributes_transfer_gradient_and_pattern() {
        let (mut doc, path) = doc_with_square();
        let stroke = ShapeStroke {
            fill: ShapeStrokeFill::Pattern {
                params: pattern(),
                angle_deg: 0.0,
            },
            width: 2,
            position: StrokePosition::Inside,
            line: StrokeLine::Dashed,
        };
        {
            let layer = resolve_path_mut(&mut doc, &path).unwrap();
            assert!(set_shape_fill_content(
                layer,
                Some(&ShapeFill::Gradient(gradient()))
            ));
            assert!(set_shape_stroke(layer, Some(&stroke)));
        }
        let attrs = copy_shape_attributes(resolve_path(&doc, &path).unwrap()).unwrap();
        assert_eq!(attrs.fill, Some(ShapeFill::Gradient(gradient())));
        assert_eq!(attrs.stroke, Some(stroke.clone()));

        {
            let layer = resolve_path_mut(&mut doc, &path).unwrap();
            assert!(set_shape_fill_content(
                layer,
                Some(&ShapeFill::Solid([1, 1, 1]))
            ));
            assert!(set_shape_stroke(layer, None));
            assert!(paste_shape_attributes(layer, &attrs));
            assert_eq!(
                shape_fill_content(layer),
                Some(ShapeFill::Gradient(gradient()))
            );
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
            fill: Some(ShapeFill::Solid([1, 2, 3])),
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
