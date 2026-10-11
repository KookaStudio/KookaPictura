//! The geometry behind the Rectangle, Rounded Rectangle, Ellipse, and Polygon
//! tools (`docs/03-tools/shape-tools.md`).
//!
//! The tools differ only in the outline a drag produces; what becomes of it (a
//! shape layer, a Work Path component, or pixels) is the caller's business. The
//! outline is one closed [`Subpath`] in the Work Path's own model, so curves
//! stay cubic Beziers rather than being flattened here: the live preview, the
//! path, and the vector mask all draw the same curve.
//!
//! Ported from photorust's `core/src/shape.rs`, which flattened every outline
//! to points; here arcs are quarter-circle cubics. The Line and the built-in
//! custom shapes live in [`line`] and [`custom`].

mod custom;
mod line;

pub use custom::{custom_shape_kernel, custom_shape_preview, CUSTOM_SHAPE_NAMES};

use crate::path::{PathPoint, Subpath, VectorPath};

/// The cubic handle length, as a fraction of the radius, that best fits a
/// quarter circle.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// Flattening tolerance for [`coverage`] and [`path_coverage`], in document pixels.
const COVERAGE_TOLERANCE: f64 = 0.1;

/// Vertical samples per pixel row in [`coverage`] and [`path_coverage`].
const SUBROWS: usize = 4;

/// Which shape tool drew the outline.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Polygon,
    Line,
    CustomShape,
}

impl ShapeKind {
    /// The bridge's integer code: 0 Rectangle, 1 Rounded Rectangle, 2 Ellipse,
    /// 3 Polygon, 4 Line, 5 Custom Shape.
    pub fn from_i32(v: i32) -> Option<ShapeKind> {
        match v {
            0 => Some(ShapeKind::Rectangle),
            1 => Some(ShapeKind::RoundedRectangle),
            2 => Some(ShapeKind::Ellipse),
            3 => Some(ShapeKind::Polygon),
            4 => Some(ShapeKind::Line),
            5 => Some(ShapeKind::CustomShape),
            _ => None,
        }
    }

    /// A shape layer is named after the tool that drew it (CS6 calls a custom
    /// shape's layer "Shape").
    pub fn layer_name(self) -> &'static str {
        match self {
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::RoundedRectangle => "Rounded Rectangle",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Polygon => "Polygon",
            ShapeKind::Line => "Line",
            ShapeKind::CustomShape => "Shape",
        }
    }
}

/// The geometry options a tool reads: the Rounded Rectangle's corner `radii`
/// in pixels (top-left, top-right, bottom-right, bottom-left), and the
/// Polygon's `sides` (clamped to 3–100), `star` indent (the percentage of the
/// radius the indents take, 1–99; `None` for a plain polygon), and whether its
/// corners and indents are smooth; the Line's `weight` (px) and `arrows`; the
/// Custom Shape's index into [`CUSTOM_SHAPE_NAMES`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShapeOptions {
    pub kind: ShapeKind,
    pub radii: [f64; 4],
    pub sides: u32,
    pub star: Option<f64>,
    pub smooth_corners: bool,
    pub smooth_indents: bool,
    pub weight: f64,
    pub arrows: Arrowheads,
    pub custom: usize,
}

/// The Line's arrowheads: at the `start` and / or `end`, `width` and `length`
/// as percentages of the line's weight (10–1000 and 10–5000), and `concavity`
/// (−50–50 %) pulling the base in toward the tip (out when negative).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Arrowheads {
    pub start: bool,
    pub end: bool,
    pub width: f64,
    pub length: f64,
    pub concavity: f64,
}

impl Default for Arrowheads {
    /// No arrowheads, sized 500 % wide and 1000 % long when switched on, as
    /// CS6's geometry pop-up starts.
    fn default() -> Self {
        Self {
            start: false,
            end: false,
            width: 500.0,
            length: 1000.0,
            concavity: 0.0,
        }
    }
}

impl ShapeOptions {
    /// `kind` with every corner `radius`, `sides`, and no star or smoothing.
    pub fn new(kind: ShapeKind, radius: f64, sides: u32) -> Self {
        Self {
            kind,
            radii: [radius; 4],
            sides,
            star: None,
            smooth_corners: false,
            smooth_indents: false,
            weight: 1.0,
            arrows: Arrowheads::default(),
            custom: 0,
        }
    }
}

/// The closed outline a drag from `from` to `to` draws, or `None` when the
/// drag encloses no area.
///
/// The rectangle-based tools square off under `shift` (Circle for the
/// Ellipse) and grow from `from` under `alt`. The Polygon is always centred on
/// `from`, the drag is its radius, its first vertex sits under the pointer, and
/// `shift` snaps that rotation to 15°. The Line runs from `from` to `to`, Shift
/// snapping it to 45°. A Custom Shape keeps its designed proportions under
/// `shift`.
pub fn outline(
    options: ShapeOptions,
    from: (f64, f64),
    to: (f64, f64),
    shift: bool,
    alt: bool,
) -> Option<Subpath> {
    let aspect = match options.kind {
        ShapeKind::Polygon => return polygon(options, from, to, shift),
        ShapeKind::Line => return line::line(from, to, options.weight, options.arrows, shift),
        ShapeKind::CustomShape => custom::aspect(options.custom),
        _ => 1.0,
    };
    outline_in_box(options, drag_rect(from, to, shift.then_some(aspect), alt))
}

/// The closed outline of `options.kind` filling the box `(x, y, w, h)`, as the
/// Create dialogs place it; `None` for an empty box. The Polygon stands upright
/// (first corner at the top) and a Custom Shape is stretched to touch all four
/// sides. A Line has no box form.
pub fn outline_in_box(options: ShapeOptions, rect: (f64, f64, f64, f64)) -> Option<Subpath> {
    if !(rect.2 > 0.0 && rect.3 > 0.0) {
        return None;
    }
    Some(match options.kind {
        ShapeKind::Rectangle => rectangle(rect),
        ShapeKind::RoundedRectangle => rounded_rectangle(rect, options.radii),
        ShapeKind::Ellipse => ellipse(rect),
        ShapeKind::Polygon => {
            let upright = -std::f64::consts::FRAC_PI_2;
            let unit = map(&unit_polygon(options), |(x, y)| {
                let (sin, cos) = upright.sin_cos();
                (x * cos - y * sin, x * sin + y * cos)
            });
            fit(&unit, rect)?
        }
        ShapeKind::CustomShape => fit(&custom::unit_shape(options.custom), rect)?,
        ShapeKind::Line => return None,
    })
}

/// The `(x, y, w, h)` a drag marks out. A `constrain` aspect (width over
/// height; 1 squares it) grows the box by the drag's longer reach, so it never
/// pulls back from the pointer; Alt makes the drag a half-diagonal from the
/// centre.
fn drag_rect(
    from: (f64, f64),
    to: (f64, f64),
    constrain: Option<f64>,
    alt: bool,
) -> (f64, f64, f64, f64) {
    let (mut dx, mut dy) = (to.0 - from.0, to.1 - from.1);
    if let Some(aspect) = constrain.filter(|a| *a > 0.0) {
        let height = (dx.abs() / aspect).max(dy.abs());
        dx = (height * aspect).copysign(dx);
        dy = height.copysign(dy);
    }
    if alt {
        (
            from.0 - dx.abs(),
            from.1 - dy.abs(),
            dx.abs() * 2.0,
            dy.abs() * 2.0,
        )
    } else {
        (
            from.0.min(from.0 + dx),
            from.1.min(from.1 + dy),
            dx.abs(),
            dy.abs(),
        )
    }
}

fn point(
    anchor: (f64, f64),
    in_handle: Option<(f64, f64)>,
    out_handle: Option<(f64, f64)>,
) -> PathPoint {
    PathPoint {
        anchor,
        in_handle,
        out_handle,
        smooth: in_handle.is_some() && out_handle.is_some(),
    }
}

fn closed(points: Vec<PathPoint>) -> Subpath {
    Subpath {
        points,
        closed: true,
    }
}

/// Four corners, clockwise from the top-left.
fn rectangle((x, y, w, h): (f64, f64, f64, f64)) -> Subpath {
    closed(
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
            .into_iter()
            .map(|a| point(a, None, None))
            .collect(),
    )
}

/// A rectangle whose corners are quarter circles of `radii` (top-left,
/// top-right, bottom-right, bottom-left), each clamped to half the shorter
/// side (a larger radius would cross the opposite corner). A rounded corner is
/// two anchors joined by one cubic, a zero one a plain corner; where the clamp
/// leaves no straight edge between two arcs, their anchors merge.
fn rounded_rectangle(rect: (f64, f64, f64, f64), radii: [f64; 4]) -> Subpath {
    let (x, y, w, h) = rect;
    let [tl, tr, br, bl] = radii.map(|r| r.max(0.0).min(w.min(h) / 2.0));
    let (right, bottom) = (x + w, y + h);
    // Each corner: the corner point, its radius, and the unit directions to the
    // anchor before it and the anchor after it (clockwise).
    let corners = [
        ((x, y), tl, (0.0, 1.0), (1.0, 0.0)),
        ((right, y), tr, (-1.0, 0.0), (0.0, 1.0)),
        ((right, bottom), br, (0.0, -1.0), (-1.0, 0.0)),
        ((x, bottom), bl, (1.0, 0.0), (0.0, -1.0)),
    ];
    let mut raw = Vec::with_capacity(8);
    for ((cx, cy), r, before, after) in corners {
        if r <= 0.0 {
            raw.push(point((cx, cy), None, None));
            continue;
        }
        let k = r * (1.0 - KAPPA);
        let start = (cx + before.0 * r, cy + before.1 * r);
        let end = (cx + after.0 * r, cy + after.1 * r);
        raw.push(point(
            start,
            None,
            Some((cx + before.0 * k, cy + before.1 * k)),
        ));
        raw.push(point(end, Some((cx + after.0 * k, cy + after.1 * k)), None));
    }
    // The top-left arc ends the outline, so the top edge starts it.
    raw.rotate_left(usize::from(tl > 0.0));
    let mut points: Vec<PathPoint> = Vec::with_capacity(raw.len());
    for p in raw {
        match points.last_mut() {
            Some(last) if same(last.anchor, p.anchor) => {
                last.out_handle = p.out_handle;
                last.smooth = true;
            }
            _ => points.push(p),
        }
    }
    closed(points)
}

/// Anchors the radius clamp meant to coincide, up to rounding.
fn same(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

/// The ellipse inscribed in the rectangle: four smooth anchors, clockwise from
/// the top.
fn ellipse((x, y, w, h): (f64, f64, f64, f64)) -> Subpath {
    let (rx, ry) = (w / 2.0, h / 2.0);
    let (cx, cy) = (x + rx, y + ry);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    closed(vec![
        point((cx, y), Some((cx - kx, y)), Some((cx + kx, y))),
        point((x + w, cy), Some((x + w, cy - ky)), Some((x + w, cy + ky))),
        point((cx, y + h), Some((cx + kx, y + h)), Some((cx - kx, y + h))),
        point((x, cy), Some((x, cy + ky)), Some((x, cy - ky))),
    ])
}

/// The polygon on the circle through `to` centred on `centre`, the first
/// corner under `to`.
fn polygon(
    options: ShapeOptions,
    centre: (f64, f64),
    to: (f64, f64),
    shift: bool,
) -> Option<Subpath> {
    let (dx, dy) = (to.0 - centre.0, to.1 - centre.1);
    let radius = dx.hypot(dy);
    if radius <= 0.0 {
        return None;
    }
    let mut rotation = dy.atan2(dx);
    if shift {
        let step = std::f64::consts::PI / 12.0;
        rotation = (rotation / step).round() * step;
    }
    let (sin, cos) = rotation.sin_cos();
    Some(map(&unit_polygon(options), |(x, y)| {
        (
            centre.0 + radius * (x * cos - y * sin),
            centre.1 + radius * (x * sin + y * cos),
        )
    }))
}

/// The polygon (or star) on the unit circle, the first corner at angle 0.
/// A star alternates corners with indents at `1 - star / 100` of the radius.
/// A smooth corner or indent passes a curve through it whose tangent runs
/// parallel to the chord between its neighbours (a Catmull-Rom tangent): an
/// approximation, CS6's smoothing is unpublished.
fn unit_polygon(options: ShapeOptions) -> Subpath {
    let sides = options.sides.clamp(3, 100) as usize;
    let indent = options.star.map(|p| 1.0 - p.clamp(1.0, 99.0) / 100.0);
    let count = if indent.is_some() { sides * 2 } else { sides };
    let anchors: Vec<(f64, f64, bool)> = (0..count)
        .map(|i| {
            let inner = indent.is_some() && i % 2 == 1;
            let r = if inner { indent.unwrap_or(1.0) } else { 1.0 };
            let angle = i as f64 / count as f64 * std::f64::consts::TAU;
            (r * angle.cos(), r * angle.sin(), inner)
        })
        .collect();
    closed(
        (0..count)
            .map(|i| {
                let (x, y, inner) = anchors[i];
                let smooth = if inner {
                    options.smooth_indents
                } else {
                    options.smooth_corners
                };
                if !smooth {
                    return point((x, y), None, None);
                }
                let prev = anchors[(i + count - 1) % count];
                let next = anchors[(i + 1) % count];
                let (tx, ty) = ((next.0 - prev.0) / 6.0, (next.1 - prev.1) / 6.0);
                point((x, y), Some((x - tx, y - ty)), Some((x + tx, y + ty)))
            })
            .collect(),
    )
}

/// `subpath` with `f` applied to every anchor and handle; affine maps keep the
/// curve exact.
fn map(subpath: &Subpath, f: impl Fn((f64, f64)) -> (f64, f64)) -> Subpath {
    Subpath {
        points: subpath
            .points
            .iter()
            .map(|p| PathPoint {
                anchor: f(p.anchor),
                in_handle: p.in_handle.map(&f),
                out_handle: p.out_handle.map(&f),
                smooth: p.smooth,
            })
            .collect(),
        closed: subpath.closed,
    }
}

/// `subpath` stretched so its curve exactly fills `(x, y, w, h)`.
fn fit(subpath: &Subpath, (x, y, w, h): (f64, f64, f64, f64)) -> Option<Subpath> {
    let flat = subpath.flatten(1e-4);
    let (l, t, r, b) = flat.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(l, t, r, b), &(px, py)| (l.min(px), t.min(py), r.max(px), b.max(py)),
    );
    if !(r > l && b > t) {
        return None;
    }
    let (sx, sy) = (w / (r - l), h / (b - t));
    Some(map(subpath, |(px, py)| {
        (x + (px - l) * sx, y + (py - t) * sy)
    }))
}

/// CS6's Align Edges: every straight horizontal or vertical segment has its
/// shared coordinate rounded to the pixel grid, so the edge lands on pixel
/// boundaries and renders crisp; curves and slanted edges keep their points.
pub fn align_edges(subpath: &Subpath) -> Subpath {
    let mut out = subpath.clone();
    let n = out.points.len();
    for i in 0..subpath.segment_count() {
        let j = (i + 1) % n;
        let (a, b) = (&subpath.points[i], &subpath.points[j]);
        if a.out_handle.is_some() || b.in_handle.is_some() {
            continue;
        }
        if (a.anchor.0 - b.anchor.0).abs() < 1e-9 {
            let x = a.anchor.0.round();
            out.points[i].anchor.0 = x;
            out.points[j].anchor.0 = x;
        }
        if (a.anchor.1 - b.anchor.1).abs() < 1e-9 {
            let y = a.anchor.1.round();
            out.points[i].anchor.1 = y;
            out.points[j].anchor.1 = y;
        }
    }
    out
}

/// A `width` × `height` document-sized, row-major, anti-aliased coverage mask
/// (0–255) of `subpath`'s interior, even-odd. Each pixel row is sampled on
/// [`SUBROWS`] scanlines and each span's ends are weighted by the fraction of
/// the pixel they cover. An open subpath is closed implicitly.
pub fn coverage(subpath: &Subpath, width: u32, height: u32) -> Vec<u8> {
    rasterize(&[subpath.flatten(COVERAGE_TOLERANCE)], width, height, |w| {
        w % 2 != 0
    })
}

/// [`coverage`] of every subpath of `path` together under the nonzero winding
/// rule, as CS6 fills a path or loads it as a selection: an inner subpath
/// drawn in the same direction stays filled, an opposite one cuts a hole. Open
/// subpaths are closed implicitly; one under three points encloses nothing.
pub fn path_coverage(path: &VectorPath, width: u32, height: u32) -> Vec<u8> {
    let contours: Vec<_> = path
        .subpaths
        .iter()
        .map(|sp| sp.flatten(COVERAGE_TOLERANCE))
        .collect();
    rasterize(&contours, width, height, |w| w != 0)
}

/// Scanline-fill the implicitly closed `contours`, a pixel sample being inside
/// where `inside(winding)` holds.
fn rasterize(
    contours: &[Vec<(f64, f64)>],
    width: u32,
    height: u32,
    inside: impl Fn(i32) -> bool,
) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let mut mask = vec![0u8; w * h];
    let contours: Vec<_> = contours.iter().filter(|c| c.len() >= 3).collect();
    if contours.is_empty() || w == 0 || h == 0 {
        return mask;
    }
    let (top, bottom) = contours
        .iter()
        .flat_map(|c| c.iter())
        .fold((f64::MAX, f64::MIN), |(t, b), p| (t.min(p.1), b.max(p.1)));
    let first_row = top.floor().max(0.0) as usize;
    let last_row = (bottom.ceil().max(0.0) as usize).min(h);
    let mut row = vec![0f32; w + 1];
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for y in first_row..last_row {
        row.iter_mut().for_each(|c| *c = 0.0);
        for s in 0..SUBROWS {
            let sy = y as f64 + (s as f64 + 0.5) / SUBROWS as f64;
            crossings.clear();
            for contour in &contours {
                let mut j = contour.len() - 1;
                for (i, &(xi, yi)) in contour.iter().enumerate() {
                    let (xj, yj) = contour[j];
                    if (yi > sy) != (yj > sy) {
                        let x = xi + (sy - yi) / (yj - yi) * (xj - xi);
                        crossings.push((x, if yi > yj { 1 } else { -1 }));
                    }
                    j = i;
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            let mut span_start = None;
            for &(x, dir) in &crossings {
                winding += dir;
                match (span_start, inside(winding)) {
                    (None, true) => span_start = Some(x),
                    (Some(x0), false) => {
                        add_span(&mut row, f64::max(x0, 0.0), x.min(w as f64));
                        span_start = None;
                    }
                    _ => {}
                }
            }
        }
        for (x, c) in row[..w].iter().enumerate() {
            mask[y * w + x] = (c / SUBROWS as f32 * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    mask
}

/// Accumulate horizontal coverage of `[x0, x1)` into `row` (one cell per pixel).
fn add_span(row: &mut [f32], x0: f64, x1: f64) {
    if x1 <= x0 {
        return;
    }
    let (first, last) = (x0.floor() as usize, x1.floor() as usize);
    if first == last {
        row[first] += (x1 - x0) as f32;
        return;
    }
    row[first] += (first as f64 + 1.0 - x0) as f32;
    for c in &mut row[first + 1..last] {
        *c += 1.0;
    }
    if last < row.len() {
        row[last] += (x1 - last as f64) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::VectorPath;

    fn opts(kind: ShapeKind) -> ShapeOptions {
        ShapeOptions::new(kind, 10.0, 5)
    }

    fn anchors(s: &Subpath) -> Vec<(f64, f64)> {
        s.points.iter().map(|p| p.anchor).collect()
    }

    /// The flattened curve's `(left, top, right, bottom)`.
    fn bounds(s: &Subpath) -> (f64, f64, f64, f64) {
        let mut path = VectorPath::default();
        path.add_subpath(s.clone());
        path.subpath_bounds(0).expect("non-empty")
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_rectangle_is_four_corners_clockwise() {
        let s = outline(
            opts(ShapeKind::Rectangle),
            (40.0, 60.0),
            (10.0, 20.0),
            false,
            false,
        )
        .unwrap();
        assert!(s.closed);
        assert_eq!(
            anchors(&s),
            vec![(10.0, 20.0), (40.0, 20.0), (40.0, 60.0), (10.0, 60.0)]
        );
        assert!(s
            .points
            .iter()
            .all(|p| p.in_handle.is_none() && p.out_handle.is_none()));
    }

    #[test]
    fn shift_squares_by_the_longer_side_and_alt_grows_from_the_centre() {
        let sq = outline(
            opts(ShapeKind::Rectangle),
            (0.0, 0.0),
            (40.0, -10.0),
            true,
            false,
        )
        .unwrap();
        assert_eq!(bounds(&sq), (0.0, -40.0, 40.0, 0.0));
        let centred = outline(
            opts(ShapeKind::Rectangle),
            (50.0, 50.0),
            (60.0, 70.0),
            false,
            true,
        )
        .unwrap();
        assert_eq!(bounds(&centred), (40.0, 30.0, 60.0, 70.0));
    }

    #[test]
    fn a_drag_that_encloses_nothing_draws_nothing() {
        for kind in [
            ShapeKind::Rectangle,
            ShapeKind::RoundedRectangle,
            ShapeKind::Ellipse,
            ShapeKind::Polygon,
        ] {
            assert!(outline(opts(kind), (5.0, 5.0), (5.0, 5.0), false, false).is_none());
        }
        assert!(outline(
            opts(ShapeKind::Rectangle),
            (5.0, 5.0),
            (30.0, 5.0),
            false,
            false
        )
        .is_none());
    }

    #[test]
    fn rounded_corners_are_quarter_circles_inside_the_box() {
        let s = outline(
            opts(ShapeKind::RoundedRectangle),
            (0.0, 0.0),
            (100.0, 60.0),
            false,
            false,
        )
        .unwrap();
        assert_eq!(s.points.len(), 8);
        let (l, t, r, b) = bounds(&s);
        assert!(near(l, 0.0) && near(t, 0.0) && near(r, 100.0) && near(b, 60.0));
        // The top-left arc's midpoint sits on the circle of radius 10 about (10, 10).
        let seg = s.segment(7).unwrap();
        let mid = (
            (seg[0].0 + 3.0 * seg[1].0 + 3.0 * seg[2].0 + seg[3].0) / 8.0,
            (seg[0].1 + 3.0 * seg[1].1 + 3.0 * seg[2].1 + seg[3].1) / 8.0,
        );
        assert!(((mid.0 - 10.0).hypot(mid.1 - 10.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn a_huge_radius_clamps_to_a_stadium_without_duplicate_anchors() {
        let mut o = opts(ShapeKind::RoundedRectangle);
        o.radii = [500.0; 4];
        let s = outline(o, (0.0, 0.0), (100.0, 40.0), false, false).unwrap();
        assert_eq!(
            s.points.len(),
            6,
            "the vertical edges collapse into arc joins"
        );
        let (l, t, r, b) = bounds(&s);
        assert!(near(l, 0.0) && near(t, 0.0) && near(r, 100.0) && near(b, 40.0));
        let circle = outline(o, (0.0, 0.0), (40.0, 40.0), false, false).unwrap();
        assert_eq!(circle.points.len(), 4);
        assert!(circle.points.iter().all(|p| p.smooth));
    }

    #[test]
    fn an_ellipse_is_four_smooth_anchors_filling_its_box() {
        let s = outline(
            opts(ShapeKind::Ellipse),
            (0.0, 0.0),
            (80.0, 40.0),
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            anchors(&s),
            vec![(40.0, 0.0), (80.0, 20.0), (40.0, 40.0), (0.0, 20.0)]
        );
        assert!(s.points.iter().all(|p| p.smooth));
        let (l, t, r, b) = bounds(&s);
        assert!(near(l, 0.0) && near(t, 0.0) && near(r, 80.0) && near(b, 40.0));
        let circle = outline(
            opts(ShapeKind::Ellipse),
            (0.0, 0.0),
            (30.0, 10.0),
            true,
            false,
        )
        .unwrap();
        assert_eq!(bounds(&circle), (0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn a_polygon_centres_on_the_press_and_turns_to_the_pointer() {
        let mut o = opts(ShapeKind::Polygon);
        o.sides = 6;
        let s = outline(o, (50.0, 50.0), (50.0, 20.0), false, false).unwrap();
        assert_eq!(s.points.len(), 6);
        assert!(near(s.points[0].anchor.0, 50.0) && near(s.points[0].anchor.1, 20.0));
        for (x, y) in anchors(&s) {
            assert!(near((x - 50.0).hypot(y - 50.0), 30.0));
        }
        // 50° snaps to 45° under Shift.
        let a = 50f64.to_radians();
        let snapped =
            outline(o, (0.0, 0.0), (10.0 * a.cos(), 10.0 * a.sin()), true, false).unwrap();
        let first = snapped.points[0].anchor;
        assert!(near(first.0, first.1));
        o.sides = 1;
        assert_eq!(
            outline(o, (0.0, 0.0), (5.0, 0.0), false, false)
                .unwrap()
                .points
                .len(),
            3
        );
    }

    #[test]
    fn each_corner_takes_its_own_radius() {
        let mut o = opts(ShapeKind::RoundedRectangle);
        o.radii = [0.0, 10.0, 20.0, 0.0];
        let s = outline_in_box(o, (0.0, 0.0, 100.0, 60.0)).unwrap();
        // Square top-left and bottom-left corners, two anchors per round one.
        assert_eq!(s.points.len(), 6);
        assert_eq!(s.points[0].anchor, (0.0, 0.0));
        assert!(s.points.iter().any(|p| p.anchor == (90.0, 0.0)));
        assert!(s.points.iter().any(|p| p.anchor == (100.0, 40.0)));
        assert!(s.points.iter().any(|p| p.anchor == (0.0, 60.0)));
        let (l, t, r, b) = bounds(&s);
        assert!(near(l, 0.0) && near(t, 0.0) && near(r, 100.0) && near(b, 60.0));
    }

    #[test]
    fn a_boxed_polygon_stands_upright_and_fills_its_box() {
        let s = outline_in_box(opts(ShapeKind::Polygon), (10.0, 20.0, 80.0, 82.0)).unwrap();
        assert_eq!(s.points.len(), 5);
        assert!(near(s.points[0].anchor.0, 50.0) && near(s.points[0].anchor.1, 20.0));
        let (l, t, r, b) = bounds(&s);
        assert!(near(l, 10.0) && near(t, 20.0) && near(r, 90.0) && near(b, 102.0));
    }

    #[test]
    fn a_star_alternates_corners_and_indents() {
        let mut o = opts(ShapeKind::Polygon);
        o.star = Some(50.0);
        let s = outline(o, (0.0, 0.0), (0.0, -40.0), false, false).unwrap();
        assert_eq!(s.points.len(), 10);
        for (i, (x, y)) in anchors(&s).into_iter().enumerate() {
            let expected = if i % 2 == 0 { 40.0 } else { 20.0 };
            assert!(near(x.hypot(y), expected), "anchor {i}");
        }
        o.smooth_corners = true;
        let smooth = outline(o, (0.0, 0.0), (0.0, -40.0), false, false).unwrap();
        assert!(smooth.points[0].smooth && !smooth.points[1].smooth);
        o.smooth_indents = true;
        let both = outline(o, (0.0, 0.0), (0.0, -40.0), false, false).unwrap();
        assert!(both.points.iter().all(|p| p.smooth));
    }

    #[test]
    fn align_edges_snaps_straight_edges_only() {
        let rect = outline_in_box(opts(ShapeKind::Rectangle), (2.4, 3.6, 8.3, 5.6)).unwrap();
        assert_eq!(
            anchors(&align_edges(&rect)),
            vec![(2.0, 4.0), (11.0, 4.0), (11.0, 9.0), (2.0, 9.0)]
        );
        let ellipse = outline_in_box(opts(ShapeKind::Ellipse), (2.4, 3.6, 8.3, 5.6)).unwrap();
        assert_eq!(align_edges(&ellipse), ellipse);
    }

    #[test]
    fn coverage_fills_the_interior_and_antialiases_the_edge() {
        let s = outline(
            opts(ShapeKind::Rectangle),
            (2.0, 2.0),
            (6.5, 6.0),
            false,
            false,
        )
        .unwrap();
        let m = coverage(&s, 8, 8);
        assert_eq!(m[3 * 8 + 3], 255);
        assert_eq!(m[3 * 8 + 1], 0);
        assert_eq!(m[3 * 8 + 6], 128, "half the pixel is covered");
        assert_eq!(m[7 * 8 + 3], 0);
        let total: u32 = m.iter().map(|&v| u32::from(v)).sum();
        assert!((total as f64 / 255.0 - 18.0).abs() < 0.1, "area is 4.5 x 4");
        let off = outline(
            opts(ShapeKind::Ellipse),
            (-20.0, -20.0),
            (40.0, 40.0),
            false,
            false,
        )
        .unwrap();
        assert_eq!(coverage(&off, 8, 8).len(), 64, "clipped to the canvas");
    }

    /// A closed rectangle path, clockwise unless `reverse`.
    fn rect_path(path: &mut VectorPath, (l, t, r, b): (f64, f64, f64, f64), reverse: bool) {
        let mut corners = vec![(l, t), (r, t), (r, b), (l, b)];
        if reverse {
            corners.reverse();
        }
        for (x, y) in corners {
            path.append_corner(x, y);
        }
        path.close_active_subpath();
        path.finish_editing();
    }

    #[test]
    fn path_coverage_follows_the_nonzero_winding_rule() {
        let mut same = VectorPath::default();
        rect_path(&mut same, (0.0, 0.0, 8.0, 8.0), false);
        rect_path(&mut same, (2.0, 2.0, 6.0, 6.0), false);
        let m = path_coverage(&same, 8, 8);
        assert_eq!(
            m[4 * 8 + 4],
            255,
            "a same-direction inner subpath stays filled"
        );
        assert_eq!(m[8], 255);
        let even_odd = coverage(&same.subpaths[1], 8, 8);
        assert_eq!(even_odd[0], 0, "coverage is one subpath");

        let mut hole = VectorPath::default();
        rect_path(&mut hole, (0.0, 0.0, 8.0, 8.0), false);
        rect_path(&mut hole, (2.0, 2.0, 6.0, 6.0), true);
        let m = path_coverage(&hole, 8, 8);
        assert_eq!(m[4 * 8 + 4], 0, "an opposite inner subpath cuts a hole");
        assert_eq!(m[8], 255);

        let mut open = VectorPath::default();
        for (x, y) in [(0.0, 0.0), (8.0, 0.0), (8.0, 8.0)] {
            open.append_corner(x, y);
        }
        open.finish_editing();
        let m = path_coverage(&open, 8, 8);
        assert_eq!(m[8 + 6], 255, "an open subpath closes implicitly");
        assert_eq!(m[6 * 8 + 1], 0);
        assert!(path_coverage(&VectorPath::default(), 8, 8)
            .iter()
            .all(|&v| v == 0));
    }
}
