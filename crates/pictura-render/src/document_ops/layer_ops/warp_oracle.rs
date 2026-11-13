//! The dense mesh-warp rasterizer the tiled one replaced (#1292), kept
//! unchanged as the test oracle: whole-source premultiplied planes in, whole
//! destination planes out.

use pictura_core::PsdRect;
use rayon::prelude::*;

use super::kernel::{self, Interpolation};
use super::warp_raster::{Triangles, BAND, MAX_SS};

/// Unit-domain source planes over one rect. With an `alpha` plane the other
/// planes are stored premultiplied by it.
pub(super) struct Planes {
    pub(super) data: Vec<Vec<f32>>,
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) alpha: Option<usize>,
}

impl Planes {
    /// Wrap straight-alpha unit planes, premultiplying by `alpha` when given.
    pub(super) fn new(mut data: Vec<Vec<f32>>, w: usize, h: usize, alpha: Option<usize>) -> Planes {
        if let Some(a) = alpha {
            let al = std::mem::take(&mut data[a]);
            for (k, plane) in data.iter_mut().enumerate() {
                if k != a {
                    plane.iter_mut().zip(&al).for_each(|(v, a)| *v *= a);
                }
            }
            data[a] = al;
        }
        Planes { data, w, h, alpha }
    }

    /// Add `weight ×` the pixel at `(x, y)` (zero outside) to `acc`.
    fn tap(&self, x: i64, y: i64, weight: f64, acc: &mut [f64]) {
        if weight == 0.0 || x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return;
        }
        let o = y as usize * self.w + x as usize;
        for (s, plane) in acc.iter_mut().zip(&self.data) {
            *s += f64::from(plane[o]) * weight;
        }
    }

    /// One sample at local position `(u, v)` (pixel centres at integers) into
    /// `acc`, which is cleared first.
    fn point(&self, interp: Interpolation, u: f64, v: f64, acc: &mut [f64]) {
        acc.fill(0.0);
        if let Some((b, c)) = kernel::cubic_params(interp) {
            // Snap to the pixel grid when within float noise, so identity is exact.
            let snap = |t: f64| {
                if (t - t.round()).abs() < 1e-7 {
                    t.round()
                } else {
                    t
                }
            };
            let (u, v) = (snap(u), snap(v));
            let (ix, iy) = (u.floor(), v.floor());
            let weights = |t: f64| {
                [
                    kernel::cubic_weight(1.0 + t, b, c),
                    kernel::cubic_weight(t, b, c),
                    kernel::cubic_weight(1.0 - t, b, c),
                    kernel::cubic_weight(2.0 - t, b, c),
                ]
            };
            let (wx, wy) = (weights(u - ix), weights(v - iy));
            for (j, wyj) in wy.iter().enumerate() {
                for (i, wxi) in wx.iter().enumerate() {
                    self.tap(
                        ix as i64 - 1 + i as i64,
                        iy as i64 - 1 + j as i64,
                        wxi * wyj,
                        acc,
                    );
                }
            }
            return;
        }
        if interp == Interpolation::Nearest {
            self.tap((u + 0.5).floor() as i64, (v + 0.5).floor() as i64, 1.0, acc);
            return;
        }
        let (ix, iy) = (u.floor(), v.floor());
        let (fx, fy) = (u - ix, v - iy);
        let (ix, iy) = (ix as i64, iy as i64);
        for (dy, wy) in [(0, 1.0 - fy), (1, fy)] {
            for (dx, wx) in [(0, 1.0 - fx), (1, fx)] {
                self.tap(ix + dx, iy + dy, wx * wy, acc);
            }
        }
    }
}

/// Resample `src` (its pixels over `rect`) through `t` into `t.dest`, one
/// unit-domain plane per source plane. Uncovered pixels are zero in every
/// plane; with an alpha plane the colour planes come out straight again.
/// `hdr` leaves colour values unclamped (32-bit planes).
pub(super) fn dense_warp_planes(
    src: &Planes,
    rect: PsdRect,
    t: &Triangles,
    interp: Interpolation,
    hdr: bool,
) -> Vec<Vec<f32>> {
    let (dl, dt, dr, db) = t.dest;
    let dw = (dr - dl) as usize;
    let bands: Vec<(i32, i32)> = (dt..db)
        .step_by(BAND as usize)
        .map(|y| (y, (y + BAND).min(db)))
        .collect();
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); bands.len()];
    for (k, tri) in t.tris.iter().enumerate() {
        let ys = tri.map(|v| t.verts[v].0[1]);
        let lo = ((ys.iter().fold(f64::MAX, |a, &b| a.min(b)) - 0.5).floor() as i32).max(dt);
        let hi = ((ys.iter().fold(f64::MIN, |a, &b| a.max(b)) + 0.5).ceil() as i32).min(db - 1);
        if hi < lo {
            continue;
        }
        for bin in &mut bins[((lo - dt) / BAND) as usize..=((hi - dt) / BAND) as usize] {
            bin.push(k);
        }
    }
    let n = src.data.len();
    let job = pictura_core::job::current();
    let rendered: Vec<Vec<Vec<f32>>> = bands
        .par_iter()
        .zip(bins.par_iter())
        .map(|(&(y0, y1), tris)| {
            if job.as_ref().is_some_and(|job| job.cancelled()) {
                // The cancelled result is dropped; keep the band's shape.
                return vec![vec![0.0; dw * (y1 - y0) as usize]; n];
            }
            let band = render_band(src, rect, t, tris, (dl, y0, dr, y1), interp, hdr);
            if let Some(job) = &job {
                job.advance((y1 - y0) as u64);
            }
            band
        })
        .collect();
    let mut out: Vec<Vec<f32>> = vec![Vec::with_capacity(dw * (db - dt) as usize); n];
    for band in rendered {
        for (plane, part) in out.iter_mut().zip(band) {
            plane.extend(part);
        }
    }
    out
}

/// One destination band `(left, top, right, bottom)` of [`dense_warp_planes`].
fn render_band(
    src: &Planes,
    rect: PsdRect,
    t: &Triangles,
    tris: &[usize],
    band: (i32, i32, i32, i32),
    interp: Interpolation,
    hdr: bool,
) -> Vec<Vec<f32>> {
    let (bl, bt, br, bb) = band;
    let (w, h) = ((br - bl) as usize, (bb - bt) as usize);
    let n = src.data.len();
    let mut out = vec![vec![0.0f32; w * h]; n];
    if tris.is_empty() {
        return out;
    }
    // Subsamples along one destination axis, from the source step `j` it makes:
    // one where the warp keeps or enlarges the source, else about one per source
    // pixel the output pixel spans (an area average; a single kernel sample
    // aliases thin lines when shrinking). The subsamples are bilinear: the box
    // does the filtering.
    let count = |j: [f64; 2]| {
        if interp == Interpolation::Nearest {
            1
        } else {
            j[0].hypot(j[1]).round().clamp(1.0, MAX_SS) as usize
        }
    };
    // Source position per destination pixel (NaN = not covered) and the
    // triangle it came from. Per triangle: its source step per destination pixel
    // along x and y (`[dx_u, dx_v, dy_u, dy_v]`, the inverse map's Jacobian,
    // constant over a triangle) and the subsamples along each.
    let mut uv = vec![[f64::NAN; 2]; w * h];
    let mut owner = vec![0u32; w * h];
    let mut jac: Vec<([f64; 4], usize, usize)> = Vec::with_capacity(tris.len());
    for (k, &tri) in tris.iter().enumerate() {
        let [(p0, s0), (p1, s1), (p2, s2)] = t.tris[tri].map(|v| t.verts[v]);
        let det = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1]);
        if det.abs() < 1e-12 {
            jac.push(([0.0; 4], 1, 1));
            continue;
        }
        let (d1, d2) = (
            [s1[0] - s0[0], s1[1] - s0[1]],
            [s2[0] - s0[0], s2[1] - s0[1]],
        );
        let (l1x, l1y, l2x, l2y) = (
            (p2[1] - p0[1]) / det,
            (p0[0] - p2[0]) / det,
            (p0[1] - p1[1]) / det,
            (p1[0] - p0[0]) / det,
        );
        // Capped, so a sliver triangle cannot ask for an unbounded footprint.
        let cap = |j: [f64; 2]| {
            let l = j[0].hypot(j[1]);
            if l > MAX_SS {
                [j[0] * MAX_SS / l, j[1] * MAX_SS / l]
            } else {
                j
            }
        };
        let jx = cap([l1x * d1[0] + l2x * d2[0], l1x * d1[1] + l2x * d2[1]]);
        let jy = cap([l1y * d1[0] + l2y * d2[0], l1y * d1[1] + l2y * d2[1]]);
        let (nx, ny) = (count(jx), count(jy));
        jac.push(if nx * ny == 1 {
            ([0.0; 4], 1, 1)
        } else {
            ([jx[0], jx[1], jy[0], jy[1]], nx, ny)
        });
        let bx0 = (p0[0].min(p1[0]).min(p2[0]) - 0.5)
            .floor()
            .max(f64::from(bl)) as i32;
        let by0 = (p0[1].min(p1[1]).min(p2[1]) - 0.5)
            .floor()
            .max(f64::from(bt)) as i32;
        let bx1 = (p0[0].max(p1[0]).max(p2[0]) + 0.5)
            .ceil()
            .min(f64::from(br)) as i32;
        let by1 = (p0[1].max(p1[1]).max(p2[1]) + 0.5)
            .ceil()
            .min(f64::from(bb)) as i32;
        let eps = -1e-9;
        for y in by0..by1 {
            let py = f64::from(y) + 0.5;
            for x in bx0..bx1 {
                let px = f64::from(x) + 0.5;
                let l1 = ((px - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (py - p0[1])) / det;
                let l2 = ((p1[0] - p0[0]) * (py - p0[1]) - (px - p0[0]) * (p1[1] - p0[1])) / det;
                let l0 = 1.0 - l1 - l2;
                if l0 < eps || l1 < eps || l2 < eps {
                    continue;
                }
                let o = (y - bt) as usize * w + (x - bl) as usize;
                uv[o] = [
                    l0 * s0[0] + l1 * s1[0] + l2 * s2[0],
                    l0 * s0[1] + l1 * s1[1] + l2 * s2[1],
                ];
                owner[o] = k as u32;
            }
        }
    }
    let (ox, oy) = (f64::from(rect.left) + 0.5, f64::from(rect.top) + 0.5);
    let mut acc = vec![0.0f64; n];
    let mut one = vec![0.0f64; n];
    for (o, (q, &k)) in uv.iter().zip(&owner).enumerate() {
        if q[0].is_nan() {
            continue;
        }
        let (u, v) = (q[0] - ox, q[1] - oy);
        let (j, nx, ny) = jac[k as usize];
        if nx * ny == 1 {
            src.point(interp, u, v, &mut acc);
        } else {
            acc.fill(0.0);
            for sy in 0..ny {
                let tt = (sy as f64 + 0.5) / ny as f64 - 0.5;
                for sx in 0..nx {
                    let s = (sx as f64 + 0.5) / nx as f64 - 0.5;
                    src.point(
                        Interpolation::Bilinear,
                        u + s * j[0] + tt * j[2],
                        v + s * j[1] + tt * j[3],
                        &mut one,
                    );
                    acc.iter_mut().zip(&one).for_each(|(c, v)| *c += v);
                }
            }
            let inv = 1.0 / (nx * ny) as f64;
            acc.iter_mut().for_each(|c| *c *= inv);
        }
        let clamp = |v: f64| if hdr { v } else { v.clamp(0.0, 1.0) };
        match src.alpha {
            Some(a) => {
                let al = acc[a].clamp(0.0, 1.0);
                // Float noise (a vertex a hair off the pixel grid) must not leave
                // invisible colour behind in transparent pixels.
                if al <= 1e-6 {
                    continue;
                }
                for (c, plane) in out.iter_mut().enumerate() {
                    plane[o] = if c == a {
                        al as f32
                    } else {
                        clamp(acc[c] / al) as f32
                    };
                }
            }
            None => {
                for (plane, v) in out.iter_mut().zip(&acc) {
                    plane[o] = clamp(*v) as f32;
                }
            }
        }
    }
    out
}
