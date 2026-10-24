//! The camera and plane geometry behind Vanishing Point.
//!
//! A root plane's four image corners give two vanishing points. With the
//! principal point at the document centre and square pixels, a rectangle's
//! two edge directions are perpendicular only when the focal length satisfies
//! `f² = −(v₁·v₂) / (w₁w₂)` (vanishing points in homogeneous, centred
//! coordinates); `f² ≤ 0` means no camera sees the quad as a rectangle. When
//! an edge pair is (nearly) parallel its vanishing point is at infinity and
//! leaves `f` free, so a normal lens is assumed. The corners then lift to a
//! 3D rectangle (`K⁻¹·H`), scaled so the plane lies at unit distance, which
//! fixes each plane's metric and lets related planes hinge on shared edges.
//! This is an approximation of Adobe's closed solver, not verified parity.

use pictura_core::Homography;

pub type P2 = (f64, f64);

/// A vanishing point farther than this many half-diagonals from the centre
/// counts as at infinity.
const FAR_VANISHING: f64 = 20.0;
/// The focal length (in half-diagonals) assumed when a vanishing point is at
/// infinity: a normal lens, whose diagonal angle of view is about 47°.
pub(super) const NORMAL_FOCAL: f64 = 2.3;
/// Quads smaller than this (px²) are degenerate.
const MIN_AREA: f64 = 4.0;
/// Points nearer the camera plane than this are treated as behind it.
const Z_EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub(super) const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn scale(self, k: f64) -> V3 {
        v3(self.x * k, self.y * k, self.z * k)
    }
    pub fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn unit(self) -> Option<V3> {
        let n = self.norm();
        (n.is_finite() && n > 1e-12).then(|| self.scale(1.0 / n))
    }
}

/// A pinhole camera: principal point `centre` (px), `half` the image
/// diagonal (px), and a focal length `f` in half-diagonals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub centre: P2,
    pub half: f64,
    pub f: f64,
}

impl Camera {
    pub fn project(&self, p: V3) -> Option<P2> {
        if p.z.is_nan() || p.z <= Z_EPSILON {
            return None;
        }
        let k = self.half * self.f / p.z;
        let q = (self.centre.0 + p.x * k, self.centre.1 + p.y * k);
        (q.0.is_finite() && q.1.is_finite()).then_some(q)
    }

    /// The viewing ray through image point `p`, at unit depth.
    pub fn ray(&self, p: P2) -> V3 {
        let k = self.half * self.f;
        v3((p.0 - self.centre.0) / k, (p.1 - self.centre.1) / k, 1.0)
    }
}

/// How a plane's outline reads, as CS6 colours it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validity {
    /// Blue: a rectangle some camera sees.
    Valid,
    /// Red: crossed, concave or degenerate, so no aspect ratio exists.
    Aspect,
    /// Yellow: convex, but its vanishing points imply no real camera.
    Vanishing,
}

impl Validity {
    pub fn code(self) -> i32 {
        match self {
            Validity::Valid => 0,
            Validity::Aspect => 1,
            Validity::Vanishing => 2,
        }
    }
}

/// A plane in 3D: points `origin + u·eu + v·ev` for `(u, v)` in `rect`
/// (`[u0, u1, v0, v1]`), with unit axes and normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: V3,
    pub eu: V3,
    pub ev: V3,
    pub n: V3,
    pub rect: [f64; 4],
}

impl Frame {
    pub fn point(&self, u: f64, v: f64) -> V3 {
        self.origin.add(self.eu.scale(u)).add(self.ev.scale(v))
    }

    /// The rect's corners in plane order: `(u0,v0) (u1,v0) (u1,v1) (u0,v1)`.
    pub fn corner_coords(&self) -> [P2; 4] {
        let [u0, u1, v0, v1] = self.rect;
        [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
    }

    pub fn corners(&self, camera: &Camera) -> Option<[P2; 4]> {
        let c = self.corner_coords();
        Some([
            camera.project(self.point(c[0].0, c[0].1))?,
            camera.project(self.point(c[1].0, c[1].1))?,
            camera.project(self.point(c[2].0, c[2].1))?,
            camera.project(self.point(c[3].0, c[3].1))?,
        ])
    }

    /// The components of the in-plane direction `d` on the (possibly
    /// oblique) axes.
    pub fn components(&self, d: V3) -> Option<P2> {
        let (a11, a12, a22) = (
            self.eu.dot(self.eu),
            self.eu.dot(self.ev),
            self.ev.dot(self.ev),
        );
        let (b1, b2) = (self.eu.dot(d), self.ev.dot(d));
        let det = a11 * a22 - a12 * a12;
        (det.abs() > 1e-12).then(|| ((b1 * a22 - b2 * a12) / det, (a11 * b2 - a12 * b1) / det))
    }

    /// The plane coordinates image point `p` looks at, if the ray meets the
    /// plane in front of the camera.
    pub fn coords_at(&self, camera: &Camera, p: P2) -> Option<P2> {
        let r = camera.ray(p);
        let denom = self.n.dot(r);
        if denom.abs() < 1e-12 {
            return None;
        }
        let lambda = self.n.dot(self.origin) / denom;
        if lambda.is_nan() || lambda <= 0.0 {
            return None;
        }
        self.components(r.scale(lambda).sub(self.origin))
    }
}

/// Signed area (shoelace) of a quad, px².
pub fn signed_area(q: &[P2; 4]) -> f64 {
    let mut s = 0.0;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        s += a.0 * b.1 - b.0 * a.1;
    }
    s / 2.0
}

/// Whether a quad is strictly convex (every turn the same way) and not tiny.
pub fn convex(q: &[P2; 4]) -> bool {
    if q.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) || signed_area(q).abs() < MIN_AREA {
        return false;
    }
    let mut sign = 0.0;
    for i in 0..4 {
        let (a, b, c) = (q[i], q[(i + 1) % 4], q[(i + 2) % 4]);
        let turn = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if turn.abs() < 1e-9 {
            return false;
        }
        if sign == 0.0 {
            sign = turn.signum();
        } else if turn.signum() != sign {
            return false;
        }
    }
    true
}

fn homog(p: P2) -> V3 {
    v3(p.0, p.1, 1.0)
}

/// The focal length (half-diagonals) a convex quad implies, or `None` when
/// its vanishing points admit no real camera.
pub fn focal_for(q: &[P2; 4], centre: P2, half: f64) -> Option<f64> {
    let n: Vec<V3> = q
        .iter()
        .map(|p| homog(((p.0 - centre.0) / half, (p.1 - centre.1) / half)))
        .collect();
    // Edges 0→1 / 3→2 and 0→3 / 1→2 meet at the two vanishing points.
    let v1 = n[0].cross(n[1]).cross(n[3].cross(n[2]));
    let v2 = n[0].cross(n[3]).cross(n[1].cross(n[2]));
    let far = |v: V3| v.z.abs() * FAR_VANISHING <= (v.x * v.x + v.y * v.y).sqrt();
    if far(v1) || far(v2) {
        return Some(NORMAL_FOCAL);
    }
    let f2 = -(v1.x * v2.x + v1.y * v2.y) / (v1.z * v2.z);
    (f2.is_finite() && f2 > 0.0).then(|| f2.sqrt())
}

/// A root plane lifted to 3D: the camera, the corner-0-anchored axes, and
/// the side lengths `(|U|, |V|)`, scaled so the plane lies at unit distance.
pub struct Lifted {
    pub camera: Camera,
    pub corner0: V3,
    pub eu: V3,
    pub ev: V3,
    pub n: V3,
    pub sides: (f64, f64),
}

/// Lift a root quad, or say why it cannot be.
pub fn lift(q: &[P2; 4], centre: P2, half: f64) -> Result<Lifted, Validity> {
    if !convex(q) {
        return Err(Validity::Aspect);
    }
    let f = focal_for(q, centre, half).ok_or(Validity::Vanishing)?;
    let camera = Camera { centre, half, f };
    let k = half * f;
    let norm = q.map(|p| ((p.0 - centre.0) / k, (p.1 - centre.1) / k));
    let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let h = Homography::from_quads(unit, norm)
        .ok_or(Validity::Aspect)?
        .0;
    let (u, v, o) = (
        v3(h[0], h[3], h[6]),
        v3(h[1], h[4], h[7]),
        v3(h[2], h[5], h[8]),
    );
    let n = u.cross(v).unit().ok_or(Validity::Aspect)?;
    let d = n.dot(o);
    if d.abs() < 1e-12 {
        return Err(Validity::Aspect);
    }
    let lambda = if o.z / d.abs() > 0.0 {
        1.0 / d.abs()
    } else {
        -1.0 / d.abs()
    };
    let (u, v, o) = (u.scale(lambda), v.scale(lambda), o.scale(lambda));
    let behind = [o, o.add(u), o.add(u).add(v), o.add(v)]
        .iter()
        .any(|p| p.z.is_nan() || p.z <= Z_EPSILON);
    if behind {
        return Err(Validity::Aspect);
    }
    Ok(Lifted {
        camera,
        corner0: o,
        eu: u.unit().ok_or(Validity::Aspect)?,
        ev: v.unit().ok_or(Validity::Aspect)?,
        n,
        sides: (u.norm(), v.norm()),
    })
}

/// The parent coordinates edge `edge` spans: `u` for edges 0 and 2, `v`
/// for 1 and 3.
pub fn hinge_span(parent: &Frame, edge: usize) -> (f64, f64) {
    let [u0, u1, v0, v1] = parent.rect;
    if edge.is_multiple_of(2) {
        (u0, u1)
    } else {
        (v0, v1)
    }
}

/// The child frame hinged on `edge` of `parent` at `angle` degrees (0 lies
/// flat beyond the edge, 90 is perpendicular on the `side` of the parent's
/// normal). `span` is `[f0, f1, t0, t1]`: `f` the fraction of the edge
/// (so a child follows its parent's edge as it is resized), `t` the 3D
/// distance away from it. The frame's `s` axis runs along the edge in the
/// parent's own coordinate, so grid lines continue across the hinge.
pub fn hinge(parent: &Frame, edge: usize, angle: f64, side: f64, span: [f64; 4]) -> Option<Frame> {
    let (lo, hi) = hinge_span(parent, edge);
    let rect = [
        lo + span[0] * (hi - lo),
        lo + span[1] * (hi - lo),
        span[2],
        span[3],
    ];
    let (along, origin, out) = hinge_axes(parent, edge)?;
    let (s, c) = angle.to_radians().sin_cos();
    let d = out.scale(c).add(parent.n.scale(s * side)).unit()?;
    Some(Frame {
        origin,
        eu: along,
        ev: d,
        n: along.cross(d).unit()?,
        rect,
    })
}

/// Edge `edge` of `parent` as a hinge: the direction along it, the point
/// where its own coordinate is zero, and the in-plane direction pointing out
/// of the parent across it.
pub fn hinge_axes(parent: &Frame, edge: usize) -> Option<(V3, V3, V3)> {
    let [u0, u1, v0, v1] = parent.rect;
    let (along, origin) = match edge {
        0 => (parent.eu, parent.origin.add(parent.ev.scale(v0))),
        1 => (parent.ev, parent.origin.add(parent.eu.scale(u1))),
        2 => (parent.eu, parent.origin.add(parent.ev.scale(v1))),
        _ => (parent.ev, parent.origin.add(parent.eu.scale(u0))),
    };
    let centre = parent.point((u0 + u1) / 2.0, (v0 + v1) / 2.0);
    let mut out = parent.n.cross(along).unit()?;
    if out.dot(centre.sub(origin)) > 0.0 {
        out = out.scale(-1.0);
    }
    Some((along, origin, out))
}

/// The depth `t` along `dir` from `from` whose image lies nearest `p`
/// (algebraic least squares), kept in front of the camera.
pub fn depth_toward(camera: &Camera, from: V3, dir: V3, p: P2) -> f64 {
    let q = (
        (p.0 - camera.centre.0) / camera.half,
        (p.1 - camera.centre.1) / camera.half,
    );
    let f = camera.f;
    let a = (f * from.x - q.0 * from.z, f * from.y - q.1 * from.z);
    let b = (f * dir.x - q.0 * dir.z, f * dir.y - q.1 * dir.z);
    let bb = b.0 * b.0 + b.1 * b.1;
    let mut t = if bb > 1e-18 {
        -(a.0 * b.0 + a.1 * b.1) / bb
    } else {
        0.0
    };
    // Stay ahead of the camera: the far point keeps at least a twentieth of
    // the hinge's depth.
    if dir.z < 0.0 {
        t = t.min((from.z * 0.95) / -dir.z);
    }
    t
}

pub fn distance(a: P2, b: P2) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
