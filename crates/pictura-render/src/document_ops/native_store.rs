//! Sample-typed rebasing of a retained store: the native analogues of the
//! 8-bit document-scope plane helpers. A canvas resize/crop blits every plane,
//! `Image > Image Size` resamples them, and orientation remaps them exactly.
//! Every helper maps whole planes, so callers pass the plane dimensions.

#[cfg(test)]
use pictura_core::Sample;
use pictura_core::Samples;

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
    pub(super) fn swaps_axes(self) -> bool {
        matches!(self, Remap::Rot90 | Remap::Rot270)
    }

    /// The source sample of destination `(x, y)` in a `w×h` source plane.
    pub(super) fn source(self, x: usize, y: usize, w: usize, h: usize) -> (usize, usize) {
        match self {
            Remap::Rot90 => (y, h - 1 - x),
            Remap::Rot180 => (w - 1 - x, h - 1 - y),
            Remap::Rot270 => (w - 1 - y, x),
            Remap::FlipH => (w - 1 - x, y),
            Remap::FlipV => (x, h - 1 - y),
        }
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
/// `pictura_ops::resize`'s kernels at native precision: the dense resampler,
/// kept as `resize_tiled`'s oracle for retained stores.
#[cfg(test)]
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
        Samples::U8(v) => Samples::U8(
            each_plane(v, plane, |p| resize_plane(p, (sw, sh), (dw, dh), resample, u8::to_unit))
                .into(),
        ),
        Samples::U16(v) => Samples::U16(
            each_plane(v, plane, |p| resize_plane(p, (sw, sh), (dw, dh), resample, u16::to_unit))
                .into(),
        ),
        Samples::F32(v) => Samples::F32(
            each_plane(v, plane, |p| {
                resize_plane(p, (sw, sh), (dw, dh), resample, |v| {
                    if v.is_finite() {
                        f64::from(v)
                    } else {
                        0.0
                    }
                })
            })
            .into(),
        ),
    }
}

#[cfg(test)]
fn resize_plane<T: Sample>(
    data: &[T],
    (sw, sh): (usize, usize),
    (dw, dh): (usize, usize),
    resample: pictura_ops::Resample,
    value: impl Fn(T) -> f64,
) -> Vec<T> {
    let xscale = sw as f64 / dw as f64;
    let yscale = sh as f64 / dh as f64;
    let mut out = Vec::with_capacity(dw * dh);
    for y in 0..dh {
        let sy = (y as f64 + 0.5) * yscale - 0.5;
        let yt = dense_taps(sy, sh, dh, resample);
        for x in 0..dw {
            let sx = (x as f64 + 0.5) * xscale - 0.5;
            let xt = dense_taps(sx, sw, dw, resample);
            let (mut sum, mut weight) = (0.0, 0.0);
            for &(i, wx) in xt.iter().flatten() {
                for &(j, wy) in yt.iter().flatten() {
                    sum += wx * wy * value(data[j * sw + i]);
                    weight += wx * wy;
                }
            }
            out.push(T::from_unit(sum / weight));
        }
    }
    out
}

#[cfg(test)]
fn dense_taps(
    at: f64,
    n: usize,
    dest: usize,
    resample: pictura_ops::Resample,
) -> [Option<(usize, f64)>; 4] {
    use pictura_ops::Resample;
    let index = |i: i64| i.clamp(0, n as i64 - 1) as usize;
    let base = at.floor() as i64;
    let fraction = at - base as f64;
    match resample {
        Resample::Nearest => [Some((index(at.round() as i64), 1.0)), None, None, None],
        Resample::Bilinear => [
            Some((index(base), 1.0 - fraction)),
            Some((index(base + 1), fraction)),
            None,
            None,
        ],
        cubic_method => {
            let a = match cubic_method {
                Resample::BicubicSmoother => -0.5,
                Resample::BicubicSharper => -1.0,
                Resample::Automatic if dest > n => -0.5,
                Resample::Automatic if dest < n => -1.0,
                _ => -0.75,
            };
            std::array::from_fn(|k| {
                let i = base + k as i64 - 1;
                let x = (at - i as f64).abs();
                let weight = if x <= 1.0 {
                    (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
                } else if x < 2.0 {
                    a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
                } else {
                    0.0
                };
                Some((index(i), weight))
            })
        }
    }
}

/// Remap every `w×h` plane of `samples`; 90°/270° swap the plane axes: the
/// dense remap, kept as `geometry_tiled::orient`'s oracle for retained stores.
#[cfg(test)]
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

#[cfg(test)]
fn remap_plane<T: Copy>(data: &[T], w: usize, h: usize, kind: Remap) -> Vec<T> {
    let (nw, nh) = if kind.swaps_axes() { (h, w) } else { (w, h) };
    let mut out = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = kind.source(x, y, w, h);
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
