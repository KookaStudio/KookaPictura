//! `Type > Warp Text`: warped type rendered from warped glyph outlines.
//!
//! The text is laid out in its untransformed box exactly as unwarped type is.
//! Each glyph's outline is then flattened, every point is mapped through the
//! warp (a [`TextWarp`] over that box) and then through the layer's matrix,
//! and the mapped outlines are filled with swash's zeno rasterizer. Mapping
//! the outline instead of resampling the glyph bitmap keeps warped edges as
//! crisp as unwarped type. Underline and strikethrough rectangles bend the
//! same way.
//!
//! ponytail: the outlines are rebuilt for every placement and render (no
//! cache); the layer rect bounds the work, as it does for unwarped type.

use pictura_core::warp::{TextWarp, Warp};
use pictura_core::{PixelBuffer, PsdRect, TypeSpec};
use swash::scale::ScaleContext;
use swash::zeno::{Command, Mask, Point, Vector, Verb};

use crate::text_render::{glyph_matrix, BundledText};
use crate::type_layer::{flat_layout, map, type_placement, TypePlacement};
use crate::type_style::RunStyle;

/// Why Warp Text refuses a type layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpRefusal {
    /// Type on a path or inside a path (area type).
    OnPath,
    FauxBold,
    /// The font has no `glyf`, `CFF ` or `CFF2` outlines (bitmap-only).
    NoOutlines,
}

/// Whether Warp Text may warp `spec`, and if not, why.
pub fn type_warp_refusal(spec: &TypeSpec) -> Option<WarpRefusal> {
    refusal_with(
        spec,
        crate::fonts::face_for(&spec.character.font_family, &spec.character.font_style),
    )
}

/// [`type_warp_refusal`] against an already resolved `face`.
fn refusal_with(spec: &TypeSpec, face: Option<&BundledText>) -> Option<WarpRefusal> {
    if spec.path.is_some() {
        Some(WarpRefusal::OnPath)
    } else if spec.character.faux_bold {
        Some(WarpRefusal::FauxBold)
    } else if !face.is_some_and(crate::fonts::face_has_outlines) {
        Some(WarpRefusal::NoOutlines)
    } else {
        None
    }
}

/// Whether `spec` renders through this module. Path type ignores a warp it
/// was opened with: its renderers place glyphs along the path.
pub(crate) fn is_warped(spec: &TypeSpec) -> bool {
    spec.path.is_none() && !spec.warp.is_identity()
}

type Contour = Vec<(f64, f64)>;

/// The warped outlines in document pixels: glyphs and decorations apart, so
/// an underline crossing a descender cannot cancel it under the non-zero rule.
struct Geometry {
    glyphs: Vec<Contour>,
    decorations: Vec<Contour>,
}

impl Geometry {
    fn points(&self) -> impl Iterator<Item = &(f64, f64)> {
        self.glyphs.iter().chain(&self.decorations).flatten()
    }
}

fn unwarped(spec: &TypeSpec) -> TypeSpec {
    TypeSpec {
        warp: TextWarp::default(),
        ..spec.clone()
    }
}

fn geometry(spec: &TypeSpec) -> Option<Geometry> {
    let flat = unwarped(spec);
    // The box relative to the origin, before the matrix: the warp's frame.
    let bounds = type_placement(&flat)?.bounds;
    let warp: Warp = spec.warp.over(bounds);
    let (bundled, layout, unmapped) = flat_layout(&flat)?;
    let run = RunStyle::of(bundled, &spec.character);
    let ascent = bundled.ascent(spec.character.size as f32);
    // A point of the flat layout (the frame glyphs paint in) in the document.
    let to_doc = |(x, y): (f64, f64)| {
        let u = f64::from(unmapped.left) + x - spec.origin.0;
        let v = f64::from(unmapped.top) + y - spec.origin.1;
        let (wu, wv) = warp.map(u, v);
        map(spec, wu, wv)
    };
    // Short enough that the warp's curvature stays smooth between points.
    let step = ((bounds[2] - bounds[0]).min(bounds[3] - bounds[1]) / 32.0).clamp(0.25, 2.0);

    let mut context = ScaleContext::new();
    let faces = crate::fonts::faces_in_order(bundled);
    let mut glyphs = Vec::new();
    for glyph in layout.lines.iter().flat_map(|line| &line.glyphs) {
        let face = faces.get(glyph.face as usize).copied().unwrap_or(bundled);
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
        let mut m = glyph_matrix(&run, glyph.scale);
        m[0] *= glyph.x_scale;
        m[2] *= glyph.x_scale;
        let m = m.map(f64::from);
        let (pen_x, pen_y) = (f64::from(glyph.x), f64::from(glyph.y + ascent));
        // swash's outline is y-up; the layout frame is y-down.
        let place = |p: Point| {
            let (x, y) = (f64::from(p.x), -f64::from(p.y));
            (pen_x + m[0] * x + m[2] * y, pen_y + m[1] * x + m[3] * y)
        };
        for contour in flatten(outline.points(), outline.verbs(), place, step) {
            glyphs.push(contour.into_iter().map(to_doc).collect());
        }
    }
    let decorations = crate::type_decor::decorations(&run, &layout, spec.vertical, ascent)
        .into_iter()
        .map(|r| {
            let [x0, y0, x1, y1] = r.map(f64::from);
            let mut contour = vec![(x0, y0)];
            for (from, to) in [
                ((x0, y0), (x1, y0)),
                ((x1, y0), (x1, y1)),
                ((x1, y1), (x0, y1)),
                ((x0, y1), (x0, y0)),
            ] {
                segment(&mut contour, |t| lerp(from, to, t), dist(from, to), step);
            }
            contour.into_iter().map(to_doc).collect()
        })
        .collect();
    Some(Geometry {
        glyphs,
        decorations,
    })
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// Append `eval(t)` for `t` in `(0, 1]`, in steps no longer than `step` along
/// a curve whose control polygon is `len` long.
fn segment(contour: &mut Contour, eval: impl Fn(f64) -> (f64, f64), len: f64, step: f64) {
    let n = (len / step).ceil().clamp(1.0, 256.0) as usize;
    contour.extend((1..=n).map(|k| eval(k as f64 / n as f64)));
}

/// An outline's closed contours as polylines. `place` is affine, so mapping
/// the control points first is exact.
fn flatten(
    points: &[Point],
    verbs: &[Verb],
    place: impl Fn(Point) -> (f64, f64),
    step: f64,
) -> Vec<Contour> {
    let mut contours = Vec::new();
    let mut current: Contour = Vec::new();
    let mut points = points.iter().map(|&p| place(p));
    let mut last = (0.0, 0.0);
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                if current.len() > 2 {
                    contours.push(std::mem::take(&mut current));
                }
                current.clear();
                let Some(p) = points.next() else { break };
                current.push(p);
                last = p;
            }
            Verb::LineTo => {
                let Some(p) = points.next() else { break };
                segment(&mut current, |t| lerp(last, p, t), dist(last, p), step);
                last = p;
            }
            Verb::QuadTo => {
                let (Some(c), Some(p)) = (points.next(), points.next()) else {
                    break;
                };
                let a = last;
                let len = dist(a, c) + dist(c, p);
                segment(
                    &mut current,
                    |t| lerp(lerp(a, c, t), lerp(c, p, t), t),
                    len,
                    step,
                );
                last = p;
            }
            Verb::CurveTo => {
                let (Some(c1), Some(c2), Some(p)) = (points.next(), points.next(), points.next())
                else {
                    break;
                };
                let a = last;
                let len = dist(a, c1) + dist(c1, c2) + dist(c2, p);
                segment(
                    &mut current,
                    |t| {
                        let (ab, bc, cd) = (lerp(a, c1, t), lerp(c1, c2, t), lerp(c2, p, t));
                        lerp(lerp(ab, bc, t), lerp(bc, cd, t), t)
                    },
                    len,
                    step,
                );
                last = p;
            }
            Verb::Close => {
                if current.len() > 2 {
                    contours.push(std::mem::take(&mut current));
                }
                current.clear();
            }
        }
    }
    if current.len() > 2 {
        contours.push(current);
    }
    contours
}

/// Warped type's placement: the rect bounds the warped ink, and `bounds` stays
/// the unwarped box (the frame the caret and the transform box use).
pub(crate) fn placement(spec: &TypeSpec) -> Option<TypePlacement> {
    let flat = type_placement(&unwarped(spec))?;
    let geometry = geometry(spec)?;
    let mut lo = (f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN);
    for &(x, y) in geometry.points() {
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    if ![lo.0, lo.1, hi.0, hi.1].iter().all(|v| v.is_finite()) || lo.0 > hi.0 || lo.1 > hi.1 {
        return None;
    }
    // A preset at ±100 % moves the box by about its own size; anything far
    // past that is corrupt input, refused before a plane is allocated for it.
    let span = f64::from(flat.rect.width().max(flat.rect.height()));
    if (hi.0 - lo.0).max(hi.1 - lo.1) > 4.0 * span + 64.0 {
        return None;
    }
    let rect = PsdRect {
        top: lo.1.floor() as i32,
        left: lo.0.floor() as i32,
        bottom: hi.1.ceil() as i32 + 1,
        right: hi.0.ceil() as i32 + 1,
    };
    Some(TypePlacement {
        rect,
        bounds: flat.bounds,
    })
}

/// Fill `contours` (document pixels) into a `rect`-sized coverage plane.
fn coverage(contours: &[Contour], rect: PsdRect) -> Vec<u8> {
    let (w, h) = (rect.width() as u32, rect.height() as u32);
    let mut plane = vec![0u8; w as usize * h as usize];
    if contours.is_empty() {
        return plane;
    }
    let local = |&(x, y): &(f64, f64)| {
        Vector::new(
            (x - f64::from(rect.left)) as f32,
            (y - f64::from(rect.top)) as f32,
        )
    };
    let mut commands = Vec::new();
    for contour in contours {
        let mut points = contour.iter().map(local);
        let Some(first) = points.next() else {
            continue;
        };
        commands.push(Command::MoveTo(first));
        commands.extend(points.map(Command::LineTo));
        commands.push(Command::Close);
    }
    Mask::new(&commands[..])
        .size(w, h)
        .render_into(&mut plane, None);
    plane
}

/// Render warped `spec` over `rect`: planar R, G, B, A, colour premultiplied
/// by coverage as unwarped type paints it. `None` when nothing inks.
pub(crate) fn render(spec: &TypeSpec, rect: PsdRect) -> Option<PixelBuffer> {
    let geometry = geometry(spec)?;
    let glyphs = coverage(&geometry.glyphs, rect);
    let decorations = coverage(&geometry.decorations, rect);
    let color = spec.rgba().map(|c| f64::from(c) / 255.0);
    let mut out = PixelBuffer::new(rect.width() as u32, rect.height() as u32, 4);
    let n = out.pixel_count();
    let mut painted = false;
    for (i, alpha) in glyphs
        .iter()
        .zip(&decorations)
        .map(|(g, d)| *g.max(d))
        .enumerate()
    {
        if alpha == 0 {
            continue;
        }
        for (channel, value) in color.iter().take(3).enumerate() {
            out.data[channel * n + i] = (f64::from(alpha) / 255.0 * value * 255.0).round() as u8;
        }
        out.data[3 * n + i] = alpha;
        painted = true;
    }
    painted.then_some(out)
}

#[cfg(test)]
mod tests;
