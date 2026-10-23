//! Filter ▸ Adaptive Wide Angle (#346, `FILT-094`). Adobe's projection and
//! solver are closed; this follows the published method the filter is known
//! to build on (Carroll, Agrawala, Agarwala, "Optimizing Content-Preserving
//! Projections for Wide-Angle Images", SIGGRAPH 2009). The layer is mapped to
//! the viewing sphere by a lens model, projected stereographically, and a
//! coarse mesh is solved so each constraint's great circle comes out
//! straight. *(approximation: behavioural parity only)*
//!
//! ponytail: no lens-profile database, Auto or Panorama correction, and no
//! GPU path.

mod render;
mod solve;
#[cfg(test)]
mod tests;

use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

pub use render::AwaSource;

/// The 35 mm full-frame diagonal, in millimetres.
const FULL_FRAME_DIAGONAL: f64 = 43.27;
/// Coarse (solved) cells across the layer's longer side.
const COARSE_CELLS: f64 = 48.0;
/// The largest fine (rendered) cell, in layer pixels.
const FINE_CELL: f64 = 16.0;
/// How far past its 90° rim a fisheye's mesh is extrapolated; pixels that
/// land past the rim are discarded, so the rim itself stays sharp.
const FISHEYE_LIMIT: f64 = 0.9 * PI;

/// The projection the source layer was shot with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwaCorrection {
    /// Equidistant fisheye: the off-axis angle grows linearly with radius.
    Fisheye,
    /// Rectilinear: straight scene lines are straight in the source.
    Perspective,
    /// A 2:1 equirectangular 360° panorama.
    FullSpherical,
}

/// What a constraint's output line must do besides being straight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AwaOrientation {
    Unfixed,
    Horizontal,
    Vertical,
    /// At this angle in radians, `atan2(dy, dx)` in output pixels (y down).
    Fixed(f64),
}

/// A user-drawn constraint: two points for a line, three or more corners for
/// a polygon, each as a fraction of the layer's width and height.
#[derive(Debug, Clone, PartialEq)]
pub struct AwaConstraint {
    pub points: Vec<[f64; 2]>,
    pub polygon: bool,
    pub orientation: AwaOrientation,
}

/// Everything the commit needs. `scale` is a fraction (1.0 = 100 %),
/// `offset` a fraction of the layer size.
#[derive(Debug, Clone, PartialEq)]
pub struct AwaParams {
    pub correction: AwaCorrection,
    pub scale: f64,
    pub focal_length: f64,
    pub crop_factor: f64,
    pub offset: [f64; 2],
    pub constraints: Arc<Vec<AwaConstraint>>,
}

impl Default for AwaParams {
    fn default() -> Self {
        Self {
            correction: AwaCorrection::Fisheye,
            scale: 1.0,
            // A normal lens (43.27 mm equivalent) at CS6's opening crop.
            focal_length: 75.91,
            crop_factor: 0.57,
            offset: [0.0; 2],
            constraints: Arc::default(),
        }
    }
}

/// The lens model: source pixels to directions on the viewing sphere and
/// back, and the base (stereographic) projection.
#[derive(Debug, Clone, Copy)]
struct Lens {
    correction: AwaCorrection,
    /// Focal length in pixels.
    f: f64,
    w: f64,
    h: f64,
}

impl Lens {
    fn new(params: &AwaParams, w: f64, h: f64) -> Self {
        let mm = params.focal_length * params.crop_factor;
        Self {
            correction: params.correction,
            f: (mm / FULL_FRAME_DIAGONAL * w.hypot(h)).max(1e-6),
            w,
            h,
        }
    }

    fn centre(&self) -> [f64; 2] {
        [self.w / 2.0, self.h / 2.0]
    }

    /// The direction source point `p` sees; `None` past a fisheye's 90°.
    fn dir(&self, p: [f64; 2]) -> Option<[f64; 3]> {
        let c = self.centre();
        let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
        match self.correction {
            AwaCorrection::Perspective => Some(normalize([dx, dy, self.f])),
            AwaCorrection::Fisheye => {
                let r = dx.hypot(dy);
                let theta = r / self.f;
                if theta > FRAC_PI_2 + 1e-9 {
                    return None;
                }
                if r < 1e-12 {
                    return Some([0.0, 0.0, 1.0]);
                }
                let s = theta.sin() / r;
                Some([dx * s, dy * s, theta.cos()])
            }
            AwaCorrection::FullSpherical => {
                let lon = (p[0] / self.w - 0.5) * 2.0 * PI;
                let lat = (0.5 - p[1] / self.h) * PI;
                Some([lat.cos() * lon.sin(), -lat.sin(), lat.cos() * lon.cos()])
            }
        }
    }

    /// The source point that sees direction `v`, if the lens covers it.
    fn source(&self, v: [f64; 3]) -> Option<[f64; 2]> {
        let c = self.centre();
        match self.correction {
            AwaCorrection::Perspective => {
                (v[2] > 1e-9).then(|| [c[0] + self.f * v[0] / v[2], c[1] + self.f * v[1] / v[2]])
            }
            AwaCorrection::Fisheye => {
                let theta = v[2].clamp(-1.0, 1.0).acos();
                if theta > FRAC_PI_2 + 1e-9 {
                    return None;
                }
                let s = v[0].hypot(v[1]);
                if s < 1e-15 {
                    return Some(c);
                }
                let r = self.f * theta / s;
                Some([c[0] + v[0] * r, c[1] + v[1] * r])
            }
            AwaCorrection::FullSpherical => {
                let lon = v[0].atan2(v[2]);
                let lat = (-v[1]).clamp(-1.0, 1.0).asin();
                Some([(lon / (2.0 * PI) + 0.5) * self.w, (0.5 - lat / PI) * self.h])
            }
        }
    }

    /// Whether a fisheye source point lies past 90°, outside the picture.
    fn clipped(&self, p: [f64; 2]) -> bool {
        let c = self.centre();
        self.correction == AwaCorrection::Fisheye
            && (p[0] - c[0]).hypot(p[1] - c[1]) / self.f > FRAC_PI_2 + 1e-9
    }

    /// The base projection of source point `p`: stereographic with unit
    /// magnification at the centre (a fisheye extrapolated past its rim up to
    /// `FISHEYE_LIMIT`), or the equirectangular picture itself for Full
    /// Spherical.
    fn base(&self, p: [f64; 2]) -> [f64; 2] {
        let c = self.centre();
        let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
        let r = dx.hypot(dy);
        let theta = match self.correction {
            AwaCorrection::FullSpherical => return p,
            AwaCorrection::Fisheye => (r / self.f).min(FISHEYE_LIMIT),
            AwaCorrection::Perspective => (r / self.f).atan(),
        };
        if r < 1e-12 {
            return c;
        }
        let k = 2.0 * self.f * (theta / 2.0).tan() / r;
        [c[0] + dx * k, c[1] + dy * k]
    }
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The great circle from source point `a` to `b`, sampled about every `step`
/// source pixels; `None` when it cannot be computed.
fn trace_arc(lens: &Lens, a: [f64; 2], b: [f64; 2], step: f64) -> Option<Vec<[f64; 2]>> {
    if (a[0] - b[0]).hypot(a[1] - b[1]) < 0.5 {
        return None;
    }
    let (va, vb) = (lens.dir(a)?, lens.dir(b)?);
    let omega = dot(va, vb).clamp(-1.0, 1.0).acos();
    if !(1e-9..PI - 1e-3).contains(&omega) {
        return None;
    }
    let reach = (a[0] - b[0]).hypot(a[1] - b[1]).max(omega * lens.f);
    let n = ((reach / step).ceil() as usize).clamp(8, 512);
    let sin = omega.sin();
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let t = k as f64 / n as f64;
        let (ka, kb) = (((1.0 - t) * omega).sin() / sin, (t * omega).sin() / sin);
        let v = [
            ka * va[0] + kb * vb[0],
            ka * va[1] + kb * vb[1],
            ka * va[2] + kb * vb[2],
        ];
        let Some(p) = lens.source(v) else {
            continue;
        };
        if !(-1e-6..=lens.w + 1e-6).contains(&p[0]) || !(-1e-6..=lens.h + 1e-6).contains(&p[1]) {
            continue;
        }
        if lens.correction == AwaCorrection::FullSpherical {
            if let Some(last) = out.last() {
                if (p[0] - last[0]).abs() > lens.w / 2.0 {
                    return None; // across the panorama's seam
                }
            }
        }
        out.push(p);
    }
    (out.len() >= 2).then_some(out)
}

/// A constraint's traced edges in source pixels, or `None` when invalid.
fn trace_constraint(lens: &Lens, c: &AwaConstraint, step: f64) -> Option<Vec<Vec<[f64; 2]>>> {
    let pts: Vec<[f64; 2]> = c
        .points
        .iter()
        .map(|p| [p[0] * lens.w, p[1] * lens.h])
        .collect();
    if !c.polygon {
        return match pts.as_slice() {
            [a, b] => Some(vec![trace_arc(lens, *a, *b, step)?]),
            _ => None,
        };
    }
    if pts.len() < 3 {
        return None;
    }
    let area: f64 = (0..pts.len())
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        / 2.0;
    if area.abs() < 1.0 {
        return None;
    }
    (0..pts.len())
        .map(|i| trace_arc(lens, pts[i], pts[(i + 1) % pts.len()], step))
        .collect()
}

/// A regular grid of `cx`×`cy` cells over the layer.
#[derive(Debug, Clone, Copy)]
struct Grid {
    cx: usize,
    cy: usize,
    sx: f64,
    sy: f64,
}

impl Grid {
    fn new(cx: usize, cy: usize, w: f64, h: f64) -> Self {
        Self {
            cx,
            cy,
            sx: w / cx as f64,
            sy: h / cy as f64,
        }
    }

    fn coarse(w: f64, h: f64) -> Self {
        let cell = (w.max(h) / COARSE_CELLS).max(1.0);
        Self::new(
            ((w / cell).ceil() as usize).max(1),
            ((h / cell).ceil() as usize).max(1),
            w,
            h,
        )
    }

    fn nx(&self) -> usize {
        self.cx + 1
    }

    fn vertices(&self) -> usize {
        self.nx() * (self.cy + 1)
    }

    fn at(&self, i: usize, j: usize) -> [f64; 2] {
        [i as f64 * self.sx, j as f64 * self.sy]
    }

    /// The four vertices around source point `p` and their bilinear weights.
    fn weights(&self, p: [f64; 2]) -> [(usize, f64); 4] {
        let gx = (p[0] / self.sx).clamp(0.0, self.cx as f64);
        let gy = (p[1] / self.sy).clamp(0.0, self.cy as f64);
        let i = (gx.floor() as usize).min(self.cx - 1);
        let j = (gy.floor() as usize).min(self.cy - 1);
        let (fx, fy) = (gx - i as f64, gy - j as f64);
        let k = j * self.nx() + i;
        [
            (k, (1.0 - fx) * (1.0 - fy)),
            (k + 1, fx * (1.0 - fy)),
            (k + self.nx(), (1.0 - fx) * fy),
            (k + self.nx() + 1, fx * fy),
        ]
    }
}

/// A solved correction for one layer size: where every source point lands
/// in the output.
#[derive(Debug, Clone)]
pub struct AwaWarp {
    lens: Lens,
    scale: f64,
    offset: [f64; 2],
    coarse: Grid,
    /// Solved coarse positions minus their base projection.
    delta: Vec<[f64; 2]>,
    fine: Grid,
    /// Fine vertices' output positions.
    fine_out: Vec<[f64; 2]>,
    fine_clipped: Vec<bool>,
    subdivision: usize,
    /// Per constraint: its traced edges in source pixels, `None` if invalid.
    traces: Vec<Option<Vec<Vec<[f64; 2]>>>>,
    orientations: Vec<AwaOrientation>,
}

impl AwaWarp {
    /// Solve `params` for a `w`×`h` layer.
    pub fn solve(params: &AwaParams, w: u32, h: u32) -> Self {
        let (w, h) = (f64::from(w.max(1)), f64::from(h.max(1)));
        let lens = Lens::new(params, w, h);
        let coarse = Grid::coarse(w, h);
        let base: Vec<[f64; 2]> = (0..=coarse.cy)
            .flat_map(|j| (0..=coarse.cx).map(move |i| (i, j)))
            .map(|(i, j)| lens.base(coarse.at(i, j)))
            .collect();
        let step = coarse.sx.min(coarse.sy) / 2.0;
        let traces: Vec<_> = params
            .constraints
            .iter()
            .map(|c| trace_constraint(&lens, c, step))
            .collect();
        let solved = solve::solve(&lens, &coarse, &base, &params.constraints, &traces);
        let delta: Vec<[f64; 2]> = solved
            .iter()
            .zip(&base)
            .map(|(v, b)| [v[0] - b[0], v[1] - b[1]])
            .collect();
        let subdivision = ((coarse.sx.max(coarse.sy) / FINE_CELL).ceil() as usize).max(1);
        let fine = Grid::new(coarse.cx * subdivision, coarse.cy * subdivision, w, h);
        let mut warp = Self {
            lens,
            scale: params.scale,
            offset: [params.offset[0] * w, params.offset[1] * h],
            coarse,
            delta,
            fine,
            fine_out: Vec::new(),
            fine_clipped: Vec::new(),
            subdivision,
            traces,
            orientations: params.constraints.iter().map(|c| c.orientation).collect(),
        };
        let points: Vec<[f64; 2]> = (0..=fine.cy)
            .flat_map(|j| (0..=fine.cx).map(move |i| fine.at(i, j)))
            .collect();
        warp.fine_clipped = points.iter().map(|&p| lens.clipped(p)).collect();
        warp.fine_out = points.iter().map(|&p| warp.forward(p)).collect();
        warp
    }

    /// The layer size the warp was solved for.
    pub fn size(&self) -> [f64; 2] {
        [self.lens.w, self.lens.h]
    }

    fn place(&self, q: [f64; 2]) -> [f64; 2] {
        let c = self.lens.centre();
        [
            c[0] + self.scale * (q[0] - c[0]) + self.offset[0],
            c[1] + self.scale * (q[1] - c[1]) + self.offset[1],
        ]
    }

    /// Where source point `p` lands in the output.
    pub fn forward(&self, p: [f64; 2]) -> [f64; 2] {
        let b = self.lens.base(p);
        let mut d = [0.0; 2];
        for (k, wt) in self.coarse.weights(p) {
            d[0] += wt * self.delta[k][0];
            d[1] += wt * self.delta[k][1];
        }
        self.place([b[0] + d[0], b[1] + d[1]])
    }

    /// The source point output point `q` shows, if any.
    pub fn inverse(&self, q: [f64; 2]) -> Option<[f64; 2]> {
        self.triangles()
            .find_map(|[a, b, c]| {
                let (oa, ob, oc) = (self.fine_out[a], self.fine_out[b], self.fine_out[c]);
                let area = edge(oa, ob, oc);
                if area.abs() < 1e-12 {
                    return None;
                }
                let la = edge(ob, oc, q) / area;
                let lb = edge(oc, oa, q) / area;
                let lc = 1.0 - la - lb;
                (la >= -1e-9 && lb >= -1e-9 && lc >= -1e-9).then_some([(a, la), (b, lb), (c, lc)])
            })
            .map(|bary| {
                let mut s = [0.0; 2];
                for (k, l) in bary {
                    let p = self.fine_point(k);
                    s[0] += l * p[0];
                    s[1] += l * p[1];
                }
                s
            })
            .filter(|&s| !self.lens.clipped(s))
    }

    fn fine_point(&self, k: usize) -> [f64; 2] {
        self.fine.at(k % self.fine.nx(), k / self.fine.nx())
    }

    /// The fine mesh's drawable triangles, in grid order. Triangles wholly
    /// past a fisheye's rim are dropped; the rest are clipped per pixel.
    fn triangles(&self) -> impl Iterator<Item = [usize; 3]> + '_ {
        let nx = self.fine.nx();
        (0..self.fine.cy)
            .flat_map(move |j| (0..self.fine.cx).map(move |i| j * nx + i))
            .flat_map(move |k| [[k, k + 1, k + nx + 1], [k, k + nx + 1, k + nx]])
            .filter(|t| !t.iter().all(|&k| self.fine_clipped[k]))
    }

    /// The number of constraints the warp was solved with.
    pub fn constraint_count(&self) -> usize {
        self.traces.len()
    }

    /// Constraint `i`'s orientation, or `None` when it is invalid.
    pub fn constraint_state(&self, i: usize) -> Option<AwaOrientation> {
        self.traces
            .get(i)?
            .as_ref()
            .and(self.orientations.get(i).copied())
    }

    /// Constraint `i`'s traced edges, in output pixels (`output`) or source
    /// pixels.
    pub fn constraint_paths(&self, i: usize, output: bool) -> Vec<Vec<[f64; 2]>> {
        let Some(Some(edges)) = self.traces.get(i) else {
            return Vec::new();
        };
        edges.iter().map(|e| self.map_path(e, output)).collect()
    }

    /// The great circle from source point `a` to `b`, in output or source
    /// pixels; empty when it cannot be computed.
    pub fn arc(&self, a: [f64; 2], b: [f64; 2], output: bool) -> Vec<[f64; 2]> {
        let step = self.coarse.sx.min(self.coarse.sy) / 2.0;
        trace_arc(&self.lens, a, b, step)
            .map(|path| self.map_path(&path, output))
            .unwrap_or_default()
    }

    fn map_path(&self, path: &[[f64; 2]], output: bool) -> Vec<[f64; 2]> {
        if output {
            path.iter().map(|&p| self.forward(p)).collect()
        } else {
            path.to_vec()
        }
    }

    /// The solved (coarse) grid's lines in output pixels, traced through the
    /// fine mesh.
    pub fn mesh_lines(&self) -> Vec<Vec<[f64; 2]>> {
        let nx = self.fine.nx();
        let s = self.subdivision;
        let rows = (0..=self.fine.cy).step_by(s).map(|j| {
            (0..=self.fine.cx)
                .map(|i| self.fine_out[j * nx + i])
                .collect()
        });
        let cols = (0..=self.fine.cx).step_by(s).map(|i| {
            (0..=self.fine.cy)
                .map(|j| self.fine_out[j * nx + i])
                .collect()
        });
        rows.chain(cols).collect()
    }
}

/// Twice the signed area of `a b p`.
fn edge(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// Whether a `w`×`h` layer is the 2:1 panorama Full Spherical needs (within
/// a pixel).
pub fn awa_two_to_one(w: u32, h: u32) -> bool {
    (i64::from(w) - 2 * i64::from(h)).abs() <= 1
}

/// Apply `params` to `buf` (three or four planes). Pixels no source covers
/// turn transparent, or white when there is no alpha plane. Full Spherical
/// on a layer that is not 2:1 is refused, whichever path asks for it.
pub fn adaptive_wide_angle(buf: &mut PixelBuffer, params: &AwaParams) -> Result<(), FilterError> {
    validate(buf)?;
    if params.correction == AwaCorrection::FullSpherical && !awa_two_to_one(buf.width, buf.height) {
        return Err(FilterError::InvalidParams(
            "Full Spherical needs a 2:1 panorama".into(),
        ));
    }
    let warp = AwaWarp::solve(params, buf.width, buf.height);
    let source = AwaSource::from_buffer(buf);
    render::commit(&warp, &source, buf);
    Ok(())
}
