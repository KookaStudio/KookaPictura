//! Sample-typed rebasing of a retained store: the native analogues of the
//! 8-bit document-scope plane helpers. A canvas resize/crop blits every plane,
//! `Image > Image Size` resamples them, and orientation remaps them exactly.
//! Every helper maps whole planes, so callers pass the plane dimensions.

use pictura_core::{Sample, Samples};

/// The five exact orientation index remaps, mirroring `pictura_ops`.
#[derive(Clone, Copy)]
pub(super) enum Remap {
    Rot90,
    Rot180,
    Rot270,
    FlipH,
    FlipV,
}

impl Remap {
    fn swaps_axes(self) -> bool {
        matches!(self, Remap::Rot90 | Remap::Rot270)
    }
}

/// Blit every `old_w×old_h` plane of `samples` into `new_w×new_h` at `(dx, dy)`,
/// zero-filling the added area. The native analogue of `extend_channel`.
pub(super) fn extend_samples(
    samples: &Samples,
    old_w: usize,
    old_h: usize,
    new_w: usize,
    new_h: usize,
    dx: i32,
    dy: i32,
) -> Samples {
    let plane = old_w * old_h;
    match samples {
        Samples::U8(v) => Samples::U8(
            each_plane(v, plane, |p| {
                extend_plane(p, old_w, old_h, new_w, new_h, dx, dy)
            })
            .into(),
        ),
        Samples::U16(v) => Samples::U16(
            each_plane(v, plane, |p| {
                extend_plane(p, old_w, old_h, new_w, new_h, dx, dy)
            })
            .into(),
        ),
        Samples::F32(v) => Samples::F32(
            each_plane(v, plane, |p| {
                extend_plane(p, old_w, old_h, new_w, new_h, dx, dy)
            })
            .into(),
        ),
    }
}

fn extend_plane<T: Copy + Default>(
    data: &[T],
    old_w: usize,
    old_h: usize,
    new_w: usize,
    new_h: usize,
    dx: i32,
    dy: i32,
) -> Vec<T> {
    let mut out = vec![T::default(); new_w * new_h];
    let x0 = dx.max(0) as usize;
    let x1 = ((old_w as i32 + dx).min(new_w as i32)).max(0) as usize;
    let y0 = dy.max(0) as usize;
    let y1 = ((old_h as i32 + dy).min(new_h as i32)).max(0) as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let src = (y as i32 - dy) as usize * old_w + (x as i32 - dx) as usize;
            out[y * new_w + x] = data.get(src).copied().unwrap_or_default();
        }
    }
    out
}

/// Resample every `sw×sh` plane of `samples` to `dw×dh`, mirroring
/// `pictura_ops::resize`'s kernels at native precision.
pub(super) fn resize_samples(
    samples: &Samples,
    sw: usize,
    sh: usize,
    dw: usize,
    dh: usize,
    resample: pictura_ops::Resample,
) -> Samples {
    if sw == dw && sh == dh {
        return samples.clone();
    }
    let plane = sw * sh;
    match samples {
        Samples::U8(v) => {
            Samples::U8(each_plane(v, plane, |p| resize_plane(p, sw, sh, dw, dh, resample)).into())
        }
        Samples::U16(v) => {
            Samples::U16(each_plane(v, plane, |p| resize_plane(p, sw, sh, dw, dh, resample)).into())
        }
        Samples::F32(v) => {
            Samples::F32(each_plane(v, plane, |p| resize_plane(p, sw, sh, dw, dh, resample)).into())
        }
    }
}

fn resize_plane<T: Sample>(
    data: &[T],
    sw: usize,
    sh: usize,
    dw: usize,
    dh: usize,
    resample: pictura_ops::Resample,
) -> Vec<T> {
    let xscale = sw as f64 / dw as f64;
    let yscale = sh as f64 / dh as f64;
    let mut out = Vec::with_capacity(dw * dh);
    for y in 0..dh {
        let sy = (y as f64 + 0.5) * yscale - 0.5;
        for x in 0..dw {
            let sx = (x as f64 + 0.5) * xscale - 0.5;
            out.push(sample_plane(data, sw, sh, sx, sy, resample));
        }
    }
    out
}

/// Clamp an out-of-bounds sample axis to `0..n-1`.
fn idx(i: i64, n: usize) -> usize {
    i.clamp(0, n as i64 - 1) as usize
}

/// 2×2 tent weights for one axis at source coordinate `s`.
fn bilevel(s: f64, n: usize) -> ([usize; 2], [f64; 2]) {
    let x0 = s.floor();
    let f = s - x0;
    let x0 = x0 as i64;
    ([idx(x0, n), idx(x0 + 1, n)], [1.0 - f, f])
}

/// Keys cubic kernel `a = -0.75` (Mitchell–Netravali `cubic(0, 0.75)`).
fn cubic(t: f64) -> f64 {
    let a = -0.75;
    let x = t.abs();
    if x <= 1.0 {
        (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
    } else if x < 2.0 {
        a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
    } else {
        0.0
    }
}

/// 4×4 cubic weights for one axis at source coordinate `s`.
fn cubics(s: f64, n: usize) -> ([usize; 4], [f64; 4]) {
    let x0 = s.floor() as i64;
    let at = |k: i64| x0 + k - 1;
    let i: [usize; 4] = std::array::from_fn(|k| idx(at(k as i64), n));
    let w: [f64; 4] = std::array::from_fn(|k| cubic(s - at(k as i64) as f64));
    (i, w)
}

fn sample_plane<T: Sample>(
    plane: &[T],
    w: usize,
    h: usize,
    sx: f64,
    sy: f64,
    resample: pictura_ops::Resample,
) -> T {
    match resample {
        pictura_ops::Resample::Nearest => {
            let x = idx(sx.round() as i64, w);
            let y = idx(sy.round() as i64, h);
            plane[y * w + x]
        }
        pictura_ops::Resample::Bilinear => {
            let (xi, xw) = bilevel(sx, w);
            let (yi, yw) = bilevel(sy, h);
            let (mut acc, mut ws) = (0.0, 0.0);
            for (i, wx) in xi.iter().zip(xw) {
                for (j, wy) in yi.iter().zip(yw) {
                    acc += wx * wy * plane[j * w + i].to_unit();
                    ws += wx * wy;
                }
            }
            T::from_unit(acc / ws)
        }
        pictura_ops::Resample::Bicubic => {
            let (xi, xw) = cubics(sx, w);
            let (yi, yw) = cubics(sy, h);
            let (mut acc, mut ws) = (0.0, 0.0);
            for (i, wx) in xi.iter().zip(xw) {
                for (j, wy) in yi.iter().zip(yw) {
                    acc += wx * wy * plane[j * w + i].to_unit();
                    ws += wx * wy;
                }
            }
            T::from_unit(acc / ws)
        }
    }
}

/// Remap every `w×h` plane of `samples`; 90°/270° swap the plane axes.
pub(super) fn remap_samples(samples: &Samples, w: usize, h: usize, kind: Remap) -> Samples {
    let plane = w * h;
    match samples {
        Samples::U8(v) => Samples::U8(each_plane(v, plane, |p| remap_plane(p, w, h, kind)).into()),
        Samples::U16(v) => {
            Samples::U16(each_plane(v, plane, |p| remap_plane(p, w, h, kind)).into())
        }
        Samples::F32(v) => {
            Samples::F32(each_plane(v, plane, |p| remap_plane(p, w, h, kind)).into())
        }
    }
}

fn remap_plane<T: Copy>(data: &[T], w: usize, h: usize, kind: Remap) -> Vec<T> {
    let (nw, nh) = if kind.swaps_axes() { (h, w) } else { (w, h) };
    let mut out = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = match kind {
                Remap::Rot90 => (y, h - 1 - x),
                Remap::Rot180 => (w - 1 - x, h - 1 - y),
                Remap::Rot270 => (w - 1 - y, x),
                Remap::FlipH => (w - 1 - x, y),
                Remap::FlipV => (x, h - 1 - y),
            };
            out.push(data[sy * w + sx]);
        }
    }
    out
}

/// Apply `f` to every `plane`-sample chunk, concatenating the results. A
/// trailing partial plane is ignored.
fn each_plane<T: Copy, U>(data: &[T], plane: usize, f: impl FnMut(&[T]) -> Vec<U>) -> Vec<U> {
    if plane == 0 {
        return Vec::new();
    }
    data.chunks_exact(plane).flat_map(f).collect()
}
