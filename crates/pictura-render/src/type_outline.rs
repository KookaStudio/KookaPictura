//! Glyph outlines for `Type > Create Work Path` and `Type > Convert to Shape`.
//!
//! Unwarped, area, and on-path type keep the swash outline's curves as cubic
//! `PathPoint` handles; warped type reuses the warp renderer's flattened
//! contours because a non-affine warp cannot preserve curves without fitting.
//! The returned subpaths are closed and in document pixels.

use pictura_core::path::{PathPoint, Subpath};
use pictura_core::{TextPathKind, TypeSpec};
use swash::scale::ScaleContext;
use swash::zeno::{Point, Verb};

use crate::text_render::{glyph_matrix, BundledText};
use crate::type_layer::{flat_layout, map};
use crate::type_style::RunStyle;

/// Why an outline command refuses a type layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineRefusal {
    /// No face resolves for the layer's family and style.
    NoFace,
    /// The face has no `glyf`, `CFF `, or `CFF2` outlines (bitmap-only).
    NoOutlines,
    /// The layer has no glyph to outline.
    Empty,
}

/// Whether `spec` can produce an outline, and if not, why.
pub fn outline_refusal(spec: &TypeSpec) -> Option<OutlineRefusal> {
    outline_refusal_with(
        spec,
        crate::fonts::face_for(&spec.character.font_family, &spec.character.font_style),
    )
}

/// [`outline_refusal`] against an already resolved `face`.
pub(crate) fn outline_refusal_with(
    spec: &TypeSpec,
    face: Option<&BundledText>,
) -> Option<OutlineRefusal> {
    let Some(face) = face else {
        return Some(OutlineRefusal::NoFace);
    };
    if !crate::fonts::face_has_outlines(face) {
        return Some(OutlineRefusal::NoOutlines);
    }
    match outline_paths(spec) {
        Some(paths) if !paths.is_empty() => None,
        _ => Some(OutlineRefusal::Empty),
    }
}

/// The type layer's glyph outlines in document pixels. `None` when the face or
/// layout is unavailable; an empty vector when the text inks nothing.
pub fn outline_paths(spec: &TypeSpec) -> Option<Vec<Subpath>> {
    match spec.path.as_ref() {
        Some(path) if path.kind == TextPathKind::OnPath => on_path_paths(spec),
        Some(_) => flat_paths(spec),
        None if crate::type_warp::is_warped(spec) => warped_paths(spec),
        None => flat_paths(spec),
    }
}

/// Point, paragraph, and area type: the flat layout mapped into the document.
fn flat_paths(spec: &TypeSpec) -> Option<Vec<Subpath>> {
    let mut layout_spec = spec.clone();
    if layout_spec
        .path
        .as_ref()
        .is_some_and(|path| path.kind == TextPathKind::Area)
        && layout_spec.box_bounds.is_none()
    {
        let flattened = layout_spec.path.as_ref()?.subpath.flattened(0.25);
        let (left, top, right, bottom) = path_bounds(&flattened.points)?;
        layout_spec.origin = (left, top);
        layout_spec.box_bounds = Some([0.0, 0.0, right - left, bottom - top]);
    }
    let (bundled, layout, unmapped) = flat_layout(&layout_spec)?;
    let run = RunStyle::of(bundled, &spec.character);
    let ascent = bundled.ascent(spec.character.size as f32);
    let to_doc = |(x, y): (f64, f64)| {
        let u = f64::from(unmapped.left) + x - layout_spec.origin.0;
        let v = f64::from(unmapped.top) + y - layout_spec.origin.1;
        map(&layout_spec, u, v)
    };
    let mut context = ScaleContext::new();
    let faces = crate::fonts::faces_in_order(bundled);
    let mut out = Vec::new();
    for glyph in layout.lines.iter().flat_map(|line| &line.glyphs) {
        let face = faces.get(glyph.face as usize).copied().unwrap_or(bundled);
        let mut m = glyph_matrix(&run, glyph.scale);
        m[0] *= glyph.x_scale;
        m[2] *= glyph.x_scale;
        let m = m.map(f64::from);
        let (pen_x, pen_y) = (f64::from(glyph.x), f64::from(glyph.y + ascent));
        let place = |p: Point| {
            let (x, y) = (f64::from(p.x), -f64::from(p.y));
            to_doc((pen_x + m[0] * x + m[2] * y, pen_y + m[1] * x + m[3] * y))
        };
        let mut scaler = context
            .builder(face.swash())
            .size(run.size)
            .hint(false)
            .build();
        let Some(mut outline) = scaler.scale_outline(glyph.id) else {
            continue;
        };
        if run.embolden > 0.0 {
            outline.embolden(run.embolden, run.embolden);
        }
        out.extend(contour_subpaths(outline.points(), outline.verbs(), place));
    }
    for rect in crate::type_decor::decorations(&run, &layout, spec.vertical, ascent) {
        let [x0, y0, x1, y1] = rect.map(f64::from);
        let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        out.push(Subpath {
            points: corners.into_iter().map(|p| corner(to_doc(p))).collect(),
            closed: true,
        });
    }
    Some(out)
}

/// Type on a path: the path layout's rotated glyphs.
fn on_path_paths(spec: &TypeSpec) -> Option<Vec<Subpath>> {
    let layout = crate::type_path::layout_path(spec)?;
    let bundled = crate::fonts::face_for(&spec.character.font_family, &spec.character.font_style)?;
    let run = RunStyle::of(bundled, &spec.character);
    let faces = crate::fonts::faces_in_order(bundled);
    let mut context = ScaleContext::new();
    let mut out = Vec::new();
    for glyph in &layout.glyphs {
        let face = faces.get(glyph.face as usize).copied().unwrap_or(bundled);
        let mut m = glyph_matrix(&run, glyph.scale);
        m[0] *= glyph.x_scale;
        m[2] *= glyph.x_scale;
        let m = m.map(f64::from);
        let (sin, cos) = glyph.angle.sin_cos();
        let place = |p: Point| {
            let (x, y) = (f64::from(p.x), -f64::from(p.y));
            let (mx, my) = (m[0] * x + m[2] * y, m[1] * x + m[3] * y);
            (glyph.x + cos * mx - sin * my, glyph.y + sin * mx + cos * my)
        };
        let mut scaler = context
            .builder(face.swash())
            .size(run.size)
            .hint(false)
            .build();
        let Some(mut outline) = scaler.scale_outline(glyph.id) else {
            continue;
        };
        if run.embolden > 0.0 {
            outline.embolden(run.embolden, run.embolden);
        }
        out.extend(contour_subpaths(outline.points(), outline.verbs(), place));
    }
    Some(out)
}

/// Warped type: the warp renderer's flattened contours.
fn warped_paths(spec: &TypeSpec) -> Option<Vec<Subpath>> {
    let (glyphs, decorations) = crate::type_warp::outline_contours(spec)?;
    let mut out = Vec::new();
    for contour in glyphs.into_iter().chain(decorations) {
        if contour.len() >= 2 {
            out.push(Subpath {
                points: contour.into_iter().map(corner).collect(),
                closed: true,
            });
        }
    }
    Some(out)
}

fn path_bounds(points: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    let &(first_x, first_y) = points.first()?;
    let (mut left, mut top, mut right, mut bottom) = (first_x, first_y, first_x, first_y);
    for &(x, y) in points {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    (right > left && bottom > top).then_some((left, top, right, bottom))
}

fn corner(anchor: (f64, f64)) -> PathPoint {
    PathPoint {
        anchor,
        in_handle: None,
        out_handle: None,
        smooth: false,
    }
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// One outline's closed contours as subpaths, keeping line and cubic segments.
fn contour_subpaths(
    points: &[Point],
    verbs: &[Verb],
    place: impl Fn(Point) -> (f64, f64),
) -> Vec<Subpath> {
    let mut out = Vec::new();
    let mut current: Vec<PathPoint> = Vec::new();
    let mut points = points.iter().map(|&p| place(p));
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                flush(&mut current, &mut out);
                let Some(anchor) = points.next() else { break };
                current.push(corner(anchor));
            }
            Verb::LineTo => {
                let Some(anchor) = points.next() else { break };
                current.push(corner(anchor));
            }
            Verb::QuadTo => {
                let (Some(control), Some(anchor)) = (points.next(), points.next()) else {
                    break;
                };
                if let Some(last) = current.last_mut() {
                    last.out_handle = Some(lerp(last.anchor, control, 2.0 / 3.0));
                }
                current.push(PathPoint {
                    anchor,
                    in_handle: Some(lerp(anchor, control, 2.0 / 3.0)),
                    out_handle: None,
                    smooth: false,
                });
            }
            Verb::CurveTo => {
                let (Some(c1), Some(c2), Some(anchor)) =
                    (points.next(), points.next(), points.next())
                else {
                    break;
                };
                if let Some(last) = current.last_mut() {
                    last.out_handle = Some(c1);
                }
                current.push(PathPoint {
                    anchor,
                    in_handle: Some(c2),
                    out_handle: None,
                    smooth: false,
                });
            }
            Verb::Close => flush(&mut current, &mut out),
        }
    }
    flush(&mut current, &mut out);
    out
}

fn flush(current: &mut Vec<PathPoint>, out: &mut Vec<Subpath>) {
    if current.len() < 2 {
        current.clear();
        return;
    }
    if let (Some(first), Some(last)) = (current.first().copied(), current.last().copied()) {
        if (first.anchor.0 - last.anchor.0).abs() < 1e-9
            && (first.anchor.1 - last.anchor.1).abs() < 1e-9
        {
            current[0].in_handle = last.in_handle;
            current.pop();
        }
    }
    if current.len() >= 2 {
        out.push(Subpath {
            points: std::mem::take(current),
            closed: true,
        });
    } else {
        current.clear();
    }
}

#[cfg(test)]
mod tests;
