//! Downsampling for the Compression page (design D8): each target pixel covers
//! a footprint of source pixels; Average takes its mean, Subsampling its centre
//! sample, Bicubic the Keys `a = -0.75` kernel at its centre (the Image Size
//! coefficient). Samples are `u16` so 16-bit output downsamples too.

use super::options::Downsample;

/// Keys cubic, `a = -0.75`.
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

/// Resample interleaved `channels`-sample pixels from `w × h` to `nw × nh`.
/// `max` is the sample ceiling (255 or 65535) Bicubic clamps to.
#[allow(clippy::too_many_arguments)]
pub(crate) fn downsample(
    samples: &[u16],
    channels: usize,
    w: usize,
    h: usize,
    nw: usize,
    nh: usize,
    kind: Downsample,
    max: u16,
) -> Vec<u16> {
    if kind == Downsample::None || (nw == w && nh == h) || nw == 0 || nh == 0 {
        return samples.to_vec();
    }
    let sx = w as f64 / nw as f64;
    let sy = h as f64 / nh as f64;
    let at = |x: usize, y: usize, c: usize| f64::from(samples[(y * w + x) * channels + c]);
    let mut out = Vec::with_capacity(nw * nh * channels);
    for ty in 0..nh {
        let (y0, y1) = span(ty, sy, h);
        let cy = (ty as f64 + 0.5) * sy - 0.5;
        for tx in 0..nw {
            let (x0, x1) = span(tx, sx, w);
            let cx = (tx as f64 + 0.5) * sx - 0.5;
            for c in 0..channels {
                let value = match kind {
                    Downsample::Average => {
                        let mut sum = 0.0;
                        for y in y0..y1 {
                            for x in x0..x1 {
                                sum += at(x, y, c);
                            }
                        }
                        sum / ((y1 - y0) * (x1 - x0)) as f64
                    }
                    Downsample::Subsample => {
                        let x = (cx.round().max(0.0) as usize).min(w - 1);
                        let y = (cy.round().max(0.0) as usize).min(h - 1);
                        at(x, y, c)
                    }
                    _ => {
                        let (bx, by) = (cx.floor() as i64, cy.floor() as i64);
                        let (mut acc, mut weights) = (0.0, 0.0);
                        for j in -1..=2 {
                            let y = (by + j).clamp(0, h as i64 - 1) as usize;
                            let wy = cubic(cy - (by + j) as f64);
                            for i in -1..=2 {
                                let x = (bx + i).clamp(0, w as i64 - 1) as usize;
                                let wt = wy * cubic(cx - (bx + i) as f64);
                                acc += wt * at(x, y, c);
                                weights += wt;
                            }
                        }
                        acc / weights
                    }
                };
                out.push(value.round().clamp(0.0, f64::from(max)) as u16);
            }
        }
    }
    out
}

/// The source rows (or columns) `[start, end)` target index `t` covers.
fn span(t: usize, scale: f64, n: usize) -> (usize, usize) {
    let start = ((t as f64 * scale).floor() as usize).min(n - 1);
    let end = (((t + 1) as f64 * scale).ceil() as usize).clamp(start + 1, n);
    (start, end)
}

/// The image size for `ppi` when downsampling applies (`ppi` above the
/// threshold), else the source size.
pub(crate) fn target_size(
    width: u32,
    height: u32,
    ppi: f64,
    kind: Downsample,
    target_ppi: f64,
    above_ppi: f64,
) -> (u32, u32) {
    if kind == Downsample::None || ppi <= above_ppi || target_ppi >= ppi {
        return (width, height);
    }
    let scale = target_ppi / ppi;
    (
        ((width as f64 * scale).round() as u32).max(1),
        ((height as f64 * scale).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_methods_differ() {
        // A 4 × 1 ramp halved: each target pixel covers two source pixels.
        let src = [0u16, 60, 120, 180];
        let average = downsample(&src, 1, 4, 1, 2, 1, Downsample::Average, 255);
        assert_eq!(average, vec![30, 150], "the mean of each pair");
        let sub = downsample(&src, 1, 4, 1, 2, 1, Downsample::Subsample, 255);
        assert_eq!(sub, vec![60, 180], "a source sample per footprint");
        // Keys weights at the footprint centre: -0.09375, 0.59375, 0.59375,
        // -0.09375 over the clamped neighbours.
        let bicubic = downsample(&src, 1, 4, 1, 2, 1, Downsample::Bicubic, 255);
        assert_eq!(bicubic, vec![24, 156]);
    }

    #[test]
    fn target_size_applies_only_above_the_threshold() {
        assert_eq!(
            target_size(600, 300, 300.0, Downsample::Bicubic, 150.0, 225.0),
            (300, 150)
        );
        assert_eq!(
            target_size(600, 300, 200.0, Downsample::Bicubic, 150.0, 225.0),
            (600, 300)
        );
        assert_eq!(
            target_size(600, 300, 300.0, Downsample::None, 150.0, 225.0),
            (600, 300)
        );
    }
}
