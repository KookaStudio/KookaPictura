//! Mesh-warp resampling: planes pushed through any forward map, such as a
//! [`pictura_core::warp::Warp`].
//!
//! The source rect is subdivided into a fine grid whose vertices are mapped
//! forward; each cell's two triangles are rasterized in destination space with
//! the source position interpolated barycentrically, then the source is sampled
//! there (premultiplied by the alpha plane, with the session's kernel). Cells
//! are a few pixels across in both the source and the destination (a thin layer
//! stretched far gets as many cells as its warped size needs), so the
//! piecewise-affine inverse is within a small fraction of a pixel of the true
//! one; affine maps (the identity included) are reproduced exactly. Where the
//! warp shrinks the source, each pixel instead averages a grid of bilinear
//! subsamples over its source footprint (about one per source pixel), so thin
//! lines do not alias. Where the warp folds over itself the later (lower-right)
//! cell wins. Work is split into destination row bands, in parallel.
//!
//! Ported from photocraft's `algo::warp` (ArtCraft Team and the PhotoCraft
//! contributors, MIT OR Apache-2.0, taken under MIT; see `NOTICE.md`), from
//! interleaved tiles to Kooka's planar channels.

use pictura_core::PsdRect;
use rayon::prelude::*;

use super::kernel::{self, Interpolation};

/// Grid cell size (source px) used to approximate the map.
const CELL: f64 = 4.0;
/// Most cells along one axis (bounds memory and time on huge layers).
const MAX_CELLS: usize = 512;
/// Most subsamples per axis when a warp shrinks the source; also the longest
/// source step per destination pixel the filter follows.
const MAX_SS: f64 = 8.0;
/// Probe lines per axis [`mapped_lengths`] follows through the map.
const PROBE: usize = 16;
/// Destination rows per parallel work item.
const BAND: i32 = 64;
/// Destination coordinates are clamped to this magnitude.
const LIMIT: f64 = (1 << 20) as f64;

/// A source rect cut into triangles and mapped forward: each vertex is
/// `(destination, source)` in document pixels, triangles in depth order (later
/// ones draw over earlier ones), and `dest` the integer box `(left, top, right,
/// bottom)` round every destination vertex.
pub(super) struct Triangles {
    verts: Vec<([f64; 2], [f64; 2])>,
    tris: Vec<[usize; 3]>,
    pub(super) dest: (i32, i32, i32, i32),
}

/// Triangulate `src` through the forward map `f` (document → document).
/// `None` for an empty rect or a non-finite or out-of-range mapped vertex.
pub(super) fn triangulate(
    src: PsdRect,
    f: &(dyn Fn(f64, f64) -> (f64, f64) + Sync),
) -> Option<Triangles> {
    let (sw, sh) = (f64::from(src.width()), f64::from(src.height()));
    if sw <= 0.0 || sh <= 0.0 {
        return None;
    }
    // Cells are a few pixels across on both sides of the map: a thin source
    // stretched far (a 3 px column scaled into a wide band, then bent) needs
    // many cells, or it comes out straight.
    let (lu, lv) = mapped_lengths(src, f);
    let cells = |s: f64, d: f64| ((s.max(d) / CELL).ceil() as usize).clamp(1, MAX_CELLS);
    let (nx, ny) = (cells(sw, lu), cells(sh, lv));
    let mut verts = Vec::with_capacity((nx + 1) * (ny + 1));
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for j in 0..=ny {
        for i in 0..=nx {
            let sx = f64::from(src.left) + sw * i as f64 / nx as f64;
            let sy = f64::from(src.top) + sh * j as f64 / ny as f64;
            let (dx, dy) = f(sx, sy);
            if !dx.is_finite() || !dy.is_finite() || dx.abs() > LIMIT || dy.abs() > LIMIT {
                return None;
            }
            x0 = x0.min(dx);
            y0 = y0.min(dy);
            x1 = x1.max(dx);
            y1 = y1.max(dy);
            verts.push(([dx, dy], [sx, sy]));
        }
    }
    let w1 = nx + 1;
    let mut tris = Vec::with_capacity(nx * ny * 2);
    for j in 0..ny {
        for i in 0..nx {
            let a = j * w1 + i;
            tris.push([a, a + 1, a + w1 + 1]);
            tris.push([a, a + w1 + 1, a + w1]);
        }
    }
    // Float noise in a fitted mesh (a corner at 17.000000001) must not grow the
    // box by a pixel.
    let snap = |v: f64| {
        if (v - v.round()).abs() < 1e-6 {
            v.round()
        } else {
            v
        }
    };
    let dest = (
        snap(x0).floor() as i32,
        snap(y0).floor() as i32,
        snap(x1).ceil() as i32,
        snap(y1).ceil() as i32,
    );
    (dest.2 > dest.0 && dest.3 > dest.1).then_some(Triangles { verts, tris, dest })
}

/// The longest destination length of a source row and of a source column under
/// `f`, measured along polylines through a [`PROBE`]-step grid. Non-finite steps
/// count as zero.
fn mapped_lengths(src: PsdRect, f: &(dyn Fn(f64, f64) -> (f64, f64) + Sync)) -> (f64, f64) {
    let (x0, y0) = (f64::from(src.left), f64::from(src.top));
    let (sw, sh) = (f64::from(src.width()), f64::from(src.height()));
    let n = PROBE + 1;
    let mut pts = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let (dx, dy) = f(
                x0 + sw * i as f64 / PROBE as f64,
                y0 + sh * j as f64 / PROBE as f64,
            );
            pts.push([dx, dy]);
        }
    }
    let step = |a: [f64; 2], b: [f64; 2]| {
        let d = (b[0] - a[0]).hypot(b[1] - a[1]);
        if d.is_finite() {
            d
        } else {
            0.0
        }
    };
    let (mut lu, mut lv) = (0.0f64, 0.0f64);
    for k in 0..n {
        let (mut row, mut col) = (0.0, 0.0);
        for m in 1..n {
            row += step(pts[k * n + m - 1], pts[k * n + m]);
            col += step(pts[(m - 1) * n + k], pts[m * n + k]);
        }
        lu = lu.max(row);
        lv = lv.max(col);
    }
    (lu, lv)
}

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
pub(super) fn warp_planes(
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
    let rendered: Vec<Vec<Vec<f32>>> = bands
        .par_iter()
        .zip(bins.par_iter())
        .map(|(&(y0, y1), tris)| render_band(src, rect, t, tris, (dl, y0, dr, y1), interp, hdr))
        .collect();
    let mut out: Vec<Vec<f32>> = vec![Vec::with_capacity(dw * (db - dt) as usize); n];
    for band in rendered {
        for (plane, part) in out.iter_mut().zip(band) {
            plane.extend(part);
        }
    }
    out
}

/// One destination band `(left, top, right, bottom)` of [`warp_planes`].
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
