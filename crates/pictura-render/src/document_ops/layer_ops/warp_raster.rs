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
//! cell wins.
//!
//! The destination is built one strip of 256 rows at a time into tiled planes
//! the evictor can pack or spill (#1292). A strip's blocks (64 rows by 256
//! columns) are rasterized in parallel, each from only the source window its
//! samples reach, converted and premultiplied for that window alone. The
//! result is byte-identical to rasterizing the whole layer at once: the dense
//! rasterizer is kept as the test oracle (`warp_oracle.rs`).
//!
//! Ported from photocraft's `algo::warp` (ArtCraft Team and the PhotoCraft
//! contributors, MIT OR Apache-2.0, taken under MIT; see `NOTICE.md`), from
//! interleaved tiles to Kooka's planar channels.

use std::ops::Range;

use pictura_core::{Plane, PsdRect, Sample, Samples, TileSample, TILE};
use rayon::prelude::*;

use super::kernel::{self, Interpolation};
use super::transform::integer_bbox;

/// Grid cell size (source px) used to approximate the map.
const CELL: f64 = 4.0;
/// Most cells along one axis (bounds memory and time on huge layers).
const MAX_CELLS: usize = 512;
/// Most subsamples per axis when a warp shrinks the source; also the longest
/// source step per destination pixel the filter follows.
pub(super) const MAX_SS: f64 = 8.0;
/// Probe lines per axis [`mapped_lengths`] follows through the map.
const PROBE: usize = 16;
/// Destination rows per block (and per triangle bin).
pub(super) const BAND: i32 = 64;
/// Destination columns per block.
const COLS: i32 = TILE as i32;
/// Destination coordinates are clamped to this magnitude.
const LIMIT: f64 = (1 << 20) as f64;

/// A source rect cut into triangles and mapped forward: each vertex is
/// `(destination, source)` in document pixels, triangles in depth order (later
/// ones draw over earlier ones), and `dest` the integer box `(left, top, right,
/// bottom)` round every destination vertex.
pub(super) struct Triangles {
    pub(super) verts: Vec<([f64; 2], [f64; 2])>,
    pub(super) tris: Vec<[usize; 3]>,
    pub(super) dest: (i32, i32, i32, i32),
}

/// Triangulate `src` through the forward map `f` (document → document).
/// `None` for an empty rect, a non-finite or out-of-range mapped vertex, or a
/// destination larger than the other transform ops allow ([`integer_bbox`]).
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
    // The coordinate limit alone admits a 2^21-square result; hold the area to
    // the transform ops' ceiling before anything is allocated for it.
    integer_bbox(&[(x0, y0), (x1, y1)])?;
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

/// The bytes one block's source window may hold across all its unit planes:
/// the per-thread window the commit gate charges (`WINDOW_BYTES` in the app's
/// `impl_transform/group.rs`).
#[cfg(not(test))]
const WINDOW_BYTES: usize = 8 << 20;
/// Small in tests, so every test warp also exercises the block split.
#[cfg(test)]
const WINDOW_BYTES: usize = 64 << 10;

/// Slack round a sample position when bounding its taps. It covers the point
/// sampler's snap (1e-7) and the rounding of the subsample offsets.
const TAP_SLACK: f64 = 1e-3;

/// One plane of a warp's source, read a window at a time.
#[derive(Clone, Copy)]
pub(super) enum SrcPlane<'a> {
    U8(&'a Plane<u8>),
    U16(&'a Plane<u16>),
    F32(&'a Plane<f32>),
    /// Full coverage: the alpha a layer mask is warped with.
    Ones,
}

impl<'a> SrcPlane<'a> {
    pub(super) fn of(samples: &'a Samples) -> Self {
        match samples {
            Samples::U8(p) => SrcPlane::U8(p),
            Samples::U16(p) => SrcPlane::U16(p),
            Samples::F32(p) => SrcPlane::F32(p),
        }
    }

    /// Columns `xs` × rows `ys` of a `width`-wide plane as unit `f32`: 8 and
    /// 16-bit through [`Sample`], 32-bit as stored (unclamped, NaN to zero).
    fn read(self, xs: Range<usize>, ys: Range<usize>, width: usize) -> Vec<f32> {
        fn typed<T: TileSample>(
            p: &Plane<T>,
            xs: Range<usize>,
            ys: Range<usize>,
            width: usize,
        ) -> Vec<T> {
            let mut buf = vec![T::default(); xs.len() * ys.len()];
            p.read_rect_into(xs, ys, width, &mut buf);
            buf
        }
        match self {
            SrcPlane::U8(p) => typed(p, xs, ys, width)
                .into_iter()
                .map(|v| v.to_unit() as f32)
                .collect(),
            SrcPlane::U16(p) => typed(p, xs, ys, width)
                .into_iter()
                .map(|v| v.to_unit() as f32)
                .collect(),
            SrcPlane::F32(p) => typed(p, xs, ys, width)
                .into_iter()
                .map(|v| if v.is_nan() { 0.0 } else { v })
                .collect(),
            SrcPlane::Ones => vec![1.0; xs.len() * ys.len()],
        }
    }
}

/// A warp's source: its planes over `rect` (all `rect`-sized), the colour
/// planes premultiplied by `alpha` when it is given.
pub(super) struct Source<'a> {
    pub(super) planes: Vec<SrcPlane<'a>>,
    pub(super) rect: PsdRect,
    pub(super) alpha: Option<usize>,
}

impl Source<'_> {
    /// The `w × h` window at source pixel `(x0, y0)`, inside the source.
    fn window(&self, x0: usize, y0: usize, w: usize, h: usize) -> Planes {
        let sw = self.rect.width() as usize;
        let mut data: Vec<Vec<f32>> = self
            .planes
            .iter()
            .map(|p| p.read(x0..x0 + w, y0..y0 + h, sw))
            .collect();
        if let Some(a) = self.alpha {
            let al = std::mem::take(&mut data[a]);
            for (k, plane) in data.iter_mut().enumerate() {
                if k != a {
                    plane.iter_mut().zip(&al).for_each(|(v, a)| *v *= a);
                }
            }
            data[a] = al;
        }
        Planes {
            data,
            x0: x0 as i64,
            y0: y0 as i64,
            w,
            h,
            sw: sw as i64,
            sh: i64::from(self.rect.height()),
        }
    }
}

/// Unit-domain samples of the `w × h` window at `(x0, y0)` of a `sw × sh`
/// source, premultiplied as [`Source`] says.
struct Planes {
    data: Vec<Vec<f32>>,
    x0: i64,
    y0: i64,
    w: usize,
    h: usize,
    sw: i64,
    sh: i64,
}

impl Planes {
    /// Add `weight ×` the source pixel at `(x, y)` (zero outside the source) to
    /// `acc`. The window holds every pixel a block's samples reach.
    fn tap(&self, x: i64, y: i64, weight: f64, acc: &mut [f64]) {
        if weight == 0.0 || x < 0 || y < 0 || x >= self.sw || y >= self.sh {
            return;
        }
        debug_assert!(
            x >= self.x0
                && y >= self.y0
                && x < self.x0 + self.w as i64
                && y < self.y0 + self.h as i64,
            "tap ({x}, {y}) outside the window"
        );
        let o = (y - self.y0) as usize * self.w + (x - self.x0) as usize;
        for (s, plane) in acc.iter_mut().zip(&self.data) {
            *s += f64::from(plane[o]) * weight;
        }
    }

    /// One sample at source position `(u, v)` (pixel centres at integers) into
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

/// The sample type of one output plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Kind {
    U8,
    U16,
    F32,
}

impl Kind {
    pub(super) fn of(samples: &Samples) -> Kind {
        match samples {
            Samples::U8(_) => Kind::U8,
            Samples::U16(_) => Kind::U16,
            Samples::F32(_) => Kind::F32,
        }
    }

    /// A `w × h` tiled plane of zeros, which holds no tile allocation.
    fn zeros(self, w: usize, h: usize) -> Samples {
        match self {
            Kind::U8 => Samples::U8(Plane::filled(w, h, 1, 0)),
            Kind::U16 => Samples::U16(Plane::filled(w, h, 1, 0)),
            Kind::F32 => Samples::F32(Plane::filled(w, h, 1, 0.0)),
        }
    }

    /// Unit `f32` samples in this type: 8/16-bit through [`Sample`], 32-bit
    /// as they are.
    fn store(self, plane: Vec<f32>) -> Block {
        match self {
            Kind::U8 => Block::U8(
                plane
                    .into_iter()
                    .map(|v| u8::from_unit(f64::from(v)))
                    .collect(),
            ),
            Kind::U16 => Block::U16(
                plane
                    .into_iter()
                    .map(|v| u16::from_unit(f64::from(v)))
                    .collect(),
            ),
            Kind::F32 => Block::F32(plane),
        }
    }
}

/// One block of one output plane, in that plane's sample type.
enum Block {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F32(Vec<f32>),
}

impl Block {
    fn write_into(&self, out: &mut Samples, xs: Range<usize>, ys: Range<usize>, width: usize) {
        match (out, self) {
            (Samples::U8(p), Block::U8(v)) => p.write_rect_from(xs, ys, width, v),
            (Samples::U16(p), Block::U16(v)) => p.write_rect_from(xs, ys, width, v),
            (Samples::F32(p), Block::F32(v)) => p.write_rect_from(xs, ys, width, v),
            _ => unreachable!("an output plane is made from its block's kind"),
        }
    }
}

fn settle(out: &mut Samples) {
    match out {
        Samples::U8(p) => p.settle(),
        Samples::U16(p) => p.settle(),
        Samples::F32(p) => p.settle(),
    }
}

/// Rasterize `src` through `t` into `t.dest`, one tiled plane of each kind in
/// `kinds`, a strip of 256 rows at a time. `post` turns a block's unit planes
/// (one per source plane) into one per output plane: a layer mask's gray is
/// flattened over its default there. Uncovered pixels are zero in every unit
/// plane; with an alpha plane the colour planes come out straight again. `hdr`
/// leaves colour values unclamped (32-bit planes). A cancelled job stops
/// between strips; the caller discards the result.
pub(super) fn warp_tiled(
    src: &Source<'_>,
    t: &Triangles,
    interp: Interpolation,
    hdr: bool,
    kinds: &[Kind],
    post: &(dyn Fn(&mut Vec<Vec<f32>>) + Sync),
) -> Vec<Samples> {
    let (dl, dt, dr, db) = t.dest;
    let dw = (dr - dl) as usize;
    let mut outs: Vec<Samples> = kinds
        .iter()
        .map(|k| k.zeros(dw, (db - dt) as usize))
        .collect();
    let bins = bin_rows(t);
    let job = pictura_core::job::current();
    for s0 in (dt..db).step_by(TILE) {
        if job.as_ref().is_some_and(|job| job.cancelled()) {
            break;
        }
        let s1 = (s0 + TILE as i32).min(db);
        let blocks: Vec<(i32, i32, i32, i32)> = (s0..s1)
            .step_by(BAND as usize)
            .flat_map(|y| {
                (dl..dr)
                    .step_by(COLS as usize)
                    .map(move |x| (x, y, (x + COLS).min(dr), (y + BAND).min(s1)))
            })
            .collect();
        let rendered: Vec<Vec<Block>> = blocks
            .par_iter()
            .map(|&block| {
                let tris: Vec<usize> = bins[((block.1 - dt) / BAND) as usize]
                    .iter()
                    .copied()
                    .filter(|&k| reaches_columns(t, k, block.0, block.2))
                    .collect();
                let mut planes = render_block(src, t, &tris, block, interp, hdr);
                post(&mut planes);
                debug_assert_eq!(planes.len(), kinds.len());
                kinds.iter().zip(planes).map(|(k, p)| k.store(p)).collect()
            })
            .collect();
        for (b, parts) in blocks.iter().zip(&rendered) {
            let xs = (b.0 - dl) as usize..(b.2 - dl) as usize;
            let ys = (b.1 - dt) as usize..(b.3 - dt) as usize;
            for (out, part) in outs.iter_mut().zip(parts) {
                part.write_into(out, xs.clone(), ys.clone(), dw);
            }
        }
        outs.iter_mut().for_each(settle);
        if let Some(job) = &job {
            job.advance((s1 - s0) as u64);
        }
    }
    outs
}

/// Per [`BAND`] of destination rows, the triangles (in depth order) whose
/// rows reach it.
fn bin_rows(t: &Triangles) -> Vec<Vec<usize>> {
    let (_, dt, _, db) = t.dest;
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); ((db - dt) as usize).div_ceil(BAND as usize)];
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
    bins
}

/// Whether triangle `k`'s padded column range, the one [`rasterize`] loops
/// over, meets columns `[left, right)`.
fn reaches_columns(t: &Triangles, k: usize, left: i32, right: i32) -> bool {
    let xs = t.tris[k].map(|v| t.verts[v].0[0]);
    let lo = (xs[0].min(xs[1]).min(xs[2]) - 0.5).floor();
    let hi = (xs[0].max(xs[1]).max(xs[2]) + 0.5).ceil();
    lo < f64::from(right) && hi > f64::from(left)
}

/// A block's rasterization: per pixel (rows `w` apart) its source position
/// (NaN where no triangle covers it) and the triangle it came from; per
/// triangle its source step per destination pixel along x and y
/// (`[dx_u, dx_v, dy_u, dy_v]`, the inverse map's Jacobian, constant over a
/// triangle) and the subsamples along each.
struct Raster {
    uv: Vec<[f64; 2]>,
    owner: Vec<u32>,
    jac: Vec<([f64; 4], usize, usize)>,
    w: usize,
}

/// Rasterize `tris` (indices into `t`, in depth order) over `block`
/// `(left, top, right, bottom)`.
fn rasterize(
    t: &Triangles,
    tris: &[usize],
    block: (i32, i32, i32, i32),
    interp: Interpolation,
) -> Raster {
    let (bl, bt, br, bb) = block;
    let (w, h) = ((br - bl) as usize, (bb - bt) as usize);
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
    Raster { uv, owner, jac, w }
}

/// Pixels `x..x + w` × `y..y + h` of a block.
#[derive(Clone, Copy)]
struct Area {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// One block `(left, top, right, bottom)` of [`warp_tiled`]: one unit plane per
/// source plane.
fn render_block(
    src: &Source<'_>,
    t: &Triangles,
    tris: &[usize],
    block: (i32, i32, i32, i32),
    interp: Interpolation,
    hdr: bool,
) -> Vec<Vec<f32>> {
    let (bl, bt, br, bb) = block;
    let (w, h) = ((br - bl) as usize, (bb - bt) as usize);
    let n = src.planes.len();
    let mut out = vec![vec![0.0f32; w * h]; n];
    if tris.is_empty() {
        return out;
    }
    let raster = rasterize(t, tris, block, interp);
    let limit = (WINDOW_BYTES / (4 * n.max(1))).max(1);
    let all = Area { x: 0, y: 0, w, h };
    fill(src, &raster, all, limit, interp, hdr, &mut out);
    out
}

/// Sample `area` from one source window, splitting it while the window would
/// hold more than `limit` pixels.
fn fill(
    src: &Source<'_>,
    raster: &Raster,
    area: Area,
    limit: usize,
    interp: Interpolation,
    hdr: bool,
    out: &mut [Vec<f32>],
) {
    // No covered pixel reads the source: every sample is zero, as `out` is.
    let Some((x0, y0, x1, y1)) = tap_box(src, raster, area) else {
        return;
    };
    let (ww, wh) = (x1 - x0, y1 - y0);
    if ww * wh > limit && (area.w > 1 || area.h > 1) {
        let (a, b) = if area.w >= area.h {
            let half = area.w / 2;
            (
                Area { w: half, ..area },
                Area {
                    x: area.x + half,
                    w: area.w - half,
                    ..area
                },
            )
        } else {
            let half = area.h / 2;
            (
                Area { h: half, ..area },
                Area {
                    y: area.y + half,
                    h: area.h - half,
                    ..area
                },
            )
        };
        fill(src, raster, a, limit, interp, hdr, out);
        fill(src, raster, b, limit, interp, hdr, out);
        return;
    }
    let window = src.window(x0, y0, ww, wh);
    sample(&window, src, raster, area, interp, hdr, out);
}

/// The source box `[x0, x1) × [y0, y1)` every tap of `area`'s covered pixels
/// reads, clipped to the source; `None` when it is empty.
fn tap_box(src: &Source<'_>, raster: &Raster, area: Area) -> Option<(usize, usize, usize, usize)> {
    let (ox, oy) = (
        f64::from(src.rect.left) + 0.5,
        f64::from(src.rect.top) + 0.5,
    );
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for y in area.y..area.y + area.h {
        for x in area.x..area.x + area.w {
            let o = y * raster.w + x;
            let q = raster.uv[o];
            if q[0].is_nan() {
                continue;
            }
            let (u, v) = (q[0] - ox, q[1] - oy);
            // A point sample reads floor(u) - 1 ..= floor(u) + 2 at most (the
            // bicubic reach); a shrinking pixel's bilinear subsamples lie
            // within half its x and y steps of (u, v).
            let (j, _, _) = raster.jac[raster.owner[o] as usize];
            let ex = 0.5 * (j[0].abs() + j[2].abs()) + TAP_SLACK;
            let ey = 0.5 * (j[1].abs() + j[3].abs()) + TAP_SLACK;
            x0 = x0.min((u - ex).floor() as i64 - 1);
            x1 = x1.max((u + ex).floor() as i64 + 2);
            y0 = y0.min((v - ey).floor() as i64 - 1);
            y1 = y1.max((v + ey).floor() as i64 + 2);
        }
    }
    let (sw, sh) = (i64::from(src.rect.width()), i64::from(src.rect.height()));
    let (x0, y0, x1, y1) = (x0.max(0), y0.max(0), x1.min(sw - 1), y1.min(sh - 1));
    (x0 <= x1 && y0 <= y1).then(|| (x0 as usize, y0 as usize, x1 as usize + 1, y1 as usize + 1))
}

/// Sample `area`'s covered pixels from `window` into `out`.
fn sample(
    window: &Planes,
    src: &Source<'_>,
    raster: &Raster,
    area: Area,
    interp: Interpolation,
    hdr: bool,
    out: &mut [Vec<f32>],
) {
    let n = src.planes.len();
    let (ox, oy) = (
        f64::from(src.rect.left) + 0.5,
        f64::from(src.rect.top) + 0.5,
    );
    let mut acc = vec![0.0f64; n];
    let mut one = vec![0.0f64; n];
    for y in area.y..area.y + area.h {
        for x in area.x..area.x + area.w {
            let o = y * raster.w + x;
            let q = raster.uv[o];
            if q[0].is_nan() {
                continue;
            }
            let (u, v) = (q[0] - ox, q[1] - oy);
            let (j, nx, ny) = raster.jac[raster.owner[o] as usize];
            if nx * ny == 1 {
                window.point(interp, u, v, &mut acc);
            } else {
                acc.fill(0.0);
                for sy in 0..ny {
                    let tt = (sy as f64 + 0.5) / ny as f64 - 0.5;
                    for sx in 0..nx {
                        let s = (sx as f64 + 0.5) / nx as f64 - 0.5;
                        window.point(
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
                    // Float noise (a vertex a hair off the pixel grid) must not
                    // leave invisible colour behind in transparent pixels.
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
    }
}
