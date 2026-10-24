//! A plane family's flat editing space. A root plane's flat coordinates are
//! its frame coordinates in metric units; each related plane unfolds about
//! its hinge into its parent's flat space, keeping distances from the hinge.
//! Pasted pixels, marquees and brush dabs are placed and sized here and
//! reach the image through the plane under each pixel.

use pictura_core::Homography;

use super::geometry::{hinge_axes, Validity, P2};
use super::planes::VpPlanes;

/// `flat = o + s·a + t·b` for frame coordinates `(s, t)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Affine {
    o: P2,
    a: P2,
    b: P2,
}

impl Affine {
    fn apply(&self, s: f64, t: f64) -> P2 {
        (
            self.o.0 + s * self.a.0 + t * self.b.0,
            self.o.1 + s * self.a.1 + t * self.b.1,
        )
    }

    fn invert(&self, p: P2) -> Option<P2> {
        let det = self.a.0 * self.b.1 - self.a.1 * self.b.0;
        if det.abs() < 1e-12 {
            return None;
        }
        let (dx, dy) = (p.0 - self.o.0, p.1 - self.o.1);
        Some((
            (dx * self.b.1 - dy * self.b.0) / det,
            (self.a.0 * dy - self.a.1 * dx) / det,
        ))
    }
}

#[derive(Clone, Debug)]
struct PlaneMap {
    index: usize,
    to_image: Homography,
    to_frame: Homography,
    flat: Affine,
    rect: [f64; 4],
    quad: [P2; 4],
}

/// The flat space of the family grown from `root`: every valid plane in it,
/// with its image and flat mappings.
#[derive(Clone, Debug)]
pub struct FamilyMap {
    pub root: usize,
    planes: Vec<PlaneMap>,
}

fn inside_quad(q: &[P2; 4], p: P2) -> bool {
    let mut inside = false;
    for k in 0..4 {
        let (a, b) = (q[k], q[(k + 1) % 4]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
            inside = !inside;
        }
    }
    inside
}

impl FamilyMap {
    /// The family `root` grows from; `None` when its root is not valid.
    pub fn new(planes: &VpPlanes, root: usize) -> Option<Self> {
        if root >= planes.len() || planes.validity(root) != Validity::Valid {
            return None;
        }
        let mut maps: Vec<PlaneMap> = Vec::new();
        for i in root..planes.len() {
            if planes.root_of(i) != root || planes.validity(i) != Validity::Valid {
                continue;
            }
            let Some(frame) = planes.frame(i).copied() else {
                continue;
            };
            let flat = match planes.hinge_of(i) {
                None => {
                    let k = 1.0 / planes.unit(i);
                    Affine {
                        o: (0.0, 0.0),
                        a: (k, 0.0),
                        b: (0.0, k),
                    }
                }
                Some((parent, edge)) => {
                    let Some(up) = maps.iter().find(|m| m.index == parent) else {
                        continue;
                    };
                    let Some(pf) = planes.frame(parent) else {
                        continue;
                    };
                    let Some((_, _, out)) = hinge_axes(pf, edge) else {
                        continue;
                    };
                    let Some((oa, ob)) = pf.components(out) else {
                        continue;
                    };
                    let [u0, u1, v0, v1] = pf.rect;
                    let p = up.flat;
                    let (o, a) = match edge {
                        0 => (p.apply(0.0, v0), p.a),
                        1 => (p.apply(u1, 0.0), p.b),
                        2 => (p.apply(0.0, v1), p.a),
                        _ => (p.apply(u0, 0.0), p.b),
                    };
                    let b = (oa * p.a.0 + ob * p.b.0, oa * p.a.1 + ob * p.b.1);
                    Affine { o, a, b }
                }
            };
            let quad = planes.corners(i);
            let Some(to_image) = Homography::from_quads(frame.corner_coords(), quad) else {
                continue;
            };
            let Some(to_frame) = to_image.inverse() else {
                continue;
            };
            maps.push(PlaneMap {
                index: i,
                to_image,
                to_frame,
                flat,
                rect: frame.rect,
                quad,
            });
        }
        (!maps.is_empty()).then_some(FamilyMap { root, planes: maps })
    }

    /// The family's planes, `prefer` first, then the newest.
    fn order(&self, prefer: Option<usize>) -> impl Iterator<Item = &PlaneMap> {
        let first = self.planes.iter().filter(move |m| Some(m.index) == prefer);
        first.chain(
            self.planes
                .iter()
                .rev()
                .filter(move |m| Some(m.index) != prefer),
        )
    }

    /// The plane under image point `p` and the flat point it shows.
    pub fn locate(&self, p: P2, prefer: Option<usize>) -> Option<(usize, P2)> {
        self.order(prefer).find_map(|m| {
            if !inside_quad(&m.quad, p) {
                return None;
            }
            let (s, t) = m.to_frame.apply(p.0, p.1)?;
            Some((m.index, m.flat.apply(s, t)))
        })
    }

    /// Where flat point `f` shows in the image: through the plane whose
    /// unfolded rect holds it, or extrapolated through `prefer`.
    pub fn to_image(&self, f: P2, prefer: Option<usize>) -> Option<P2> {
        const SLACK: f64 = 1e-9;
        let inside = self.order(prefer).find_map(|m| {
            let (s, t) = m.flat.invert(f)?;
            let [u0, u1, v0, v1] = m.rect;
            (s >= u0 - SLACK && s <= u1 + SLACK && t >= v0 - SLACK && t <= v1 + SLACK)
                .then(|| m.to_image.apply(s, t))
                .flatten()
        });
        inside.or_else(|| {
            let m = self.order(prefer).next()?;
            let (s, t) = m.flat.invert(f)?;
            m.to_image.apply(s, t)
        })
    }

    /// Plane `i`'s rect in flat space, as its four corners.
    pub fn flat_corners(&self, i: usize) -> Option<[P2; 4]> {
        let m = self.planes.iter().find(|m| m.index == i)?;
        let [u0, u1, v0, v1] = m.rect;
        Some([
            m.flat.apply(u0, v0),
            m.flat.apply(u1, v0),
            m.flat.apply(u1, v1),
            m.flat.apply(u0, v1),
        ])
    }

    /// The image bounding box of the family's planes, `[x0, y0, x1, y1]`.
    pub fn image_bounds(&self) -> [f64; 4] {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in self.planes.iter().flat_map(|m| m.quad) {
            b = [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)];
        }
        b
    }
}

/// The family an image point falls in: the first valid plane (newest
/// first) containing it, as `(map, plane, flat point)`.
pub fn family_at(
    planes: &VpPlanes,
    p: P2,
    prefer: Option<usize>,
) -> Option<(FamilyMap, usize, P2)> {
    let mut order: Vec<usize> = prefer.into_iter().collect();
    order.extend((0..planes.len()).rev().filter(|&i| Some(i) != prefer));
    order.into_iter().find_map(|i| {
        if planes.validity(i) != Validity::Valid || !planes.contains(i, p) {
            return None;
        }
        let map = FamilyMap::new(planes, planes.root_of(i))?;
        let (plane, flat) = map.locate(p, Some(i))?;
        Some((map, plane, flat))
    })
}
