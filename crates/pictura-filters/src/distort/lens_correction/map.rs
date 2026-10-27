//! The inverse map: each output point back through Scale, the angle and
//! perspective homography, Remove Distortion, Auto Scale, and the profile's
//! distortion to the source point it shows.

use std::f64::consts::PI;

use pictura_core::homography::HOMOGRAPHY_EPSILON;

use super::{LensCorrectionParams, LensEdge, LensModel};

/// A perspective slider's end keystones like a 30° camera tilt with a
/// focal length of the diagonal.
const MAX_TILT: f64 = PI / 6.0;
/// Remove Distortion +100 (removing barrel) is `k1` −0.25: the corners show
/// the source at three-quarters of their radius, and the map stays
/// monotone.
const BARREL_K1: f64 = 0.25;
/// Remove Distortion −100 (removing pincushion) is `k1` +1.5, divided by
/// `1 + k1` so the corners stay put and the frame stays filled: the centre
/// is magnified 2.5×, the bulge CS6 captures show.
const PINCUSHION_K1: f64 = 1.5;
/// A fringe slider's end scales its channel by ±0.5 %.
const FRINGE_SCALE: f64 = 0.005;
/// Auto Scale's search range.
const AUTO_RANGE: (f64, f64) = (0.25, 4.0);
/// Samples per frame edge when testing a zoom for blank area.
const AUTO_SAMPLES: usize = 64;

/// One parameter set's map over a `width`×`height` layer.
#[derive(Debug, Clone)]
pub struct LensMap {
    width: f64,
    height: f64,
    centre: [f64; 2],
    /// The half-diagonal, the radius the polynomials are normalised by.
    radius: f64,
    scale: f64,
    /// Output (after scale) to undistorted plane; `None` when the angle and
    /// perspective are zero.
    inverse: Option<[f64; 9]>,
    k1: f64,
    /// Remove Distortion's normalisation, `1 / (1 + k1)` for a bulge.
    k1_norm: f64,
    profile: Option<LensModel>,
    auto: f64,
    /// The fringe sliders' radial scale per channel.
    fringe: [f64; 3],
    vignette_amount: f64,
    vignette_start: f64,
    /// Whether the vignette or the profile's falloff changes colour.
    shades: bool,
    pub(super) edge: LensEdge,
}

impl LensMap {
    pub fn new(p: &LensCorrectionParams, width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width.max(1)), f64::from(height.max(1)));
        let profile = p.profile.filter(|m| *m != LensModel::NONE);
        let fringe = |i: usize| 1.0 + FRINGE_SCALE * p.fringe[i].clamp(-100.0, 100.0) / 100.0;
        let d = p.distortion.clamp(-100.0, 100.0) / 100.0;
        let k1 = if d >= 0.0 {
            -BARREL_K1 * d
        } else {
            -PINCUSHION_K1 * d
        };
        let mut map = Self {
            width: w,
            height: h,
            centre: [w / 2.0, h / 2.0],
            radius: w.hypot(h) / 2.0,
            scale: p.scale,
            inverse: None,
            k1,
            k1_norm: if k1 > 0.0 { 1.0 / (1.0 + k1) } else { 1.0 },
            profile,
            auto: 1.0,
            fringe: [0, 1, 2].map(fringe),
            vignette_amount: p.vignette_amount.clamp(-100.0, 100.0) / 100.0,
            vignette_start: 0.7 * p.vignette_midpoint.clamp(0.0, 100.0) / 100.0,
            shades: p.vignette_amount != 0.0 || profile.is_some_and(|m| m.vignette != [0.0; 3]),
            edge: p.edge,
        };
        map.inverse = map.homography(p).and_then(|m| invert(m, map.centre));
        if p.auto_scale && profile.is_some_and(|m| m.distortion != [0.0; 6]) {
            map.auto = map.auto_scale();
        }
        map
    }

    /// The forward angle-and-perspective map, `None` when it is the
    /// identity. The perspective is a keystone about the centre, `(u, v) /
    /// (1 + gx·u + gy·v)` in units of the diagonal, so the centre lines stay
    /// level and plumb as in CS6 captures; it is zoomed so the frame stays
    /// filled (the captures show no blank from the perspective sliders
    /// alone), then rotated on screen. Positive Vertical widens the top,
    /// positive Horizontal narrows the right side.
    fn homography(&self, p: &LensCorrectionParams) -> Option<[f64; 9]> {
        let angle = p.angle.rem_euclid(360.0);
        if p.vertical == 0.0 && p.horizontal == 0.0 && angle == 0.0 {
            return None;
        }
        let [cx, cy] = self.centre;
        let f = 2.0 * self.radius;
        let gy = MAX_TILT.tan() * p.vertical.clamp(-100.0, 100.0) / 100.0;
        let gx = MAX_TILT.tan() * p.horizontal.clamp(-100.0, 100.0) / 100.0;
        let keystone = mul(
            &translate(cx, cy),
            &mul(
                &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, gx / f, gy / f, 1.0],
                &translate(-cx, -cy),
            ),
        );
        let z = self.fill_zoom(&keystone);
        let tilted = mul(&about(cx, cy, [z, 0.0, 0.0, z]), &keystone);
        let t = angle.to_radians();
        let spin = about(cx, cy, [t.cos(), -t.sin(), t.sin(), t.cos()]);
        Some(mul(&spin, &tilted))
    }

    /// The smallest zoom about the centre (1…8) at which the forward
    /// perspective map `m` covers the whole frame. The frame and its image
    /// are convex, so the four corners decide.
    fn fill_zoom(&self, m: &[f64; 9]) -> f64 {
        let Some(inv) = invert(*m, self.centre) else {
            return 1.0;
        };
        let [cx, cy] = self.centre;
        let corners = [
            [0.0, 0.0],
            [self.width, 0.0],
            [0.0, self.height],
            [self.width, self.height],
        ];
        let fits = |z: f64| {
            corners.iter().all(|c| {
                project(&inv, [cx + (c[0] - cx) / z, cy + (c[1] - cy) / z])
                    .is_some_and(|s| self.inside(s, 1e-6))
            })
        };
        let (mut lo, mut hi) = (1.0, 8.0);
        if fits(lo) {
            return lo;
        }
        if !fits(hi) {
            return hi;
        }
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if fits(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    }

    /// The smallest zoom (within [`AUTO_RANGE`]) at which the profile's
    /// distortion maps the whole frame edge inside the layer.
    /// ponytail: bisection on frame-edge samples, so a profile whose radial
    /// map is not monotone may still leave a sliver.
    fn auto_scale(&self) -> f64 {
        let fits = |k: f64| {
            let (w, h) = (self.width, self.height);
            (0..=AUTO_SAMPLES).all(|i| {
                let t = i as f64 / AUTO_SAMPLES as f64;
                [[t * w, 0.0], [t * w, h], [0.0, t * h], [w, t * h]]
                    .into_iter()
                    .all(|q| self.inside(self.profile_point(q, k), 1e-9))
            })
        };
        let (mut lo, mut hi) = AUTO_RANGE;
        if fits(lo) {
            return lo;
        }
        if !fits(hi) {
            return hi;
        }
        for _ in 0..50 {
            let mid = 0.5 * (lo + hi);
            if fits(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        hi
    }

    /// `q` pushed along its radius by `1 + c1 r + … + c6 r⁶`.
    fn radial(&self, q: [f64; 2], c: &[f64; 6], zoom: f64) -> [f64; 2] {
        let d = [
            (q[0] - self.centre[0]) / zoom,
            (q[1] - self.centre[1]) / zoom,
        ];
        let r = d[0].hypot(d[1]) / self.radius;
        let g = 1.0 + c.iter().rev().fold(0.0, |acc, k| (acc + k) * r);
        [self.centre[0] + d[0] * g, self.centre[1] + d[1] * g]
    }

    fn profile_point(&self, q: [f64; 2], zoom: f64) -> [f64; 2] {
        match &self.profile {
            Some(m) => self.radial(q, &m.distortion, zoom),
            None => q,
        }
    }

    pub(super) fn inside(&self, s: [f64; 2], slack: f64) -> bool {
        (-slack..=self.width + slack).contains(&s[0])
            && (-slack..=self.height + slack).contains(&s[1])
    }

    /// The source point output point `p` shows, `None` past the horizon.
    pub fn source(&self, p: [f64; 2]) -> Option<[f64; 2]> {
        let [cx, cy] = self.centre;
        let mut q = [cx + (p[0] - cx) / self.scale, cy + (p[1] - cy) / self.scale];
        if let Some(m) = &self.inverse {
            q = project(m, q)?;
        }
        if self.k1 != 0.0 {
            q = self.radial(q, &LensModel::brown([self.k1, 0.0, 0.0]), 1.0);
            q = [
                cx + (q[0] - cx) * self.k1_norm,
                cy + (q[1] - cy) * self.k1_norm,
            ];
        }
        Some(self.profile_point(q, self.auto))
    }

    /// The point channel `c` samples for source point `s`: the fringe
    /// slider's scale times the profile's `v + c r + b r²`.
    pub(super) fn channel(&self, s: [f64; 2], c: usize) -> [f64; 2] {
        let mut k = self.fringe[c];
        if let Some(m) = &self.profile {
            let [v, a, b] = m.ca[c];
            let r = (s[0] - self.centre[0]).hypot(s[1] - self.centre[1]) / self.radius;
            k *= v + r * (a + r * b);
        }
        [
            self.centre[0] + (s[0] - self.centre[0]) * k,
            self.centre[1] + (s[1] - self.centre[1]) * k,
        ]
    }

    pub(super) fn has_fringe(&self) -> bool {
        self.fringe != [1.0; 3] || self.profile.is_some_and(|m| m.ca != LensModel::NONE.ca)
    }

    pub(super) fn shades(&self) -> bool {
        self.shades
    }

    /// The linear-light gain at source point `s`: the Vignette sliders'
    /// falloff over the profile's divided-out lens falloff. The sliders
    /// weigh a smoothstep from the Midpoint's start radius out to the
    /// corners; +100 is two stops brighter there, −100 black.
    pub(super) fn gain(&self, s: [f64; 2]) -> f64 {
        let r = (s[0] - self.centre[0]).hypot(s[1] - self.centre[1]) / self.radius;
        let mut g = 1.0;
        if self.vignette_amount != 0.0 {
            let t = ((r - self.vignette_start) / (1.0 - self.vignette_start)).clamp(0.0, 1.0);
            let w = t * t * (3.0 - 2.0 * t);
            g = if self.vignette_amount > 0.0 {
                1.0 + 3.0 * self.vignette_amount * w
            } else {
                1.0 + self.vignette_amount * w
            };
        }
        if let Some(m) = &self.profile {
            let r2 = r * r;
            let v = m.vignette;
            let falloff = 1.0 + r2 * (v[0] + r2 * (v[1] + r2 * v[2]));
            if falloff > 1e-3 {
                g /= falloff;
            }
        }
        g
    }
}

fn mul(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            out[r * 3 + c] = (0..3).map(|k| a[r * 3 + k] * b[k * 3 + c]).sum();
        }
    }
    out
}

fn translate(x: f64, y: f64) -> [f64; 9] {
    [1.0, 0.0, x, 0.0, 1.0, y, 0.0, 0.0, 1.0]
}

/// The linear map `[a b; c d]` about `(cx, cy)`.
fn about(cx: f64, cy: f64, l: [f64; 4]) -> [f64; 9] {
    mul(
        &translate(cx, cy),
        &mul(
            &[l[0], l[1], 0.0, l[2], l[3], 0.0, 0.0, 0.0, 1.0],
            &translate(-cx, -cy),
        ),
    )
}

/// `m` applied to `q`, `None` on or behind the horizon (the inverse is
/// normalised so the frame's centre has a positive divisor).
fn project(m: &[f64; 9], q: [f64; 2]) -> Option<[f64; 2]> {
    let w = m[6] * q[0] + m[7] * q[1] + m[8];
    if w.is_nan() || w <= HOMOGRAPHY_EPSILON {
        return None;
    }
    Some([
        (m[0] * q[0] + m[1] * q[1] + m[2]) / w,
        (m[3] * q[0] + m[4] * q[1] + m[5]) / w,
    ])
}

fn invert(m: [f64; 9], centre: [f64; 2]) -> Option<[f64; 9]> {
    let inv = pictura_core::Homography(m).inverse()?.0;
    // The forward map's divisor is positive at the centre; keep the
    // inverse's positive there too.
    let w = inv[6] * centre[0] + inv[7] * centre[1] + inv[8];
    let sign = if w < 0.0 { -1.0 } else { 1.0 };
    Some(inv.map(|v| v * sign))
}
