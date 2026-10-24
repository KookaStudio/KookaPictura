//! The perspective planes of one Vanishing Point session: root planes pinned
//! by four image corners, and related planes hinged on a parent's edge.
//! Each family shares its root's camera, metric unit and grid size, so grid
//! lines run on across a hinge.

use super::geometry::{
    depth_toward, distance, hinge, hinge_span, lift, signed_area, Camera, Frame, Validity, P2,
};

/// The opening Grid Size.
pub const DEFAULT_GRID: f64 = 100.0;
/// The smallest plane side, in metric units (≈ pixels).
const MIN_EXTENT: f64 = 2.0;
/// The most grid lines drawn across one plane axis; denser grids skip lines.
// ponytail: past this a grid shows every n-th line rather than a solid fill.
const MAX_GRID_LINES: f64 = 1000.0;

/// What a pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Corner(usize),
    /// An edge's middle node: edge `k` runs from corner `k` to `k + 1`.
    Edge(usize),
    Inside,
}

#[derive(Clone, Debug, PartialEq)]
enum Shape {
    /// `anchor` is corner 0's plane coordinate (3D units) and `unit` the 3D
    /// length of one metric unit, so resizing an edge leaves the grid where
    /// it is on the surface.
    Root {
        corners: [P2; 4],
        anchor: P2,
        unit: f64,
        grid: f64,
    },
    /// Hinged on `edge` of `parent`; `span` is `[f0, f1, t0, t1]` (see
    /// [`hinge`]). `last` keeps the outline to show while the family is
    /// invalid.
    Child {
        parent: usize,
        edge: usize,
        angle: f64,
        side: f64,
        span: [f64; 4],
        last: [P2; 4],
    },
}

#[derive(Clone, Debug, PartialEq)]
struct Solved {
    corners: [P2; 4],
    validity: Validity,
    frame: Option<Frame>,
    camera: Option<Camera>,
}

/// The planes over a `width × height` document. The camera's principal
/// point is the document centre.
#[derive(Clone, Debug, PartialEq)]
pub struct VpPlanes {
    width: u32,
    height: u32,
    centre: P2,
    half: f64,
    shapes: Vec<Shape>,
    solved: Vec<Solved>,
}

/// 3D area per px² of a lifted root quad, as the length of one metric unit.
fn area_unit(q: &[P2; 4], sides: (f64, f64), axes_sine: f64) -> f64 {
    (sides.0 * sides.1 * axes_sine / signed_area(q).abs()).sqrt()
}

impl VpPlanes {
    pub fn new(width: u32, height: u32) -> Self {
        let (w, h) = (width as f64, height as f64);
        VpPlanes {
            width,
            height,
            centre: (w / 2.0, h / 2.0),
            half: w.hypot(h) / 2.0,
            shapes: Vec::new(),
            solved: Vec::new(),
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    pub fn corners(&self, i: usize) -> [P2; 4] {
        self.solved[i].corners
    }

    pub fn validity(&self, i: usize) -> Validity {
        self.solved[i].validity
    }

    pub(super) fn frame(&self, i: usize) -> Option<&Frame> {
        self.solved[i].frame.as_ref()
    }

    pub(super) fn camera(&self, i: usize) -> Option<&Camera> {
        self.solved[i].camera.as_ref()
    }

    pub fn parent(&self, i: usize) -> Option<usize> {
        match self.shapes[i] {
            Shape::Child { parent, .. } => Some(parent),
            Shape::Root { .. } => None,
        }
    }

    pub fn has_children(&self, i: usize) -> bool {
        (0..self.len()).any(|j| self.parent(j) == Some(i))
    }

    fn root_of(&self, mut i: usize) -> usize {
        while let Some(p) = self.parent(i) {
            i = p;
        }
        i
    }

    fn unit(&self, i: usize) -> f64 {
        match self.shapes[self.root_of(i)] {
            Shape::Root { unit, .. } => unit,
            Shape::Child { .. } => unreachable!("a root has no parent"),
        }
    }

    pub fn grid(&self, i: usize) -> f64 {
        match self.shapes[self.root_of(i)] {
            Shape::Root { grid, .. } => grid,
            Shape::Child { .. } => unreachable!("a root has no parent"),
        }
    }

    /// Set the Grid Size of `i`'s whole family.
    pub fn set_grid(&mut self, i: usize, size: f64) {
        let root = self.root_of(i);
        if let Shape::Root { grid, .. } = &mut self.shapes[root] {
            *grid = size.max(1.0);
        }
    }

    /// A related plane's angle to its parent, in degrees.
    pub fn angle(&self, i: usize) -> Option<f64> {
        match self.shapes[i] {
            Shape::Child { angle, .. } => Some(angle),
            Shape::Root { .. } => None,
        }
    }

    /// Only a related plane with no planes of its own may turn.
    pub fn angle_editable(&self, i: usize) -> bool {
        self.parent(i).is_some() && !self.has_children(i)
    }

    pub fn set_angle(&mut self, i: usize, degrees: f64) -> bool {
        if !self.angle_editable(i) || !degrees.is_finite() {
            return false;
        }
        if let Shape::Child { angle, .. } = &mut self.shapes[i] {
            *angle = degrees.rem_euclid(360.0);
        }
        self.solve();
        true
    }

    /// How a root plane on `q` would read.
    pub fn quad_validity(&self, q: &[P2; 4]) -> Validity {
        match lift(q, self.centre, self.half) {
            Ok(_) => Validity::Valid,
            Err(v) => v,
        }
    }

    /// Add a root plane on `q` (invalid quads too, as CS6 keeps them).
    pub fn add(&mut self, q: [P2; 4]) -> usize {
        let unit = match lift(&q, self.centre, self.half) {
            Ok(l) => area_unit(&q, l.sides, l.eu.cross(l.ev).norm()),
            Err(_) => 1.0,
        };
        self.shapes.push(Shape::Root {
            corners: q,
            anchor: (0.0, 0.0),
            unit,
            grid: DEFAULT_GRID,
        });
        self.solve();
        self.len() - 1
    }

    /// Remove `i` and every plane hinged on it, directly or not.
    pub fn remove(&mut self, i: usize) {
        let mut gone = vec![false; self.len()];
        gone[i] = true;
        for j in i + 1..self.len() {
            if let Some(p) = self.parent(j) {
                gone[j] = gone[p];
            }
        }
        let mut index = vec![usize::MAX; self.len()];
        let mut next = 0;
        for (j, g) in gone.iter().enumerate() {
            if !g {
                index[j] = next;
                next += 1;
            }
        }
        let shapes = std::mem::take(&mut self.shapes);
        self.shapes = shapes
            .into_iter()
            .enumerate()
            .filter(|(j, _)| !gone[*j])
            .map(|(_, mut s)| {
                if let Shape::Child { parent, .. } = &mut s {
                    *parent = index[*parent];
                }
                s
            })
            .collect();
        self.solve();
    }

    fn solve(&mut self) {
        self.solved.clear();
        for i in 0..self.shapes.len() {
            let solved = match &self.shapes[i] {
                Shape::Root {
                    corners, anchor, ..
                } => match lift(corners, self.centre, self.half) {
                    Ok(l) => {
                        let origin = l
                            .corner0
                            .sub(l.eu.scale(anchor.0))
                            .sub(l.ev.scale(anchor.1));
                        Solved {
                            corners: *corners,
                            validity: Validity::Valid,
                            frame: Some(Frame {
                                origin,
                                eu: l.eu,
                                ev: l.ev,
                                n: l.n,
                                rect: [
                                    anchor.0,
                                    anchor.0 + l.sides.0,
                                    anchor.1,
                                    anchor.1 + l.sides.1,
                                ],
                            }),
                            camera: Some(l.camera),
                        }
                    }
                    Err(validity) => Solved {
                        corners: *corners,
                        validity,
                        frame: None,
                        camera: None,
                    },
                },
                Shape::Child {
                    parent,
                    edge,
                    angle,
                    side,
                    span,
                    last,
                } => {
                    let up = &self.solved[*parent];
                    let placed = up.frame.zip(up.camera).and_then(|(frame, camera)| {
                        let child = hinge(&frame, *edge, *angle, *side, *span)?;
                        Some((child, camera, child.corners(&camera)?))
                    });
                    match placed {
                        Some((frame, camera, corners)) => Solved {
                            corners,
                            validity: Validity::Valid,
                            frame: Some(frame),
                            camera: Some(camera),
                        },
                        None => Solved {
                            corners: *last,
                            validity: match up.validity {
                                Validity::Valid => Validity::Aspect,
                                v => v,
                            },
                            frame: None,
                            camera: None,
                        },
                    }
                }
            };
            if let Shape::Child { last, .. } = &mut self.shapes[i] {
                *last = solved.corners;
            }
            self.solved.push(solved);
        }
    }

    /// The image points of the four edges' middle nodes: the plane's metric
    /// midpoints, or the plain midpoints of an invalid outline.
    pub fn edge_nodes(&self, i: usize) -> [P2; 4] {
        let c = self.corners(i);
        let plain = std::array::from_fn(|k| {
            let (a, b) = (c[k], c[(k + 1) % 4]);
            ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
        });
        let (Some(frame), Some(camera)) = (self.frame(i), self.camera(i)) else {
            return plain;
        };
        let [u0, u1, v0, v1] = frame.rect;
        let (um, vm) = ((u0 + u1) / 2.0, (v0 + v1) / 2.0);
        let mids = [(um, v0), (u1, vm), (um, v1), (u0, vm)];
        std::array::from_fn(|k| {
            camera
                .project(frame.point(mids[k].0, mids[k].1))
                .unwrap_or(plain[k])
        })
    }

    pub fn contains(&self, i: usize, p: P2) -> bool {
        let c = self.corners(i);
        let mut inside = false;
        for k in 0..4 {
            let (a, b) = (c[k], c[(k + 1) % 4]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                inside = !inside;
            }
        }
        inside
    }

    /// The node or plane under `p` within `tol` px: `first`'s nodes win,
    /// then the newest plane's; then the plane containing `p`.
    pub fn hit(&self, p: P2, tol: f64, first: Option<usize>) -> Option<(usize, Part)> {
        let order: Vec<usize> = first
            .filter(|&a| a < self.len())
            .into_iter()
            .chain((0..self.len()).rev().filter(|&j| Some(j) != first))
            .collect();
        for &i in &order {
            let c = self.corners(i);
            if let Some(k) = (0..4).find(|&k| distance(c[k], p) <= tol) {
                return Some((i, Part::Corner(k)));
            }
            let e = self.edge_nodes(i);
            if let Some(k) = (0..4).find(|&k| distance(e[k], p) <= tol) {
                return Some((i, Part::Edge(k)));
            }
        }
        order
            .into_iter()
            .find(|&i| self.contains(i, p))
            .map(|i| (i, Part::Inside))
    }

    /// The planes containing `p`, oldest first.
    pub fn planes_at(&self, p: P2) -> Vec<usize> {
        (0..self.len()).filter(|&i| self.contains(i, p)).collect()
    }

    fn min_extent(&self, i: usize) -> f64 {
        MIN_EXTENT * self.unit(i)
    }

    /// `rect` with edge `k` (or, for a corner, both of corner `k`'s edges)
    /// moved to plane coordinate `at`, keeping the minimum extent.
    fn reshape(rect: [f64; 4], part: Part, at: P2, min: f64) -> [f64; 4] {
        let [mut u0, mut u1, mut v0, mut v1] = rect;
        let edges: &[usize] = match part {
            Part::Edge(0) => &[0],
            Part::Edge(1) => &[1],
            Part::Edge(2) => &[2],
            Part::Edge(_) => &[3],
            Part::Corner(0) => &[0, 3],
            Part::Corner(1) => &[0, 1],
            Part::Corner(2) => &[1, 2],
            Part::Corner(_) => &[2, 3],
            Part::Inside => &[],
        };
        for &e in edges {
            match e {
                0 => v0 = at.1.min(v1 - min),
                1 => u1 = at.0.max(u0 + min),
                2 => v1 = at.1.max(v0 + min),
                _ => u0 = at.0.min(u1 - min),
            }
        }
        [u0, u1, v0, v1]
    }

    /// A related plane's frame rect (`[s0, s1, t0, t1]`) as its stored span.
    fn span_of(&self, c: usize, rect: [f64; 4]) -> Option<[f64; 4]> {
        let Shape::Child { parent, edge, .. } = self.shapes[c] else {
            return None;
        };
        let (lo, hi) = hinge_span(self.frame(parent)?, edge);
        let w = hi - lo;
        (w.abs() > 1e-12).then(|| [(rect[0] - lo) / w, (rect[1] - lo) / w, rect[2], rect[3]])
    }

    /// Resize `i` in its own plane so `part` follows image point `p`.
    fn resize(&mut self, i: usize, part: Part, p: P2) {
        let (Some(frame), Some(camera)) = (self.frame(i).copied(), self.camera(i).copied()) else {
            return;
        };
        let Some(at) = frame.coords_at(&camera, p) else {
            return;
        };
        let rect = Self::reshape(frame.rect, part, at, self.min_extent(i));
        let span = self.span_of(i, rect);
        match &mut self.shapes[i] {
            Shape::Root {
                corners, anchor, ..
            } => {
                let moved = Frame { rect, ..frame };
                if let Some(c) = moved.corners(&camera) {
                    *corners = c;
                    *anchor = (rect[0], rect[2]);
                }
            }
            Shape::Child { span: s, .. } => {
                if let Some(span) = span {
                    *s = span;
                }
            }
        }
        self.solve();
    }

    /// Drag corner `k` of `i` to `p`. A root corner moves freely and re-fits
    /// the family's metric; a related plane's corner resizes it in its plane.
    pub fn move_corner(&mut self, i: usize, k: usize, p: P2) {
        if self.parent(i).is_some() {
            self.resize(i, Part::Corner(k), p);
            return;
        }
        let (centre, half) = (self.centre, self.half);
        if let Shape::Root {
            corners,
            anchor,
            unit,
            ..
        } = &mut self.shapes[i]
        {
            corners[k] = p;
            if let Ok(l) = lift(corners, centre, half) {
                *unit = area_unit(corners, l.sides, l.eu.cross(l.ev).norm());
                *anchor = (0.0, 0.0);
            }
        }
        self.solve();
    }

    /// Drag edge `k`'s node of `i` to `p`, scaling the plane in perspective.
    /// An invalid root's edge simply follows the pointer.
    pub fn move_edge(&mut self, i: usize, k: usize, p: P2) {
        if self.frame(i).is_some() {
            self.resize(i, Part::Edge(k), p);
            return;
        }
        let node = self.edge_nodes(i)[k];
        if let Shape::Root { corners, .. } = &mut self.shapes[i] {
            for c in [k, (k + 1) % 4] {
                corners[c] = (corners[c].0 + p.0 - node.0, corners[c].1 + p.1 - node.1);
            }
            self.solve();
        }
    }

    /// Drag the inside of `i` from `from` to `to`: a root moves in the
    /// image, a related plane slides along its hinge.
    pub fn slide(&mut self, i: usize, from: P2, to: P2) {
        if self.parent(i).is_none() {
            if let Shape::Root { corners, .. } = &mut self.shapes[i] {
                for c in corners.iter_mut() {
                    *c = (c.0 + to.0 - from.0, c.1 + to.1 - from.1);
                }
            }
            self.solve();
            return;
        }
        let (Some(frame), Some(camera)) = (self.frame(i).copied(), self.camera(i).copied()) else {
            return;
        };
        let (Some(a), Some(b)) = (frame.coords_at(&camera, from), frame.coords_at(&camera, to))
        else {
            return;
        };
        let mut rect = frame.rect;
        rect[0] += b.0 - a.0;
        rect[1] += b.0 - a.0;
        if let (Some(moved), Shape::Child { span, .. }) =
            (self.span_of(i, rect), &mut self.shapes[i])
        {
            *span = moved;
        }
        self.solve();
    }

    /// Tear a new plane off edge `edge` of the valid plane `i`, at 90° and
    /// spanning the edge; `None` while `i` is invalid.
    pub fn tear_off(&mut self, i: usize, edge: usize) -> Option<usize> {
        self.frame(i)?;
        let min = self.min_extent(i);
        self.shapes.push(Shape::Child {
            parent: i,
            edge: edge % 4,
            angle: 90.0,
            side: 1.0,
            span: [0.0, 1.0, 0.0, min],
            last: self.corners(i),
        });
        self.solve();
        let child = self.len() - 1;
        if self.frame(child).is_none() {
            self.shapes.pop();
            self.solve();
            return None;
        }
        Some(child)
    }

    /// While tearing off, put related plane `c`'s far edge under `p`, on
    /// whichever side of its parent `p` lies.
    pub fn extend(&mut self, c: usize, p: P2) {
        let Some(up) = self.parent(c) else {
            return;
        };
        let (Some(pf), Some(camera)) = (self.frame(up).copied(), self.camera(up).copied()) else {
            return;
        };
        let min = self.min_extent(c);
        let Shape::Child {
            edge, angle, span, ..
        } = self.shapes[c].clone()
        else {
            return;
        };
        let best = [1.0, -1.0]
            .into_iter()
            .filter_map(|side| {
                let f = hinge(&pf, edge, angle, side, span)?;
                let [s0, s1, t0, _] = f.rect;
                let mid = (s0 + s1) / 2.0;
                let t = depth_toward(&camera, f.point(mid, t0), f.ev, p).max(t0 + min);
                let shown = camera.project(f.point(mid, t))?;
                Some((distance(shown, p), side, t))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let (Some((_, s, t)), Shape::Child { side, span, .. }) = (best, &mut self.shapes[c]) {
            *side = s;
            span[3] = t;
        }
        self.solve();
    }

    /// Turn related plane `c` about its hinge so its far edge's node lies
    /// nearest `p` (CS6's Alt-drag).
    pub fn aim(&mut self, c: usize, p: P2) {
        if !self.angle_editable(c) {
            return;
        }
        let up = self.parent(c).expect("editable planes are related");
        let (Some(pf), Some(camera)) = (self.frame(up).copied(), self.camera(up).copied()) else {
            return;
        };
        let Shape::Child {
            edge, side, span, ..
        } = self.shapes[c].clone()
        else {
            return;
        };
        let miss = |deg: f64| {
            hinge(&pf, edge, deg, side, span)
                .and_then(|f| {
                    let [s0, s1, _, t1] = f.rect;
                    camera.project(f.point((s0 + s1) / 2.0, t1))
                })
                .map_or(f64::INFINITY, |q| distance(q, p))
        };
        let coarse = (0..360)
            .map(f64::from)
            .min_by(|a, b| miss(*a).total_cmp(&miss(*b)))
            .unwrap_or(90.0);
        let fine = (-20..=20)
            .map(|k| coarse + f64::from(k) * 0.05)
            .min_by(|a, b| miss(*a).total_cmp(&miss(*b)))
            .unwrap_or(coarse);
        if miss(fine).is_finite() {
            self.set_angle(c, fine);
        }
    }

    /// The grid lines inside plane `i` (its outline excluded), or none
    /// while it is invalid. Lines fall on multiples of Grid Size from the
    /// family's origin, so they continue across hinges.
    pub fn grid_lines(&self, i: usize) -> Vec<[P2; 2]> {
        let (Some(frame), Some(camera)) = (self.frame(i), self.camera(i)) else {
            return Vec::new();
        };
        let unit = self.unit(i);
        let grid = self.grid(i);
        let [u0, u1, v0, v1] = frame.rect.map(|v| v / unit);
        let mut lines = Vec::new();
        let mut axis = |lo: f64, hi: f64, segment: &dyn Fn(f64) -> Option<[P2; 2]>| {
            let step = grid * ((hi - lo) / grid / MAX_GRID_LINES).ceil().max(1.0);
            let mut k = (lo / step + 1e-6).ceil();
            while k * step < hi - 1e-6 {
                if let Some(s) = segment(k * step) {
                    lines.push(s);
                }
                k += 1.0;
            }
        };
        let at = |u: f64, v: f64| camera.project(frame.point(u * unit, v * unit));
        axis(u0, u1, &|u| Some([at(u, v0)?, at(u, v1)?]));
        axis(v0, v1, &|v| Some([at(u0, v)?, at(u1, v)?]));
        lines
    }

    /// Every valid plane's outline and grid, offset by `(dx, dy)`: what
    /// Render Grids draws.
    pub fn rendered_grid(&self, dx: f64, dy: f64) -> Vec<[P2; 2]> {
        let shift = |p: P2| (p.0 + dx, p.1 + dy);
        let mut lines = Vec::new();
        for i in 0..self.len() {
            if self.validity(i) != Validity::Valid {
                continue;
            }
            let c = self.corners(i);
            lines.extend((0..4).map(|k| [shift(c[k]), shift(c[(k + 1) % 4])]));
            lines.extend(
                self.grid_lines(i)
                    .into_iter()
                    .map(|[a, b]| [shift(a), shift(b)]),
            );
        }
        lines
    }
}
