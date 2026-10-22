//! The document's Work Path: the vector outline the Pen tool group draws and
//! edits (`docs/03-tools/pen-and-path-tools.md`).
//!
//! A path is a list of subpaths of **anchor points**. Each anchor may carry two
//! direction handles, stored as *absolute* positions, so every edit reads as
//! "set this position". A **smooth** point keeps its handles collinear through
//! the anchor; a corner has none, or two independent ones.
//!
//! A segment with no handle on either end is a straight line: a cubic whose
//! control points sit on its endpoints already draws one, so
//! [`Subpath::segment`] defaults a missing handle to its anchor and nothing
//! downstream special-cases straight segments.
//!
//! Coordinates are document pixels in `f64`, for PSB-sized canvases.
//!
//! Ported from photorust's `core/src/path.rs`.

/// Which of a point's two handles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HandleSide {
    In,
    Out,
}

/// One anchor point on a subpath.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    pub anchor: (f64, f64),
    /// The handle curving the segment arriving at this point.
    pub in_handle: Option<(f64, f64)>,
    /// The handle curving the segment leaving this point.
    pub out_handle: Option<(f64, f64)>,
    /// Whether dragging one handle swings the other with it. A point can hold
    /// handles and not be smooth: an Alt-dragged handle leaves one behind.
    pub smooth: bool,
}

impl PathPoint {
    fn corner(anchor: (f64, f64)) -> Self {
        Self {
            anchor,
            in_handle: None,
            out_handle: None,
            smooth: false,
        }
    }
}

/// One contiguous run of anchors, open or closed back to its first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Subpath {
    pub points: Vec<PathPoint>,
    pub closed: bool,
}

impl Subpath {
    /// One fewer than the points when open; one per point when closed.
    pub fn segment_count(&self) -> usize {
        match self.points.len() {
            0 | 1 => 0,
            n if self.closed => n,
            n => n - 1,
        }
    }

    /// Segment `i`'s four Bezier control points, a missing handle defaulting
    /// to its own anchor.
    pub fn segment(&self, i: usize) -> Option<[(f64, f64); 4]> {
        if i >= self.segment_count() {
            return None;
        }
        let a = &self.points[i];
        let b = &self.points[(i + 1) % self.points.len()];
        Some([
            a.anchor,
            a.out_handle.unwrap_or(a.anchor),
            b.in_handle.unwrap_or(b.anchor),
            b.anchor,
        ])
    }

    /// The subpath as a polyline within `tolerance` of the true curve.
    pub fn flatten(&self, tolerance: f64) -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = self.points.first().map(|p| p.anchor).into_iter().collect();
        for seg in 0..self.segment_count() {
            if let Some(quad) = self.segment(seg) {
                flatten_cubic(quad, tolerance, FLATTEN_MAX_DEPTH, &mut out);
            }
        }
        out
    }

    /// Move every anchor and handle by `(dx, dy)`.
    pub fn translate(&mut self, dx: f64, dy: f64) {
        for point in &mut self.points {
            translate_point(point, dx, dy);
        }
    }

    /// Apply the affine `[xx, xy, yx, yy, tx, ty]` to every anchor and handle.
    pub fn transform(&mut self, a: [f64; 6]) {
        for point in &mut self.points {
            for p in [
                Some(&mut point.anchor),
                point.in_handle.as_mut(),
                point.out_handle.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                let (x, y) = *p;
                *p = (a[0] * x + a[2] * y + a[4], a[1] * x + a[3] * y + a[5]);
            }
        }
    }

    /// The polyline with its cumulative arc lengths, for type-on-path layout
    /// and picking.
    pub fn flattened(&self, tolerance: f64) -> FlattenedSubpath {
        let points = self.flatten(tolerance);
        let mut cumulative = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, &point) in points.iter().enumerate() {
            if i > 0 {
                total += (point.0 - points[i - 1].0).hypot(point.1 - points[i - 1].1);
            }
            cumulative.push(total);
        }
        FlattenedSubpath {
            points,
            cumulative,
            closed: self.closed,
            total,
        }
    }
}

/// A subpath flattened to a polyline with cumulative arc lengths.
#[derive(Clone, Debug, PartialEq)]
pub struct FlattenedSubpath {
    pub points: Vec<(f64, f64)>,
    /// Arc length at each point; the last value is the total length.
    pub cumulative: Vec<f64>,
    pub closed: bool,
    pub total: f64,
}

impl FlattenedSubpath {
    /// The point and unit tangent at `arc`: clamped on an open subpath, wrapped
    /// once on a closed one. `None` when the subpath has no length.
    pub fn point_at(&self, arc: f64) -> Option<((f64, f64), (f64, f64))> {
        if self.points.len() < 2 || self.total <= 0.0 {
            return None;
        }
        let arc = if self.closed {
            arc.rem_euclid(self.total)
        } else {
            arc.clamp(0.0, self.total)
        };
        let mut index = match self
            .cumulative
            .binary_search_by(|value| value.partial_cmp(&arc).unwrap_or(std::cmp::Ordering::Equal))
        {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        if index + 1 >= self.points.len() {
            index = self.points.len() - 2;
        }
        let start = self.cumulative[index];
        let length = self.cumulative[index + 1] - start;
        let t = if length > 0.0 {
            ((arc - start) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let a = self.points[index];
        let b = self.points[index + 1];
        let position = lerp(a, b, t);
        let tangent = if length > 0.0 {
            ((b.0 - a.0) / length, (b.1 - a.1) / length)
        } else {
            (1.0, 0.0)
        };
        Some((position, tangent))
    }

    /// The nearest arc length to `(x, y)` and its distance. `None` without a
    /// segment.
    pub fn nearest(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if self.points.len() < 2 {
            return None;
        }
        let mut best: Option<(f64, f64)> = None;
        for i in 0..self.points.len() - 1 {
            let a = self.points[i];
            let b = self.points[i + 1];
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length2 = dx * dx + dy * dy;
            let length = length2.sqrt();
            let t = if length2 > 0.0 {
                (((x - a.0) * dx + (y - a.1) * dy) / length2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance = (x - (a.0 + dx * t)).hypot(y - (a.1 + dy * t));
            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((self.cumulative[i] + length * t, distance));
            }
        }
        best
    }

    /// Even-odd containment inside a closed subpath.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        self.closed && point_in_polygon(&self.points, (x, y))
    }
}

/// A path of one or more subpaths. `editing` is the subpath the Pen extends
/// next; `None` between drawing sessions, so the next anchor starts a new one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VectorPath {
    pub subpaths: Vec<Subpath>,
    editing: Option<usize>,
}

/// A saved path in the Paths panel: a [`VectorPath`] kept under a name, as
/// opposed to the one temporary Work Path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NamedPath {
    pub name: String,
    pub path: VectorPath,
}

/// Recursion cap when flattening a curve.
const FLATTEN_MAX_DEPTH: u32 = 10;

/// Below this drag distance (document pixels) a Pen press stays a corner.
const HANDLE_THRESHOLD: f64 = 1.0;

/// Flattening tolerance for picking and bounds, in document pixels.
const BOUNDS_TOLERANCE: f64 = 0.25;

impl VectorPath {
    pub fn is_empty(&self) -> bool {
        self.subpaths.iter().all(|s| s.points.is_empty())
    }

    // -- drawing with the Pen ------------------------------------------------

    /// Append a corner anchor, starting a new subpath if none is being edited.
    /// Returns the new point's `(subpath, point)`.
    pub fn append_corner(&mut self, x: f64, y: f64) -> (usize, usize) {
        let sp = match self.editing {
            Some(i) => i,
            None => {
                self.subpaths.push(Subpath::default());
                self.subpaths.len() - 1
            }
        };
        self.editing = Some(sp);
        self.subpaths[sp].points.push(PathPoint::corner((x, y)));
        (sp, self.subpaths[sp].points.len() - 1)
    }

    /// The anchor the Pen last placed on the subpath being edited.
    pub fn last_anchor(&self) -> Option<(f64, f64)> {
        let sp = self.editing?;
        self.subpaths[sp].points.last().map(|p| p.anchor)
    }

    /// The Pen's drag after placing an anchor: pull out a symmetric pair of
    /// handles, making the point smooth. Under a pixel of drag it stays a
    /// plain corner. `independent` (Alt) sets only the outgoing handle, for a
    /// straight-in, curved-out corner.
    pub fn update_last_handle(&mut self, x: f64, y: f64, independent: bool) -> bool {
        let Some(sp) = self.editing else {
            return false;
        };
        let Some(pt) = self.subpaths[sp].points.last_mut() else {
            return false;
        };
        let (ax, ay) = pt.anchor;
        if (x - ax).hypot(y - ay) < HANDLE_THRESHOLD {
            *pt = PathPoint::corner(pt.anchor);
            return true;
        }
        pt.out_handle = Some((x, y));
        pt.smooth = !independent;
        pt.in_handle = (!independent).then_some((2.0 * ax - x, 2.0 * ay - y));
        true
    }

    /// Close the subpath being edited back to its first anchor. Refused with
    /// fewer than two points.
    pub fn close_active_subpath(&mut self) -> bool {
        let Some(sp) = self.editing else {
            return false;
        };
        if self.subpaths[sp].points.len() < 2 {
            return false;
        }
        self.subpaths[sp].closed = true;
        self.editing = None;
        true
    }

    /// Append a finished subpath (a shape tool's outline), ending any drawing
    /// session. Returns its index.
    pub fn add_subpath(&mut self, subpath: Subpath) -> usize {
        self.editing = None;
        self.subpaths.push(subpath);
        self.subpaths.len() - 1
    }

    /// Stop extending the current subpath, leaving it open.
    pub fn finish_editing(&mut self) {
        self.editing = None;
    }

    pub fn editing_subpath(&self) -> Option<usize> {
        self.editing
    }

    /// Resume drawing from an open subpath's endpoint (CS6's "extend"). A
    /// press on the first point reverses the subpath so new anchors still
    /// append at the end. Refused for a closed subpath or an interior point.
    pub fn resume_at(&mut self, sp: usize, pt: usize) -> bool {
        let Some(subpath) = self.subpaths.get_mut(sp) else {
            return false;
        };
        let last = subpath.points.len().saturating_sub(1);
        if subpath.closed || subpath.points.is_empty() || (pt != 0 && pt != last) {
            return false;
        }
        if pt == 0 && last > 0 {
            subpath.points.reverse();
            for p in &mut subpath.points {
                std::mem::swap(&mut p.in_handle, &mut p.out_handle);
            }
        }
        self.editing = Some(sp);
        true
    }

    /// A Freeform Pen drag, reduced to corner anchors within `tolerance` and
    /// appended as a new subpath. False (nothing added) for fewer than two
    /// surviving points.
    pub fn add_freeform(&mut self, points: &[(f64, f64)], tolerance: f64, close: bool) -> bool {
        let simplified = simplify_freehand(points, tolerance);
        if simplified.len() < 2 {
            return false;
        }
        self.editing = None;
        for &(x, y) in &simplified {
            self.append_corner(x, y);
        }
        if !(close && simplified.len() > 2 && self.close_active_subpath()) {
            self.finish_editing();
        }
        true
    }

    // -- Convert Point ---------------------------------------------------------

    /// Drag one handle. On a smooth point the opposite handle keeps its length
    /// and swings to stay collinear, unless `independent`, which moves just
    /// the one and permanently breaks the point's smoothness.
    pub fn move_handle(
        &mut self,
        sp: usize,
        pt: usize,
        side: HandleSide,
        x: f64,
        y: f64,
        independent: bool,
    ) -> bool {
        let Some(point) = self.point_mut(sp, pt) else {
            return false;
        };
        let anchor = point.anchor;
        match side {
            HandleSide::In => point.in_handle = Some((x, y)),
            HandleSide::Out => point.out_handle = Some((x, y)),
        }
        if independent {
            point.smooth = false;
            return true;
        }
        if !point.smooth {
            return true;
        }
        let (dx, dy) = (x - anchor.0, y - anchor.1);
        let dist = dx.hypot(dy).max(1e-9);
        let other = match side {
            HandleSide::In => &mut point.out_handle,
            HandleSide::Out => &mut point.in_handle,
        };
        let length = other.map_or(dist, |(ox, oy)| (ox - anchor.0).hypot(oy - anchor.1));
        *other = Some((anchor.0 - dx / dist * length, anchor.1 - dy / dist * length));
        true
    }

    /// Strip both handles, leaving a plain corner. False when the point was
    /// already one (nothing changed) or does not exist.
    pub fn set_corner(&mut self, sp: usize, pt: usize) -> bool {
        let Some(point) = self.point_mut(sp, pt) else {
            return false;
        };
        let corner = PathPoint::corner(point.anchor);
        if *point == corner {
            return false;
        }
        *point = corner;
        true
    }

    /// Pull a fresh symmetric pair of handles out of an anchor, turning it
    /// smooth.
    pub fn drag_new_handles(&mut self, sp: usize, pt: usize, x: f64, y: f64) -> bool {
        let Some(point) = self.point_mut(sp, pt) else {
            return false;
        };
        let (ax, ay) = point.anchor;
        point.out_handle = Some((x, y));
        point.in_handle = Some((2.0 * ax - x, 2.0 * ay - y));
        point.smooth = true;
        true
    }

    // -- Path Selection / Direct Selection --------------------------------------

    /// Move an anchor to `(x, y)`, carrying its handles by the same delta so
    /// the curve either side keeps its shape.
    pub fn move_anchor(&mut self, sp: usize, pt: usize, x: f64, y: f64) -> bool {
        let Some(point) = self.point_mut(sp, pt) else {
            return false;
        };
        let (dx, dy) = (x - point.anchor.0, y - point.anchor.1);
        translate_point(point, dx, dy);
        true
    }

    /// Move a whole subpath (a path component) by `(dx, dy)`.
    pub fn move_subpath(&mut self, sp: usize, dx: f64, dy: f64) -> bool {
        let Some(subpath) = self.subpaths.get_mut(sp) else {
            return false;
        };
        for point in &mut subpath.points {
            translate_point(point, dx, dy);
        }
        true
    }

    /// Append a copy of subpath `sp`, returning the copy's index.
    pub fn duplicate_subpath(&mut self, sp: usize) -> Option<usize> {
        let copy = self.subpaths.get(sp)?.clone();
        self.subpaths.push(copy);
        Some(self.subpaths.len() - 1)
    }

    /// Remove a whole subpath, renumbering `editing` if a later subpath
    /// shifts down.
    pub fn remove_subpath(&mut self, sp: usize) -> bool {
        if sp >= self.subpaths.len() {
            return false;
        }
        self.subpaths.remove(sp);
        self.editing = match self.editing {
            Some(e) if e == sp => None,
            Some(e) if e > sp => Some(e - 1),
            other => other,
        };
        true
    }

    /// The bounds of subpath `sp`'s curve as `(left, top, right, bottom)`;
    /// `None` when it has no points.
    pub fn subpath_bounds(&self, sp: usize) -> Option<(f64, f64, f64, f64)> {
        let points = self.subpaths.get(sp)?.flatten(BOUNDS_TOLERANCE);
        let &(x0, y0) = points.first()?;
        Some(
            points
                .iter()
                .fold((x0, y0, x0, y0), |(l, t, r, b), &(x, y)| {
                    (l.min(x), t.min(y), r.max(x), b.max(y))
                }),
        )
    }

    // -- Add / Delete Anchor Point ---------------------------------------------

    /// Split segment `seg` of subpath `sp` at `t` with de Casteljau, so the
    /// visible shape is unchanged. A straight segment splits into two straight
    /// ones rather than gaining a curve.
    pub fn insert_anchor(&mut self, sp: usize, seg: usize, t: f64) -> bool {
        let Some(subpath) = self.subpaths.get_mut(sp) else {
            return false;
        };
        let Some([p0, p1, p2, p3]) = subpath.segment(seg) else {
            return false;
        };
        let t = t.clamp(0.0, 1.0);
        let next = (seg + 1) % subpath.points.len();
        let curved =
            subpath.points[seg].out_handle.is_some() || subpath.points[next].in_handle.is_some();

        let p01 = lerp(p0, p1, t);
        let p12 = lerp(p1, p2, t);
        let p23 = lerp(p2, p3, t);
        let p012 = lerp(p01, p12, t);
        let p123 = lerp(p12, p23, t);
        let new_point = PathPoint {
            anchor: lerp(p012, p123, t),
            in_handle: curved.then_some(p012),
            out_handle: curved.then_some(p123),
            smooth: curved,
        };

        if curved {
            subpath.points[seg].out_handle = Some(p01);
            subpath.points[next].in_handle = Some(p23);
        }
        // The closing segment of a closed subpath wraps to index 0; its new
        // point belongs at the end.
        if next == 0 {
            subpath.points.push(new_point);
        } else {
            subpath.points.insert(next, new_point);
        }
        true
    }

    /// Remove an anchor. Emptying a subpath removes it, renumbering `editing`
    /// if a later subpath shifts down.
    pub fn delete_anchor(&mut self, sp: usize, pt: usize) -> bool {
        let Some(subpath) = self.subpaths.get_mut(sp) else {
            return false;
        };
        if pt >= subpath.points.len() {
            return false;
        }
        subpath.points.remove(pt);
        if subpath.points.len() < 2 {
            subpath.closed = false;
        }
        if subpath.points.is_empty() {
            self.remove_subpath(sp);
        }
        true
    }

    fn point_mut(&mut self, sp: usize, pt: usize) -> Option<&mut PathPoint> {
        self.subpaths.get_mut(sp)?.points.get_mut(pt)
    }

    // -- hit testing -----------------------------------------------------------

    /// The nearest anchor within `radius`, as `(subpath, point)`.
    pub fn hit_anchor(&self, x: f64, y: f64, radius: f64) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, f64)> = None;
        for (si, sp) in self.subpaths.iter().enumerate() {
            for (pi, pt) in sp.points.iter().enumerate() {
                let d = (pt.anchor.0 - x).hypot(pt.anchor.1 - y);
                if d <= radius && best.is_none_or(|(_, _, bd)| d < bd) {
                    best = Some((si, pi, d));
                }
            }
        }
        best.map(|(s, p, _)| (s, p))
    }

    /// The nearest existing handle within `radius`.
    pub fn hit_handle(&self, x: f64, y: f64, radius: f64) -> Option<(usize, usize, HandleSide)> {
        let mut best: Option<(usize, usize, HandleSide, f64)> = None;
        for (si, sp) in self.subpaths.iter().enumerate() {
            for (pi, pt) in sp.points.iter().enumerate() {
                for (side, h) in [
                    (HandleSide::In, pt.in_handle),
                    (HandleSide::Out, pt.out_handle),
                ] {
                    let Some(h) = h else { continue };
                    let d = (h.0 - x).hypot(h.1 - y);
                    if d <= radius && best.is_none_or(|(_, _, _, bd)| d < bd) {
                        best = Some((si, pi, side, d));
                    }
                }
            }
        }
        best.map(|(s, p, side, _)| (s, p, side))
    }

    /// The nearest point on any segment within `radius`, as
    /// `(subpath, segment, t)`.
    pub fn hit_segment(&self, x: f64, y: f64, radius: f64) -> Option<(usize, usize, f64)> {
        let mut best: Option<(usize, usize, f64, f64)> = None;
        for (si, sp) in self.subpaths.iter().enumerate() {
            for seg in 0..sp.segment_count() {
                let Some(quad) = sp.segment(seg) else {
                    continue;
                };
                let (t, d) = nearest_on_cubic(quad, (x, y));
                if d <= radius && best.is_none_or(|(_, _, _, bd)| d < bd) {
                    best = Some((si, seg, t, d));
                }
            }
        }
        best.map(|(s, seg, t, _)| (s, seg, t))
    }

    /// The subpath under `(x, y)`, Path Selection's pick: within `radius` of a
    /// segment or a lone anchor, else inside a closed one (even-odd, the
    /// topmost first), so a line drawn inside a closed shape stays reachable.
    pub fn hit_subpath(&self, x: f64, y: f64, radius: f64) -> Option<usize> {
        self.hit_segment(x, y, radius)
            .map(|(s, _, _)| s)
            .or_else(|| self.hit_anchor(x, y, radius).map(|(s, _)| s))
            .or_else(|| {
                self.subpaths.iter().rposition(|sp| {
                    sp.closed && point_in_polygon(&sp.flatten(BOUNDS_TOLERANCE), (x, y))
                })
            })
    }

    // -- flattening ------------------------------------------------------------

    /// Every subpath as a polyline within `tolerance` of the true curve, with
    /// its closed flag.
    pub fn flatten(&self, tolerance: f64) -> Vec<(Vec<(f64, f64)>, bool)> {
        self.subpaths
            .iter()
            .map(|sp| (sp.flatten(tolerance), sp.closed))
            .collect()
    }
}

fn translate_point(point: &mut PathPoint, dx: f64, dy: f64) {
    for p in [
        Some(&mut point.anchor),
        point.in_handle.as_mut(),
        point.out_handle.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        p.0 += dx;
        p.1 += dy;
    }
}

/// Even-odd point-in-polygon over a flattened, implicitly closed contour.
fn point_in_polygon(points: &[(f64, f64)], (px, py): (f64, f64)) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for (i, &(xi, yi)) in points.iter().enumerate() {
        let (xj, yj) = points[j];
        if (yi > py) != (yj > py) && px < xi + (py - yi) / (yj - yi) * (xj - xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn cubic_at([p0, p1, p2, p3]: [(f64, f64); 4], t: f64) -> (f64, f64) {
    let p012 = lerp(lerp(p0, p1, t), lerp(p1, p2, t), t);
    let p123 = lerp(lerp(p1, p2, t), lerp(p2, p3, t), t);
    lerp(p012, p123, t)
}

/// Split until each piece is flat within `tolerance`, appending far endpoints.
fn flatten_cubic(q: [(f64, f64); 4], tolerance: f64, depth: u32, out: &mut Vec<(f64, f64)>) {
    let [p0, p1, p2, p3] = q;
    let flat = distance_to_segment(p1, p0, p3) <= tolerance
        && distance_to_segment(p2, p0, p3) <= tolerance;
    if depth == 0 || flat {
        out.push(p3);
        return;
    }
    let p01 = lerp(p0, p1, 0.5);
    let p12 = lerp(p1, p2, 0.5);
    let p23 = lerp(p2, p3, 0.5);
    let p012 = lerp(p01, p12, 0.5);
    let p123 = lerp(p12, p23, 0.5);
    let mid = lerp(p012, p123, 0.5);
    flatten_cubic([p0, p01, p012, mid], tolerance, depth - 1, out);
    flatten_cubic([mid, p123, p23, p3], tolerance, depth - 1, out);
}

fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    if len2 <= 1e-12 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0);
    (p.0 - (a.0 + abx * t)).hypot(p.1 - (a.1 + aby * t))
}

/// Closest point on a cubic to `p` as `(t, distance)`: coarse sampling, then
/// a ternary refinement around the best sample.
fn nearest_on_cubic(q: [(f64, f64); 4], p: (f64, f64)) -> (f64, f64) {
    const SAMPLES: usize = 24;
    let dist = |t: f64| {
        let c = cubic_at(q, t);
        (c.0 - p.0).hypot(c.1 - p.1)
    };
    let mut best = (0.0, f64::MAX);
    for i in 0..=SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        let d = dist(t);
        if d < best.1 {
            best = (t, d);
        }
    }
    let step = 1.0 / SAMPLES as f64;
    let (mut lo, mut hi) = ((best.0 - step).max(0.0), (best.0 + step).min(1.0));
    for _ in 0..30 {
        let left = lo + (hi - lo) / 3.0;
        let right = hi - (hi - lo) / 3.0;
        if dist(left) < dist(right) {
            hi = right;
        } else {
            lo = left;
        }
    }
    let t = (lo + hi) * 0.5;
    (t, dist(t))
}

/// Reduce a freehand drag to the points Douglas-Peucker keeps within
/// `tolerance`, ends included.
///
/// ponytail: every kept point becomes a corner anchor; CS6's Freeform Pen fits
/// curves, so a freehand circle comes out as a polygon here.
pub fn simplify_freehand(points: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    douglas_peucker(points, 0, points.len() - 1, tolerance, &mut keep);
    points
        .iter()
        .zip(&keep)
        .filter_map(|(&p, &k)| k.then_some(p))
        .collect()
}

fn douglas_peucker(
    points: &[(f64, f64)],
    start: usize,
    end: usize,
    tolerance: f64,
    keep: &mut [bool],
) {
    if end <= start + 1 {
        return;
    }
    let (mut farthest, mut farthest_dist) = (start, 0.0);
    for i in (start + 1)..end {
        let d = distance_to_segment(points[i], points[start], points[end]);
        if d > farthest_dist {
            farthest = i;
            farthest_dist = d;
        }
    }
    if farthest_dist <= tolerance {
        return;
    }
    keep[farthest] = true;
    douglas_peucker(points, start, farthest, tolerance, keep);
    douglas_peucker(points, farthest, end, tolerance, keep);
}

#[cfg(test)]
mod tests;
