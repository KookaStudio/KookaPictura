//! The inverse map: each output point back through Scale, the angle and
//! perspective homography, Remove Distortion, Auto Scale, and the profile's
//! distortion to the source point it shows.

use std::f64::consts::PI;

use pictura_core::homography::HOMOGRAPHY_EPSILON;

use super::{LensCorrectionParams, LensEdge, LensModel};

/// The virtual camera tilt at a perspective slider's end.
const MAX_TILT: f64 = PI / 6.0;
/// Remove Distortion ±100 is `k1` ∓0.3, inside the ±⅓ that keeps the
/// radial map monotone out to the corners.
const DISTORTION_K1: f64 = 0.3;
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
    profile: Option<LensModel>,
    auto: f64,
    ca: [f64; 3],
    vignette_amount: f64,
    vignette_start: f64,
    pub(super) edge: LensEdge,
}

impl LensMap {
    pub fn new(p: &LensCorrectionParams, width: u32, height: u32) -> Self {
        let (w, h) = (f64::from(width.max(1)), f64::from(height.max(1)));
        let profile = p.profile.filter(|m| *m != LensModel::NONE);
        let fringe = |i: usize| 1.0 + FRINGE_SCALE * p.fringe[i].clamp(-100.0, 100.0) / 100.0;
        let ca_profile = profile.map_or([1.0; 3], |m| m.ca_scale);
        let mut map = Self {
            width: w,
            height: h,
            centre: [w / 2.0, h / 2.0],
            radius: w.hypot(h) / 2.0,
            scale: p.scale,
            inverse: None,
            k1: -DISTORTION_K1 * p.distortion.clamp(-100.0, 100.0) / 100.0,
            profile,
            auto: 1.0,
            ca: [0, 1, 2].map(|i| fringe(i) * ca_profile[i]),
            vignette_amount: p.vignette_amount.clamp(-100.0, 100.0) / 100.0,
            vignette_start: 0.8 * p.vignette_midpoint.clamp(0.0, 100.0) / 100.0,
            edge: p.edge,
        };
        map.inverse = map.homography(p).and_then(|m| invert(m, map.centre));
        if p.auto_scale && profile.is_some_and(|m| m.distortion != [0.0; 3]) {
            map.auto = map.auto_scale();
        }
        map
    }

    /// The forward angle-and-perspective map, `None` when it is the
    /// identity. The tilt is a virtual camera rotation about the centre
    /// (focal length = the diagonal), re-centred, then rotated on screen.
    fn homography(&self, p: &LensCorrectionParams) -> Option<[f64; 9]> {
        let angle = p.angle.rem_euclid(360.0);
        if p.vertical == 0.0 && p.horizontal == 0.0 && angle == 0.0 {
            return None;
        }
        let [cx, cy] = self.centre;
        let f = 2.0 * self.radius;
        let a = -MAX_TILT * p.vertical.clamp(-100.0, 100.0) / 100.0;
        let b = -MAX_TILT * p.horizontal.clamp(-100.0, 100.0) / 100.0;
        let rx = [1.0, 0.0, 0.0, 0.0, a.cos(), -a.sin(), 0.0, a.sin(), a.cos()];
        let ry = [b.cos(), 0.0, b.sin(), 0.0, 1.0, 0.0, -b.sin(), 0.0, b.cos()];
        let k = [f, 0.0, cx, 0.0, f, cy, 0.0, 0.0, 1.0];
        let k_inv = [1.0 / f, 0.0, -cx / f, 0.0, 1.0 / f, -cy / f, 0.0, 0.0, 1.0];
        let tilt = mul(&k, &mul(&mul(&rx, &ry), &k_inv));
        let moved = project(&tilt, self.centre).unwrap_or(self.centre);
        let back = translate(cx - moved[0], cy - moved[1]);
        let t = angle.to_radians();
        let spin = mul(
            &translate(cx, cy),
            &mul(
                &[t.cos(), -t.sin(), 0.0, t.sin(), t.cos(), 0.0, 0.0, 0.0, 1.0],
                &translate(-cx, -cy),
            ),
        );
        Some(mul(&spin, &mul(&back, &tilt)))
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

    fn radial(&self, q: [f64; 2], k: [f64; 3], zoom: f64) -> [f64; 2] {
        let d = [
            (q[0] - self.centre[0]) / zoom,
            (q[1] - self.centre[1]) / zoom,
        ];
        let r2 = (d[0] * d[0] + d[1] * d[1]) / (self.radius * self.radius);
        let g = 1.0 + r2 * (k[0] + r2 * (k[1] + r2 * k[2]));
        [self.centre[0] + d[0] * g, self.centre[1] + d[1] * g]
    }

    fn profile_point(&self, q: [f64; 2], zoom: f64) -> [f64; 2] {
        match &self.profile {
            Some(m) => self.radial(q, m.distortion, zoom),
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
            q = self.radial(q, [self.k1, 0.0, 0.0], 1.0);
        }
        Some(self.profile_point(q, self.auto))
    }

    /// The point channel `c` samples for source point `s`.
    pub(super) fn channel(&self, s: [f64; 2], c: usize) -> [f64; 2] {
        let k = self.ca[c];
        [
            self.centre[0] + (s[0] - self.centre[0]) * k,
            self.centre[1] + (s[1] - self.centre[1]) * k,
        ]
    }

    pub(super) fn has_fringe(&self) -> bool {
        self.ca != [1.0; 3]
    }

    /// The colour gain at source point `s`: the Vignette sliders' falloff
    /// over the profile's divided-out lens falloff.
    pub(super) fn gain(&self, s: [f64; 2]) -> f64 {
        let r = (s[0] - self.centre[0]).hypot(s[1] - self.centre[1]) / self.radius;
        let mut g = 1.0;
        if self.vignette_amount != 0.0 {
            let t = ((r - self.vignette_start) / (1.0 - self.vignette_start)).clamp(0.0, 1.0);
            g += self.vignette_amount * t * t;
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
