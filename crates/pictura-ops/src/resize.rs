//! `Image > Image Size` resampling (`IMG-001`).
//!
//! The exact edge handling is unpublished, so parity is behavioral only,
//! but its Bicubic coefficient is publicly documented as a Mitchell–Netravali
//! `cubic(0, 0.75)` kernel (Jason Summers, entropymine.com/resamplescope), not
//! Catmull-Rom. All three kernels sample each plane independently and clamp to
//! the edge at borders.

use pictura_core::PixelBuffer;

use crate::{validate, OpsError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resample {
    Nearest,
    Bilinear,
    Bicubic,
}

/// One axis's taps for a sample at a source coordinate: up to four indices,
/// clamped to the edge, and their weights. Image Size's dense and tiled
/// resamplers (#1293) share them, so both weigh the same samples alike.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Taps {
    pub idx: [usize; 4],
    pub weight: [f64; 4],
    pub len: usize,
}

/// The source coordinate destination index `i` samples at `scale`, the
/// source extent over the destination extent.
pub fn source_coord(i: usize, scale: f64) -> f64 {
    (i as f64 + 0.5) * scale - 0.5
}

/// The taps of source coordinate `s` on an axis of `n` samples: one for
/// Nearest, a 2-tap tent for Bilinear, four Keys taps for Bicubic.
pub fn axis_taps(s: f64, n: usize, resample: Resample) -> Taps {
    let idx = |i: i64| i.clamp(0, n as i64 - 1) as usize;
    let mut taps = Taps::default();
    match resample {
        Resample::Nearest => {
            taps.idx[0] = idx(s.round() as i64);
            taps.weight[0] = 1.0;
            taps.len = 1;
        }
        Resample::Bilinear => {
            let x0 = s.floor();
            let f = s - x0;
            let x0 = x0 as i64;
            taps.idx[..2].copy_from_slice(&[idx(x0), idx(x0 + 1)]);
            taps.weight[..2].copy_from_slice(&[1.0 - f, f]);
            taps.len = 2;
        }
        Resample::Bicubic => {
            let x0 = s.floor() as i64;
            for k in 0..4 {
                let at = x0 + k as i64 - 1;
                taps.idx[k] = idx(at);
                taps.weight[k] = cubic(s - at as f64);
            }
            taps.len = 4;
        }
    }
    taps
}

/// Keys cubic kernel `a = -0.75`, i.e. Mitchell–Netravali `cubic(0, 0.75)`, the
/// documents the reference Bicubic coefficient.
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

/// Weigh the samples `at(column, row)` reads at the taps, `x` taps outer and
/// `y` inner, normalized by the weight sum so clamped edge duplicates do not
/// bias the result.
pub fn weigh(xt: &Taps, yt: &Taps, mut at: impl FnMut(usize, usize) -> f64) -> f64 {
    let (mut acc, mut ws) = (0.0, 0.0);
    for (&i, &wx) in xt.idx[..xt.len].iter().zip(&xt.weight[..xt.len]) {
        for (&j, &wy) in yt.idx[..yt.len].iter().zip(&yt.weight[..yt.len]) {
            acc += wx * wy * at(i, j);
            ws += wx * wy;
        }
    }
    acc / ws
}

/// Sample one planar channel at source coordinate (`sx`, `sy`).
fn sample(plane: &[u8], w: u32, h: u32, sx: f64, sy: f64, resample: Resample) -> f64 {
    let stride = w as usize;
    let xt = axis_taps(sx, w as usize, resample);
    let yt = axis_taps(sy, h as usize, resample);
    weigh(&xt, &yt, |i, j| plane[j * stride + i] as f64)
}

pub fn resize(
    buf: &PixelBuffer,
    width: u32,
    height: u32,
    resample: Resample,
) -> Result<PixelBuffer, OpsError> {
    validate(buf)?;
    if width == 0 || height == 0 {
        return Err(OpsError::InvalidParams(
            "width and height must be >= 1".into(),
        ));
    }
    if buf.width == width && buf.height == height {
        return Ok(buf.clone());
    }

    let (sw, sh) = (buf.width, buf.height);
    let planes = buf.pixel_count();
    let channels = buf.channels as usize;
    let xscale = sw as f64 / width as f64;
    let yscale = sh as f64 / height as f64;
    let dst_plane = width as usize * height as usize;

    let mut out = PixelBuffer::new(width, height, buf.channels);
    // A document job (#1278) counts output rows; a cancelled one stops here
    // and drops the result.
    let job = pictura_core::job::current();
    for y in 0..height {
        if let Some(job) = &job {
            if job.cancelled() {
                break;
            }
            job.advance(1);
        }
        let sy = source_coord(y as usize, yscale);
        for x in 0..width {
            let sx = source_coord(x as usize, xscale);
            let d = y as usize * width as usize + x as usize;
            for c in 0..channels {
                let plane = &buf.data[c * planes..(c + 1) * planes];
                out.data[c * dst_plane + d] = sample(plane, sw, sh, sx, sy, resample)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(w: u32, h: u32, ch: u8, f: impl Fn(u32, u32) -> [u8; 4]) -> PixelBuffer {
        let mut b = PixelBuffer::new(w, h, ch);
        let np = b.pixel_count();
        for y in 0..h {
            for x in 0..w {
                let p = f(x, y);
                for (c, &v) in p.iter().take(ch as usize).enumerate() {
                    b.data[c * np + (y * w + x) as usize] = v;
                }
            }
        }
        b
    }

    fn buf3(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> PixelBuffer {
        build(w, h, 3, |x, y| {
            let p = f(x, y);
            [p[0], p[1], p[2], 255]
        })
    }

    fn buf4(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> PixelBuffer {
        build(w, h, 4, f)
    }

    fn at(b: &PixelBuffer, x: u32, y: u32, c: usize) -> u8 {
        b.data[c * b.pixel_count() + (y * b.width + x) as usize]
    }

    #[test]
    fn identity_size_is_clone() {
        let src = buf3(4, 3, |x, y| [x as u8 * 10, y as u8 * 10, 7]);
        let out = resize(&src, 4, 3, Resample::Bicubic).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn nearest_2x2_to_4x4_replicates() {
        let src = buf3(2, 2, |x, y| [(y * 2 + x) as u8 * 50, 10, 20]);
        let out = resize(&src, 4, 4, Resample::Nearest).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                let want = at(&src, x / 2, y / 2, 0);
                assert_eq!(at(&out, x, y, 0), want, "at {x},{y}");
            }
        }
    }

    #[test]
    fn solid_color_all_methods() {
        for m in [Resample::Nearest, Resample::Bilinear, Resample::Bicubic] {
            let out = resize(&buf3(5, 5, |_, _| [10, 20, 30]), 3, 2, m).unwrap();
            for y in 0..2 {
                for x in 0..3 {
                    assert_eq!(at(&out, x, y, 0), 10, "{m:?} R");
                    assert_eq!(at(&out, x, y, 1), 20, "{m:?} G");
                    assert_eq!(at(&out, x, y, 2), 30, "{m:?} B");
                }
            }
        }
    }

    #[test]
    fn bilinear_step_edge_is_intermediate() {
        let src = buf3(2, 1, |x, _| [x as u8 * 200, 0, 0]);
        let out = resize(&src, 4, 1, Resample::Bilinear).unwrap();
        let mid = at(&out, 1, 0, 0);
        assert!(mid > 0 && mid < 200, "expected intermediate, got {mid}");
    }

    #[test]
    fn dimensions_and_channels_preserved() {
        let src = buf3(5, 3, |_, _| [1, 2, 3]);
        let out = resize(&src, 7, 2, Resample::Bicubic).unwrap();
        assert_eq!((out.width, out.height, out.channels), (7, 2, 3));
        assert_eq!(out.data.len(), 7 * 2 * 3);
    }

    #[test]
    fn invalid_dimensions_error() {
        let src = buf3(2, 2, |_, _| [0, 0, 0]);
        assert!(matches!(
            resize(&src, 0, 2, Resample::Nearest),
            Err(OpsError::InvalidParams(_))
        ));
        assert!(matches!(
            resize(&src, 2, 0, Resample::Nearest),
            Err(OpsError::InvalidParams(_))
        ));
        let empty = PixelBuffer::new(0, 0, 3);
        assert!(matches!(
            resize(&empty, 2, 2, Resample::Nearest),
            Err(OpsError::InvalidParams(_))
        ));
    }

    #[test]
    fn alpha_plane_resampled_and_preserved() {
        let src = buf4(4, 1, |x, _| {
            [
                0,
                0,
                0,
                match x {
                    0 => 0,
                    1 => 100,
                    2 => 200,
                    _ => 255,
                },
            ]
        });
        let out = resize(&src, 2, 1, Resample::Bilinear).unwrap();
        assert_eq!(out.channels, 4);
        assert_eq!([at(&out, 0, 0, 3), at(&out, 1, 0, 3)], [50, 228]);
        assert_eq!(at(&out, 0, 0, 0), 0);
    }

    #[test]
    fn downscale_then_upscale_ramp() {
        let src = buf3(8, 1, |x, _| {
            let v = x as u8 * 32;
            [v, v, v]
        });
        let small = resize(&src, 2, 1, Resample::Bilinear).unwrap();
        assert_eq!(small.width, 2);
        let back = resize(&small, 8, 1, Resample::Nearest).unwrap();
        assert_eq!(back.width, 8);
        let v0 = at(&back, 0, 0, 0);
        let v7 = at(&back, 7, 0, 0);
        assert!(v7 > v0, "ramp must stay monotonic: {v0} -> {v7}");
        for x in 0..4 {
            assert_eq!(at(&back, x, 0, 0), v0);
        }
        for x in 4..8 {
            assert_eq!(at(&back, x, 0, 0), v7);
        }
    }

    #[test]
    fn one_by_one_no_panic() {
        let src = buf3(1, 1, |_, _| [7, 8, 9]);
        for m in [Resample::Nearest, Resample::Bilinear, Resample::Bicubic] {
            let out = resize(&src, 3, 3, m).unwrap();
            assert_eq!(out.pixel_count(), 9);
        }
        assert_eq!(resize(&src, 1, 1, Resample::Nearest).unwrap(), src);
    }
}
