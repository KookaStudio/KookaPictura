//! A self-intersecting (bowtie) target quad. A homography onto a crossed quad
//! puts the horizon through the source, so its image runs off to infinity and
//! wraps round the outside of the quad. Photoshop's Perspective instead fills
//! the quad's own outline: the two straight-edged triangles that meet where its
//! edges cross. Here the source splits at a row (or column) `split`: the part
//! before it fills the first triangle and the rest the second, each row parallel
//! to its triangle's base and shrinking to the crossing point. Inferred from a
//! CS6 screen recording of a flat-colour layer (no image oracle), so the pixel
//! layout inside the triangles is not verified.

use pictura_core::PsdRect;

use super::transform::PlaneMap;

type Point = (f64, f64);
type Quad = [Point; 4];

fn orient(a: Point, b: Point, c: Point) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Whether the open segments `ab` and `cd` cross at a single interior point.
fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    orient(a, b, c) * orient(a, b, d) < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0
}

/// Whether `quad` (TL, TR, BR, BL) crosses itself: its side edges or its top
/// and bottom edges intersect.
pub(super) fn quad_is_crossed(quad: &Quad) -> bool {
    let [tl, tr, br, bl] = *quad;
    segments_cross(tl, bl, tr, br) || segments_cross(tl, tr, bl, br)
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Barycentric weights of `p` in triangle `(a, b, c)`, or `None` outside it.
fn barycentric(a: Point, b: Point, c: Point, p: Point) -> Option<[f64; 3]> {
    const SLACK: f64 = 1e-9;
    let area = orient(a, b, c);
    if !area.is_finite() || area.abs() < 1e-12 {
        return None;
    }
    let w = [
        orient(b, c, p) / area,
        orient(c, a, p) / area,
        orient(a, b, p) / area,
    ];
    w.iter().all(|w| *w >= -SLACK).then_some(w)
}

/// The two-triangle map of the unit square onto a crossed quad, in a frame
/// where the side edges `q[0]→q[3]` and `q[1]→q[2]` cross at `apex` (a quad
/// whose top and bottom edges cross is transposed into this frame).
#[derive(Clone, Copy)]
pub(crate) struct Hourglass {
    q: Quad,
    apex: Point,
    /// The source row at the apex: the mean of the apex's fractions along the
    /// two side edges (equal for a symmetric bowtie).
    split: f64,
    transposed: bool,
}

impl Hourglass {
    /// The map onto `quad`, or `None` when it does not cross itself.
    pub(crate) fn new(quad: &Quad) -> Option<Self> {
        let [tl, tr, br, bl] = *quad;
        let (q, transposed) = if segments_cross(tl, bl, tr, br) {
            (*quad, false)
        } else if segments_cross(tl, tr, bl, br) {
            ([tl, bl, br, tr], true)
        } else {
            return None;
        };
        // Solve q0 + s·(q3 − q0) = q1 + t·(q2 − q1).
        let d0 = (q[3].0 - q[0].0, q[3].1 - q[0].1);
        let d1 = (q[2].0 - q[1].0, q[2].1 - q[1].1);
        let r = (q[1].0 - q[0].0, q[1].1 - q[0].1);
        let den = d0.0 * d1.1 - d0.1 * d1.0;
        if den.abs() < 1e-12 {
            return None;
        }
        let s = (r.0 * d1.1 - r.1 * d1.0) / den;
        let t = (r.0 * d0.1 - r.1 * d0.0) / den;
        let split = ((s + t) / 2.0).clamp(1e-6, 1.0 - 1e-6);
        Some(Self {
            q,
            apex: lerp(q[0], q[3], s),
            split,
            transposed,
        })
    }

    /// The point at source fractions `(u, v)` (`u` along TL→TR, `v` along
    /// TL→BL).
    pub(crate) fn forward(&self, u: f64, v: f64) -> Point {
        let (u, v) = if self.transposed { (v, u) } else { (u, v) };
        let q = &self.q;
        if v <= self.split {
            lerp(lerp(q[0], q[1], u), self.apex, v / self.split)
        } else {
            let m = (v - self.split) / (1.0 - self.split);
            lerp(self.apex, lerp(q[3], q[2], u), m)
        }
    }

    /// The source fractions `(u, v)` landing on `p`, or `None` outside both
    /// triangles.
    pub(crate) fn inverse(&self, p: Point) -> Option<(f64, f64)> {
        let q = &self.q;
        // Upper: p = (1 − λ)·lerp(q0, q1, u) + λ·apex, so the apex weight is λ.
        // Lower: p = (1 − m)·apex + m·lerp(q3, q2, u).
        let (u, v) = if let Some([_, w1, wa]) = barycentric(q[0], q[1], self.apex, p) {
            let rest = 1.0 - wa;
            let u = if rest > 1e-12 { w1 / rest } else { 0.5 };
            (u, wa.clamp(0.0, 1.0) * self.split)
        } else if let Some([wa, _, w2]) = barycentric(self.apex, q[3], q[2], p) {
            let m = 1.0 - wa;
            let u = if m > 1e-12 { w2 / m } else { 0.5 };
            (u, self.split + m.clamp(0.0, 1.0) * (1.0 - self.split))
        } else {
            return None;
        };
        let u = u.clamp(0.0, 1.0);
        Some(if self.transposed { (v, u) } else { (u, v) })
    }
}

/// The two-triangle map of a layer rect onto a crossed quad.
#[derive(Clone, Copy)]
pub(super) struct CrossedMap {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    hourglass: Hourglass,
}

impl CrossedMap {
    /// The map of `rect` onto `quad`; `None` for an empty rect, a non-finite
    /// corner, or a quad that does not cross itself.
    pub(super) fn new(rect: PsdRect, quad: Quad) -> Option<Self> {
        let (width, height) = (f64::from(rect.width()), f64::from(rect.height()));
        if width <= 0.0
            || height <= 0.0
            || quad.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
        {
            return None;
        }
        Some(Self {
            left: f64::from(rect.left),
            top: f64::from(rect.top),
            width,
            height,
            hourglass: Hourglass::new(&quad)?,
        })
    }
}

impl PlaneMap for CrossedMap {
    fn forward(&self, x: f64, y: f64) -> Option<Point> {
        let u = (x - self.left) / self.width;
        let v = (y - self.top) / self.height;
        Some(self.hourglass.forward(u, v))
    }

    fn inverse(&self, qx: f64, qy: f64) -> Option<Point> {
        let (u, v) = self.hourglass.inverse((qx, qy))?;
        // Just inside the far edge, so a sample on it still lands in the source.
        let x = self.left + (u * self.width).min(self.width - 1e-9);
        let y = self.top + (v * self.height).min(self.height - 1e-9);
        Some((x, y))
    }

    /// Layer-relative: a mask samples where it overlaps the layer rect.
    // ponytail: mask pixels outside the layer rect take the default colour on a
    // crossed quad; extend the map past [0, 1] if that ever matters.
    fn for_rect(&self, _rect: PsdRect) -> Self {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOWTIE: Quad = [(80.0, 0.0), (20.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    // Top corners swapped with a tilted top edge: the side edges cross at
    // (48, 52), off-centre along both.
    const LOPSIDED: Quad = [(100.0, 0.0), (0.0, 20.0), (120.0, 100.0), (0.0, 100.0)];

    fn inside_triangle(a: Point, b: Point, c: Point, p: Point) -> bool {
        let (d0, d1, d2) = (orient(a, b, p), orient(b, c, p), orient(c, a, p));
        (d0 >= 0.0 && d1 >= 0.0 && d2 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0 && d2 <= 0.0)
    }

    #[test]
    fn crossed_quads_are_detected() {
        assert!(!quad_is_crossed(&[
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0)
        ]));
        assert!(!quad_is_crossed(&[
            (-20.0, 0.0),
            (120.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0)
        ]));
        assert!(quad_is_crossed(&BOWTIE));
        // The right corners swapped: the top and bottom edges cross.
        assert!(quad_is_crossed(&[
            (0.0, 0.0),
            (100.0, 100.0),
            (100.0, 0.0),
            (0.0, 100.0)
        ]));
    }

    #[test]
    fn hourglass_round_trips() {
        for quad in [
            BOWTIE,
            LOPSIDED,
            [(0.0, 0.0), (100.0, 100.0), (100.0, 0.0), (0.0, 100.0)],
        ] {
            let map = Hourglass::new(&quad).unwrap();
            for &(u, v) in &[(0.1, 0.1), (0.5, 0.2), (0.9, 0.9), (0.3, 0.7), (0.0, 1.0)] {
                let p = map.forward(u, v);
                let (iu, iv) = map.inverse(p).unwrap();
                let back = map.forward(iu, iv);
                assert!(
                    (back.0 - p.0).abs() < 1e-6 && (back.1 - p.1).abs() < 1e-6,
                    "{quad:?} {u},{v} -> {iu},{iv}"
                );
            }
            for (i, corner) in quad.iter().enumerate() {
                let (u, v) = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)][i];
                let p = map.forward(u, v);
                assert!((p.0 - corner.0).abs() < 1e-9 && (p.1 - corner.1).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn a_lopsided_bowtie_fills_two_straight_triangles() {
        let map = Hourglass::new(&LOPSIDED).unwrap();
        let apex = (48.0, 52.0);
        assert!((map.apex.0 - apex.0).abs() < 1e-9 && (map.apex.1 - apex.1).abs() < 1e-9);
        let [tl, tr, br, bl] = LOPSIDED;
        for y in 0..100 {
            for x in 0..120 {
                let p = (x as f64 + 0.5, y as f64 + 0.5);
                let want = inside_triangle(tl, tr, apex, p) || inside_triangle(apex, br, bl, p);
                assert_eq!(map.inverse(p).is_some(), want, "{p:?}");
            }
        }
    }
}
