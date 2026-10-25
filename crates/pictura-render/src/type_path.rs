//! Type on a path and area type.
//!
//! On-path text is laid out by arc length: the click is the start offset, each
//! glyph's baseline sits on the path point at its accumulated advance, and its
//! outline is rotated by the path tangent. Horizontal type stands upright
//! across the tangent; vertical type turns a further 90° so its upright axis
//! follows the tangent. An open path ends at its last anchor; a closed path
//! wraps once from the start.
//!
//! Area type reuses the paragraph layout in the path's bounding box and clips
//! the rendered coverage to the closed path interior. This is an approximation
//! of Photoshop's line-to-shape reflow, recorded as a ceiling.
//!
//! ponytail: one subpath per path type; hard line breaks on a path become
//! spaces; underline/strikethrough along a path are not painted.

use pictura_core::path::{FlattenedSubpath, Subpath};
use pictura_core::{TextPath, TextPathKind, TypeSpec};

use crate::fonts::face_for;
use crate::text_render::{blit_glyph, glyph_matrix};
use crate::type_layer::{render_view, type_lines, TypePlacement};
use crate::type_style::{shape_styled, tracking_step, RunStyle};

const FLATTEN: f64 = 0.25;

/// A glyph placed on a path: its baseline pen, rotation in radians, and outline
/// scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PathGlyph {
    pub id: u16,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub scale: f32,
    pub x_scale: f32,
    pub face: u16,
    pub advance: f64,
}

/// The laid-out on-path text and where it stops.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PathLayout {
    pub glyphs: Vec<PathGlyph>,
    pub advance: f64,
    pub start: f64,
    pub limit: f64,
    pub overflow: bool,
    pub end: (f64, f64),
    pub path_len: f64,
}

/// A path a Type tool hit: the kind, the path itself, and (on-path only) the
/// arc length under the click.
#[derive(Debug, Clone, PartialEq)]
pub struct PathHit {
    pub kind: TextPathKind,
    pub subpath: Subpath,
    pub start: f64,
}

impl PathHit {
    pub fn text_path(&self) -> TextPath {
        TextPath {
            kind: self.kind,
            subpath: self.subpath.clone(),
            start: self.start,
        }
    }

    /// Cubic segments as `[x0, y0, c1x, c1y, c2x, c2y, x1, y1]` for the bridge.
    pub fn control_points(&self) -> Vec<f64> {
        segments(&self.subpath)
            .into_iter()
            .flat_map(|s| s.into_iter().flat_map(|(x, y)| [x, y]))
            .collect()
    }
}

/// The cubic segments of `subpath`; a closed subpath includes its closing
/// segment.
pub(crate) fn segments(subpath: &Subpath) -> Vec<[(f64, f64); 4]> {
    (0..subpath.segment_count())
        .filter_map(|i| subpath.segment(i))
        .collect()
}

/// The path point at `arc`, wrapped for a closed subpath.
fn point_at(path: &FlattenedSubpath, arc: f64) -> Option<((f64, f64), (f64, f64))> {
    path.point_at(arc)
}

/// Text laid out for a path, with hard line breaks folded to spaces.
fn path_text(spec: &TypeSpec) -> String {
    type_lines(&spec.text).join(" ")
}

/// Lay `spec`'s text along its path. `None` without a path, a face, or a
/// positive size.
pub(crate) fn layout_path(spec: &TypeSpec) -> Option<PathLayout> {
    let path_spec = spec.path.as_ref()?;
    if path_spec.kind != TextPathKind::OnPath {
        return None;
    }
    let bundled = face_for(&spec.character.font_family, &spec.character.font_style)?;
    let size = spec.character.size as f32;
    if size.is_nan() || size <= 0.0 {
        return None;
    }
    let flattened = path_spec.subpath.flattened(FLATTEN);
    if flattened.points.len() < 2 || flattened.total <= 0.0 {
        return None;
    }
    let start = if flattened.closed {
        path_spec.start.rem_euclid(flattened.total)
    } else {
        path_spec.start.clamp(0.0, flattened.total)
    };
    let text = path_text(spec);
    let shaped = shape_styled(bundled, &text, &spec.character);
    let rise = f64::from(RunStyle::of(bundled, &spec.character).rise);
    let track = f64::from(tracking_step(&spec.character, size));
    let mut pen = 0.0;
    let mut glyphs = Vec::with_capacity(shaped.len());
    for glyph in shaped {
        let advance = f64::from(glyph.advance) + track;
        let arc = start + pen;
        if !flattened.closed && arc >= flattened.total {
            break;
        }
        let Some((position, tangent)) = point_at(&flattened, arc) else {
            break;
        };
        let mut angle = tangent.1.atan2(tangent.0);
        if spec.vertical {
            angle -= std::f64::consts::FRAC_PI_2;
        }
        let normal = (-tangent.1, tangent.0);
        glyphs.push(PathGlyph {
            id: glyph.id,
            x: position.0 - normal.0 * rise,
            y: position.1 - normal.1 * rise,
            angle,
            scale: glyph.scale,
            x_scale: glyph.x_scale,
            face: glyph.face,
            advance,
        });
        pen += advance;
    }
    let limit = if flattened.closed {
        start + flattened.total
    } else {
        flattened.total
    };
    let overflow = start + pen > limit + 1e-6;
    let end = point_at(&flattened, limit).map_or((0.0, 0.0), |(p, _)| p);
    Some(PathLayout {
        glyphs,
        advance: pen,
        start,
        limit,
        overflow,
        end,
        path_len: flattened.total,
    })
}

/// The layer rect and TySh bounds of path type. Area type uses the path's
/// bounding box; on-path type unions the rotated glyph boxes.
pub(crate) fn placement(spec: &TypeSpec) -> Option<TypePlacement> {
    let path = spec.path.as_ref()?;
    let flattened = path.subpath.flattened(FLATTEN);
    if path.kind == TextPathKind::Area {
        let (left, top, right, bottom) = bounds(&flattened.points)?;
        let (origin, box_bounds) = match spec.box_bounds {
            Some(box_bounds) => (spec.origin, box_bounds),
            None => ((left, top), [0.0, 0.0, right - left, bottom - top]),
        };
        return Some(TypePlacement {
            rect: crate::type_layer::flat_rect(
                (origin.0 + box_bounds[0]) as f32,
                (origin.1 + box_bounds[1]) as f32,
                (box_bounds[2] - box_bounds[0]) as f32,
                (box_bounds[3] - box_bounds[1]) as f32,
            ),
            bounds: box_bounds,
        });
    }
    let bundled = face_for(&spec.character.font_family, &spec.character.font_style)?;
    let size = spec.character.size as f32;
    if size.is_nan() || size <= 0.0 {
        return None;
    }
    let ascent = f64::from(bundled.ascent(size));
    let descent = f64::from(bundled.descent(size));
    let run = RunStyle::of(bundled, &spec.character);
    let overhang = f64::from(run.embolden) + ascent * f64::from(run.skew);
    let layout = layout_path(spec)?;
    let origin = spec.origin;
    let mut min = (f64::MAX, f64::MAX);
    let mut max = (f64::MIN, f64::MIN);
    let mut include = |x: f64, y: f64| {
        min.0 = min.0.min(x);
        min.1 = min.1.min(y);
        max.0 = max.0.max(x);
        max.1 = max.1.max(y);
    };
    for glyph in &layout.glyphs {
        let corners = [
            (-overhang, -ascent),
            (glyph.advance + overhang, -ascent),
            (glyph.advance + overhang, descent),
            (-overhang, descent),
        ];
        let (sin, cos) = glyph.angle.sin_cos();
        for (u, v) in corners {
            include(glyph.x + u * cos - v * sin, glyph.y + u * sin + v * cos);
        }
    }
    if layout.glyphs.is_empty() {
        include(origin.0 - 1.0, origin.1 - ascent);
        include(origin.0 + 1.0, origin.1 + descent);
    }
    let left = min.0 - 1.0;
    let top = min.1 - 1.0;
    let right = max.0 + 1.0;
    let bottom = max.1 + 1.0;
    Some(TypePlacement {
        rect: crate::type_layer::flat_rect(
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ),
        bounds: [
            left - origin.0,
            top - origin.1,
            right - origin.0,
            bottom - origin.1,
        ],
    })
}

/// Render on-path text over `rect`.
pub(crate) fn render_path(
    spec: &TypeSpec,
    rect: pictura_core::PsdRect,
) -> Option<pictura_core::PixelBuffer> {
    let layout = layout_path(spec)?;
    let bundled = face_for(&spec.character.font_family, &spec.character.font_style)?;
    let run = RunStyle::of(bundled, &spec.character);
    let color = spec.rgba().map(|c| f64::from(c) / 255.0);
    let mut out = pictura_core::PixelBuffer::new(rect.width() as u32, rect.height() as u32, 4);
    let faces = crate::fonts::faces_in_order(bundled);
    let mut painted = false;
    for glyph in &layout.glyphs {
        let fx = glyph.x - f64::from(rect.left);
        let fy = glyph.y - f64::from(rect.top);
        let px = fx.floor();
        let py = fy.floor();
        let request = pictura_core::RasterRequest {
            glyph: glyph.id,
            px_size: run.size,
            subpixel_x: (fx - px) as f32,
            subpixel_y: (py - fy) as f32,
        };
        let (sin, cos) = glyph.angle.sin_cos();
        let rotation = [cos as f32, sin as f32, -sin as f32, cos as f32];
        let face = faces.get(glyph.face as usize).copied().unwrap_or(bundled);
        let rasterizer = face.rasterizer();
        let mut matrix = glyph_matrix(&run, glyph.scale);
        matrix[0] *= glyph.x_scale;
        matrix[2] *= glyph.x_scale;
        let outline = compose(rotation, matrix);
        if let Some(mask) = rasterizer.rasterize_styled(&request, outline, run.embolden) {
            painted |= blit_glyph(&mut out, &mask, px as i32, py as i32, color);
        }
    }
    painted.then_some(out)
}

/// Render area type: paragraph layout in the path box, clipped to the path
/// interior.
pub(crate) fn render_area(
    spec: &TypeSpec,
    rect: pictura_core::PsdRect,
) -> Option<pictura_core::PixelBuffer> {
    let path = spec.path.as_ref()?;
    let mut view_spec = spec.clone();
    if view_spec.box_bounds.is_none() {
        let (left, top, right, bottom) = bounds(&path.subpath.flattened(FLATTEN).points)?;
        view_spec.origin = (left, top);
        view_spec.box_bounds = Some([0.0, 0.0, right - left, bottom - top]);
    }
    let mut buffer = crate::text_render::render_text_buffer(
        &render_view(&view_spec),
        rect.width(),
        rect.height(),
    )?;
    let flattened = path.subpath.flattened(FLATTEN);
    if !flattened.closed || flattened.points.len() < 3 {
        return Some(buffer);
    }
    let plane = buffer.pixel_count();
    for y in 0..rect.height() {
        for x in 0..rect.width() {
            let mut inside = 0;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let px = f64::from(rect.left) + f64::from(x) + sx;
                let py = f64::from(rect.top) + f64::from(y) + sy;
                if flattened.contains(px, py) {
                    inside += 1;
                }
            }
            if inside == 4 {
                continue;
            }
            let index = (y as usize) * (rect.width() as usize) + x as usize;
            if inside == 0 {
                for channel in 0..4 {
                    buffer.data[channel * plane + index] = 0;
                }
            } else {
                let scale = inside as f32 / 4.0;
                for channel in 0..4 {
                    let slot = &mut buffer.data[channel * plane + index];
                    *slot = (f32::from(*slot) * scale).round() as u8;
                }
            }
        }
    }
    Some(buffer)
}

/// The caret at every UTF-16 boundary, perpendicular to the path tangent.
pub(crate) fn caret_stops(spec: &TypeSpec) -> Vec<[f64; 4]> {
    let Some(path_spec) = spec.path.as_ref() else {
        return Vec::new();
    };
    if path_spec.kind != TextPathKind::OnPath {
        return Vec::new();
    }
    let Some(bundled) = face_for(&spec.character.font_family, &spec.character.font_style) else {
        return Vec::new();
    };
    let size = spec.character.size as f32;
    if size.is_nan() || size <= 0.0 {
        return Vec::new();
    }
    let flattened = path_spec.subpath.flattened(FLATTEN);
    let start = if flattened.closed {
        path_spec.start.rem_euclid(flattened.total.max(1e-9))
    } else {
        path_spec.start.clamp(0.0, flattened.total)
    };
    let chars: Vec<char> = path_text(spec).chars().collect();
    let ascent = f64::from(bundled.ascent(size));
    let descent = f64::from(bundled.descent(size));
    let rise = f64::from(RunStyle::of(bundled, &spec.character).rise);
    let mut stops = Vec::new();
    for k in 0..=chars.len() {
        let advance = if k == 0 {
            0.0
        } else {
            f64::from(crate::type_paragraph::line_advance_with(
                bundled,
                &chars[..k].iter().collect::<String>(),
                &spec.character,
                &spec.paragraph,
            ))
        };
        let arc = if flattened.closed {
            start + advance
        } else {
            (start + advance).min(flattened.total)
        };
        let Some((position, tangent)) = point_at(&flattened, arc) else {
            continue;
        };
        let normal = (-tangent.1, tangent.0);
        let origin = (position.0 - normal.0 * rise, position.1 - normal.1 * rise);
        let stop = [
            origin.0 - normal.0 * ascent,
            origin.1 - normal.1 * ascent,
            origin.0 + normal.0 * descent,
            origin.1 + normal.1 * descent,
        ];
        let repeat = chars.get(k).map_or(1, |c| c.len_utf16());
        for _ in 0..repeat {
            stops.push(stop);
        }
    }
    stops
}

/// The path as the baseline guide, sampled every few document pixels.
pub(crate) fn baselines(spec: &TypeSpec) -> Vec<[f64; 4]> {
    let Some(path_spec) = spec.path.as_ref() else {
        return Vec::new();
    };
    if path_spec.kind != TextPathKind::OnPath {
        return Vec::new();
    }
    let flattened = path_spec.subpath.flattened(FLATTEN);
    if flattened.total <= 0.0 {
        return Vec::new();
    }
    let start = if flattened.closed {
        path_spec.start.rem_euclid(flattened.total)
    } else {
        path_spec.start.clamp(0.0, flattened.total)
    };
    let length = if flattened.closed {
        flattened.total
    } else {
        flattened.total - start
    };
    let steps = ((length / 8.0).ceil() as usize).max(1);
    let mut out = Vec::with_capacity(steps);
    let mut previous = point_at(&flattened, start).map(|(p, _)| p);
    for i in 1..=steps {
        let arc = start + length * i as f64 / steps as f64;
        let Some((point, _)) = point_at(&flattened, arc) else {
            continue;
        };
        if let Some(previous) = previous {
            out.push([previous.0, previous.1, point.0, point.1]);
        }
        previous = Some(point);
    }
    out
}

/// Where the overflow `+` goes on an on-path session.
pub fn overflow_point(spec: &TypeSpec) -> Option<(f64, f64)> {
    let layout = layout_path(spec)?;
    layout.overflow.then_some(layout.end)
}

/// The path hit under `(x, y)`: the nearest edge within `tolerance`, else a
/// closed path containing the point.
pub fn path_hit_at(
    doc: &pictura_core::Document,
    x: f64,
    y: f64,
    tolerance: f64,
) -> Option<PathHit> {
    let mut edge: Option<(f64, PathHit)> = None;
    let mut area: Option<PathHit> = None;
    let mut consider = |subpath: &Subpath| {
        let flattened = subpath.flattened(FLATTEN);
        if let Some((arc, distance)) = flattened.nearest(x, y) {
            if distance <= tolerance && edge.as_ref().is_none_or(|(d, _)| distance < *d) {
                edge = Some((
                    distance,
                    PathHit {
                        kind: TextPathKind::OnPath,
                        subpath: subpath.clone(),
                        start: arc,
                    },
                ));
            }
        }
        if area.is_none() && flattened.contains(x, y) {
            area = Some(PathHit {
                kind: TextPathKind::Area,
                subpath: subpath.clone(),
                start: 0.0,
            });
        }
    };
    for subpath in &doc.work_path.subpaths {
        consider(subpath);
    }
    for saved in &doc.saved_paths {
        for subpath in &saved.path.subpaths {
            consider(subpath);
        }
    }
    edge.map(|(_, hit)| hit).or(area)
}

fn bounds(points: &[(f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    let first = points.first()?;
    let mut left = first.0;
    let mut top = first.1;
    let mut right = first.0;
    let mut bottom = first.1;
    for &(x, y) in points {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    (right > left && bottom > top).then_some((left, top, right, bottom))
}

/// `a · b` for linear parts in the `TySh` order (`xx, xy, yx, yy`).
fn compose(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::path::{PathPoint, Subpath};

    fn open_path() -> Subpath {
        Subpath {
            points: vec![
                PathPoint {
                    anchor: (0.0, 100.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (300.0, 100.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed: false,
        }
    }

    fn closed_rect(w: f64, h: f64) -> Subpath {
        Subpath {
            points: vec![
                PathPoint {
                    anchor: (0.0, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (w, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (w, h),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (0.0, h),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed: true,
        }
    }

    fn spec(text: &str, path: Subpath, start: f64, kind: TextPathKind) -> TypeSpec {
        let mut spec = TypeSpec::new(text, "Liberation Sans", 20.0);
        spec.path = Some(TextPath {
            kind,
            subpath: path,
            start,
        });
        spec
    }

    #[test]
    fn path_text_starts_at_click_arc() {
        let spec = spec("abcd", open_path(), 150.0, TextPathKind::OnPath);
        let layout = layout_path(&spec).expect("lays out");
        assert!(!layout.glyphs.is_empty());
        assert!((layout.glyphs[0].x - 150.0).abs() < 0.5);
        assert!((layout.glyphs[0].y - 100.0).abs() < 0.5);
        assert!(layout.glyphs[1].x > layout.glyphs[0].x);
    }

    #[test]
    fn path_text_wraps_closed_path() {
        let path = closed_rect(200.0, 100.0);
        let circumference = 600.0_f64;
        let short = spec("abc", path.clone(), 590.0, TextPathKind::OnPath);
        let layout = layout_path(&short).expect("lays out");
        assert!(layout.glyphs.len() >= 3);
        assert!(!layout.overflow, "a short run wraps around the start");
        let long = spec(&"M".repeat(200), path, 0.0, TextPathKind::OnPath);
        assert!(layout_path(&long).expect("lays out").overflow);
        assert!((circumference - 600.0).abs() < 1e-6);
    }

    #[test]
    fn area_text_clips_to_closed_path() {
        let triangle = Subpath {
            points: vec![
                PathPoint {
                    anchor: (0.0, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (120.0, 0.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
                PathPoint {
                    anchor: (60.0, 120.0),
                    in_handle: None,
                    out_handle: None,
                    smooth: false,
                },
            ],
            closed: true,
        };
        let mut spec = spec("MMMM", triangle, 0.0, TextPathKind::Area);
        spec.character.fill_color = [1.0, 0.0, 0.0, 1.0];
        let placement = placement(&spec).expect("placement");
        let rect = placement.rect;
        let buffer = render_area(&spec, rect).expect("renders");
        let plane = buffer.pixel_count();
        let mut painted = 0;
        for y in 0..rect.height() {
            for x in 0..rect.width() {
                let i = (y as usize) * (rect.width() as usize) + x as usize;
                if buffer.data[3 * plane + i] == 0 {
                    continue;
                }
                painted += 1;
                let px = f64::from(rect.left) + f64::from(x) + 0.5;
                let py = f64::from(rect.top) + f64::from(y) + 0.5;
                let path = spec.path.as_ref().unwrap().subpath.flattened(FLATTEN);
                let on_edge = path
                    .nearest(px, py)
                    .is_some_and(|(_, distance)| distance <= 1.0);
                assert!(
                    path.contains(px, py) || on_edge,
                    "painted pixel outside the triangle: {px}, {py}"
                );
            }
        }
        assert!(painted > 0);
    }

    #[test]
    fn path_hit_prefers_edge_then_inside() {
        let mut doc = pictura_core::Document::new(
            200,
            200,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.work_path.add_subpath(closed_rect(100.0, 100.0));
        let edge = path_hit_at(&doc, 50.0, 1.0, 4.0).expect("edge");
        assert_eq!(edge.kind, TextPathKind::OnPath);
        assert!((edge.start - 50.0).abs() < 2.0);
        let inside = path_hit_at(&doc, 50.0, 50.0, 4.0).expect("inside");
        assert_eq!(inside.kind, TextPathKind::Area);
    }

    #[test]
    fn path_caret_follows_tangent() {
        let spec = spec("ab", open_path(), 0.0, TextPathKind::OnPath);
        let stops = caret_stops(&spec);
        assert_eq!(stops.len(), 3);
        assert!((stops[0][0] - 0.0).abs() < 1.0);
        assert!(stops[1][0] > stops[0][0]);
        assert!((stops[0][0] - stops[0][2]).abs() < 1e-6);
    }

    #[test]
    fn type_on_path_applies_baseline_shift() {
        let base = spec("ab", open_path(), 0.0, TextPathKind::OnPath);
        let mut raised = base.clone();
        raised.character.baseline_shift = 20.0;

        let a = layout_path(&base).expect("base layout");
        let b = layout_path(&raised).expect("raised layout");
        assert!(!a.glyphs.is_empty() && !b.glyphs.is_empty());
        assert!((a.glyphs[0].y - b.glyphs[0].y - 20.0).abs() < 0.5);

        let ca = caret_stops(&base);
        let cb = caret_stops(&raised);
        assert!((ca[0][1] - cb[0][1] - 20.0).abs() < 0.5);

        // Vertical type turns the glyph, but the shift stays along the path
        // normal (CS6 moves type on/off the path), so the offset is the same.
        let mut vertical = raised.clone();
        vertical.vertical = true;
        let v = layout_path(&vertical).expect("vertical layout");
        assert!((a.glyphs[0].y - v.glyphs[0].y - 20.0).abs() < 0.5);

        let pa = placement(&base).expect("base placement");
        let pb = placement(&raised).expect("raised placement");
        assert!((pa.bounds[1] - pb.bounds[1] - 20.0).abs() < 1.0);
    }
}
