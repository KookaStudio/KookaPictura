//! Smart Sharpen: deconvolution of a sharp-core + halo blur model, with a
//! smooth noise floor and Shadow/Highlight tonal damping.
//!
//! The blur the softness came from is modelled as
//! `H = CORE_WEIGHT·core + (1 − CORE_WEIGHT)·halo`: a pixel-scale core (a
//! separable 3-tap `[c, 1−2c, c]`) under a halo shaped by `remove` at `radius`.
//! Inverting `H` lifts the finest detail most and the radius-scale structure
//! moderately, the response measured from CS6/CC screenshots; an unsharp mask
//! lifts every scale above the radius equally and reads as blotchy instead.

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::kernel::{clamp_index, gaussian_blur_planes, gaussian_kernel, sigma_from_radius};
use crate::luma::luma_plane;
use crate::{validate, FilterError, SharpenRemove, TonalFade};

/// ponytail: the model constants are fitted to two Photoshop screenshots of
/// one JPEG (Gaussian r 30.3 / 378 %, Lens r 9.9 / 405 %), not to a
/// controlled sweep; Adobe's algorithm is closed. Mean error to those
/// screenshots drops from 24.1/18.9 levels (the former unsharp-mask model) to
/// 17.5/14.0.
const CORE_WEIGHT: f32 = 0.62;
const CORE_SPREAD: f64 = 0.17;
/// Gaussian σ, disc radius, and motion half-length per pixel of UI radius.
const HALO_SCALE: f64 = 0.8;
/// Detail floor, in levels, at Reduce Noise 100 %.
const NOISE_FLOOR: f64 = 10.0;
/// Van Cittert relaxation: the halo response spans roughly `0.54..=1`, so
/// 1.25 contracts every frequency by at least 3× per iteration.
const RELAX: f32 = 1.25;
const ITERATIONS: usize = 3;
const ACCURATE_ITERATIONS: usize = 8;

/// Smart Sharpen: deconvolve the blur the softness came from, with a smooth
/// noise floor holding back the finest detail.
///
/// `amount` and `reduce_noise` are percentages, `radius` in pixels (0.1..=64),
/// `angle` only read for [`SharpenRemove::MotionBlur`]. `more_accurate` runs
/// the halo solve to convergence instead of three iterations. Color planes
/// only; alpha is never read or written.
#[allow(clippy::too_many_arguments)]
pub fn smart_sharpen(
    buf: &mut PixelBuffer,
    amount: f64,
    radius: f64,
    reduce_noise: f64,
    remove: SharpenRemove,
    angle: f64,
    more_accurate: bool,
    shadow: TonalFade,
    highlight: TonalFade,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !amount.is_finite() || !(1.0..=500.0).contains(&amount) {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen amount {amount} is outside 1..=500"
        )));
    }
    if !radius.is_finite() || radius < 0.1 || radius > 64.0 {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen radius {radius} is outside 0.1..=64"
        )));
    }
    if !reduce_noise.is_finite() || !(0.0..=100.0).contains(&reduce_noise) {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen reduce noise {reduce_noise} is outside 0..=100"
        )));
    }
    if !angle.is_finite() || !(-360.0..=360.0).contains(&angle) {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen angle {angle} is outside -360..=360"
        )));
    }
    validate_fade(shadow, "shadow")?;
    validate_fade(highlight, "highlight")?;

    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = buf.channels as usize;
    let halo = Halo::new(remove, radius, angle);
    let iterations = if more_accurate {
        ACCURATE_ITERATIONS
    } else {
        ITERATIONS
    };
    let fade = tonal_fade_mask(&buf.data, buf.width, buf.height, shadow, highlight);
    let strength = amount / 100.0;
    // Smooth roll-off: detail below the floor fades away instead of being
    // cut, so a flat area cannot break along a hard threshold.
    let floor = reduce_noise / 100.0 * NOISE_FLOOR;
    let floor_sq = floor * floor;
    for c in 0..planes {
        let plane = &mut buf.data[c * n..(c + 1) * n];
        let observed: Vec<f32> = plane.iter().map(|&v| v as f32).collect();
        let mut sharp = invert_halo(&observed, w, h, &halo, iterations);
        invert_core(&mut sharp, w, h);
        for (i, px) in plane.iter_mut().enumerate() {
            let o = observed[i] as f64;
            let d = sharp[i] as f64 - o;
            let gain = if floor > 0.0 {
                let d2 = d * d;
                d2 / (d2 + floor_sq)
            } else {
                1.0
            };
            let keep = fade.as_ref().map_or(1.0, |f| 1.0 - f[i]);
            *px = (o + d * strength * gain * keep).round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

/// The radius-scale part of the blur model, one per `remove`.
enum Halo {
    Gaussian(Vec<f32>),
    /// Disc radius in pixels; rows are spans with fractional end weights so
    /// the kernel grows smoothly with the radius.
    Disc(f64),
    /// Bilinear tap offsets `(dx, dy)` along the motion line, equal weights.
    Line(Vec<(f64, f64)>),
}

impl Halo {
    fn new(remove: SharpenRemove, radius: f64, angle: f64) -> Self {
        let scale = radius * HALO_SCALE;
        match remove {
            SharpenRemove::GaussianBlur => {
                Halo::Gaussian(gaussian_kernel(scale).iter().map(|&k| k as f32).collect())
            }
            SharpenRemove::LensBlur => Halo::Disc(scale),
            SharpenRemove::MotionBlur => {
                // Photoshop angles run counter-clockwise on screen; this
                // buffer's y grows downward.
                let a = angle.to_radians();
                let (ux, uy) = (a.cos(), -a.sin());
                let taps = (2.0 * scale).ceil() as usize + 1;
                let step = 2.0 * scale / (taps - 1) as f64;
                Halo::Line(
                    (0..taps)
                        .map(|j| {
                            let t = -scale + j as f64 * step;
                            (t * ux, t * uy)
                        })
                        .collect(),
                )
            }
        }
    }

    /// `dst = halo ∗ src`, clamp-to-edge.
    fn apply(&self, src: &[f32], dst: &mut [f32], w: usize, h: usize) {
        match self {
            Halo::Gaussian(kernel) => gaussian_plane(src, dst, w, h, kernel),
            Halo::Disc(radius) => disc_plane(src, dst, w, h, *radius),
            Halo::Line(taps) => line_plane(src, dst, w, h, taps),
        }
    }
}

/// Solve `CORE_WEIGHT·x + (1 − CORE_WEIGHT)·halo(x) = observed` by relaxed
/// Van Cittert iteration. Starting from `observed` keeps flat areas exact:
/// the halo preserves a constant, so their residual is zero throughout.
fn invert_halo(observed: &[f32], w: usize, h: usize, halo: &Halo, iterations: usize) -> Vec<f32> {
    let mut x = observed.to_vec();
    let mut blurred = vec![0f32; x.len()];
    for _ in 0..iterations {
        halo.apply(&x, &mut blurred, w, h);
        x.par_iter_mut()
            .zip(observed.par_iter().zip(blurred.par_iter()))
            .for_each(|(x, (&o, &b))| {
                *x += RELAX * (o - CORE_WEIGHT * *x - (1.0 - CORE_WEIGHT) * b);
            });
    }
    x
}

/// Pole `p` and gain of the core's exact inverse. `[c, 1−2c, c]` factors as
/// `(1 − p·z⁻¹)(1 − p·z) / (1 − p)²`, so the inverse is a causal and an
/// anticausal one-pole pass scaled by `(1 − p)²`, `|p| < 1` for `c < 1/4`.
fn core_pole() -> (f32, f32) {
    let q = (1.0 - 2.0 * CORE_SPREAD) / CORE_SPREAD;
    let p = (-q + (q * q - 4.0).sqrt()) / 2.0;
    (p as f32, ((1.0 - p) * (1.0 - p)) as f32)
}

/// Exactly undo the separable core blur. Each pass starts from the steady
/// state of a constant extension, matching clamp-to-edge.
fn invert_core(plane: &mut [f32], w: usize, h: usize) {
    let (p, gain) = core_pole();
    plane.par_chunks_mut(w).for_each(|row| {
        let mut acc = row[0] / (1.0 - p);
        for v in row.iter_mut() {
            acc = *v + p * acc;
            *v = acc;
        }
        let mut acc = row[w - 1] / (1.0 - p);
        for v in row.iter_mut().rev() {
            acc = *v + p * acc;
            *v = acc * gain;
        }
    });
    // Columns run a whole row at a time so the inner loop stays contiguous.
    let mut state: Vec<f32> = plane[..w].iter().map(|&v| v / (1.0 - p)).collect();
    for y in 0..h {
        for (s, v) in state.iter_mut().zip(&mut plane[y * w..(y + 1) * w]) {
            *s = *v + p * *s;
            *v = *s;
        }
    }
    let last = (h - 1) * w;
    let mut state: Vec<f32> = plane[last..].iter().map(|&v| v / (1.0 - p)).collect();
    for y in (0..h).rev() {
        for (s, v) in state.iter_mut().zip(&mut plane[y * w..(y + 1) * w]) {
            *s = *v + p * *s;
            *v = *s * gain;
        }
    }
}

fn gaussian_plane(src: &[f32], dst: &mut [f32], w: usize, h: usize, kernel: &[f32]) {
    let r = kernel.len() / 2;
    let mut tmp = vec![0f32; src.len()];
    tmp.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let line = &src[y * w..(y + 1) * w];
        let ext: Vec<f32> = (0..w + 2 * r)
            .map(|i| line[clamp_index(i as isize - r as isize, w)])
            .collect();
        for (x, out) in row.iter_mut().enumerate() {
            *out = kernel.iter().zip(&ext[x..]).map(|(k, v)| k * v).sum();
        }
    });
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        row.fill(0.0);
        for (k, &weight) in kernel.iter().enumerate() {
            let sy = clamp_index(y as isize + k as isize - r as isize, h);
            for (out, &v) in row.iter_mut().zip(&tmp[sy * w..(sy + 1) * w]) {
                *out += weight * v;
            }
        }
    });
}

/// Disc rows `dy` cover `|dx| ≤ √(r² − dy²)`: whole pixels inside the span,
/// the fractional remainder on the next pixel out. Per-row prefix sums make
/// each row `O(1)`, so a pixel costs `O(r)`.
fn disc_rows(radius: f64) -> Vec<(isize, isize, f64)> {
    let reach = radius.floor() as isize;
    (-reach..=reach)
        .map(|dy| {
            let half = (radius * radius - (dy * dy) as f64).max(0.0).sqrt();
            (dy, half.floor() as isize, half.fract())
        })
        .collect()
}

fn disc_plane(src: &[f32], dst: &mut [f32], w: usize, h: usize, radius: f64) {
    let rows = disc_rows(radius);
    let total: f64 = rows
        .iter()
        .map(|&(_, full, frac)| (2 * full + 1) as f64 + 2.0 * frac)
        .sum();
    let mut prefix = vec![0f64; h * (w + 1)];
    prefix
        .par_chunks_mut(w + 1)
        .enumerate()
        .for_each(|(y, pre)| {
            for x in 0..w {
                pre[x + 1] = pre[x] + src[y * w + x] as f64;
            }
        });
    // Clamp-to-edge sum of `line[a..=b]`.
    let span = |y: usize, a: isize, b: isize| -> f64 {
        let line = &src[y * w..(y + 1) * w];
        let pre = &prefix[y * (w + 1)..(y + 1) * (w + 1)];
        let lo = a.clamp(0, w as isize - 1) as usize;
        let hi = b.clamp(0, w as isize - 1) as usize;
        let left = (-a).max(0) as f64 * line[0] as f64;
        let right = (b - (w as isize - 1)).max(0) as f64 * line[w - 1] as f64;
        left + pre[hi + 1] - pre[lo] + right
    };
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let x = x as isize;
            let mut acc = 0f64;
            for &(dy, full, frac) in &rows {
                let sy = clamp_index(y as isize + dy, h);
                acc += span(sy, x - full, x + full);
                if frac > 0.0 {
                    let line = &src[sy * w..(sy + 1) * w];
                    let l = line[clamp_index(x - full - 1, w)] as f64;
                    let r = line[clamp_index(x + full + 1, w)] as f64;
                    acc += frac * (l + r);
                }
            }
            *out = (acc / total) as f32;
        }
    });
}

fn line_plane(src: &[f32], dst: &mut [f32], w: usize, h: usize, taps: &[(f64, f64)]) {
    let weight = 1.0 / taps.len() as f64;
    dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let acc: f64 = taps
                .iter()
                .map(|&(dx, dy)| bilinear(src, w, h, x as f64 + dx, y as f64 + dy))
                .sum();
            *out = (acc * weight) as f32;
        }
    });
}

fn bilinear(plane: &[f32], w: usize, h: usize, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (xi, yi) = (x0 as isize, y0 as isize);
    let (xa, xb) = (clamp_index(xi, w), clamp_index(xi + 1, w));
    let (ya, yb) = (clamp_index(yi, h), clamp_index(yi + 1, h));
    let at = |xx: usize, yy: usize| plane[yy * w + xx] as f64;
    (at(xa, ya) * (1.0 - fx) + at(xb, ya) * fx) * (1.0 - fy)
        + (at(xa, yb) * (1.0 - fx) + at(xb, yb) * fx) * fy
}

fn validate_fade(fade: TonalFade, which: &str) -> Result<(), FilterError> {
    if fade.amount > 100 {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen {which} amount {} is outside 0..=100",
            fade.amount
        )));
    }
    if fade.width > 100 {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen {which} width {} is outside 0..=100",
            fade.width
        )));
    }
    if !(1..=100).contains(&fade.radius) {
        return Err(FilterError::InvalidParams(format!(
            "smart sharpen {which} radius {} is outside 1..=100",
            fade.radius
        )));
    }
    Ok(())
}

/// Per-pixel tonal fade amount in `0..=1`, or `None` when both tabs are
/// inactive. The sharpening delta is scaled by `1 - fade`.
fn tonal_fade_mask(
    data: &[u8],
    width: u32,
    height: u32,
    shadow: TonalFade,
    highlight: TonalFade,
) -> Option<Vec<f64>> {
    let shadow_active = shadow.amount > 0 && shadow.width > 0;
    let highlight_active = highlight.amount > 0 && highlight.width > 0;
    if !shadow_active && !highlight_active {
        return None;
    }
    let n = width as usize * height as usize;
    let shadow_l = shadow_active.then(|| blurred_luma(data, width, height, shadow.radius));
    let highlight_l = highlight_active.then(|| blurred_luma(data, width, height, highlight.radius));
    let mut fade = vec![0.0f64; n];
    for (i, f) in fade.iter_mut().enumerate() {
        let mut value = 0.0f64;
        if let Some(l) = &shadow_l {
            let mask = 1.0 - smoothstep(0.0, 0.5 * shadow.width as f64 / 100.0, l[i]);
            value = value.max(shadow.amount as f64 / 100.0 * mask);
        }
        if let Some(l) = &highlight_l {
            let mask = smoothstep(1.0 - 0.5 * highlight.width as f64 / 100.0, 1.0, l[i]);
            value = value.max(highlight.amount as f64 / 100.0 * mask);
        }
        *f = value;
    }
    Some(fade)
}

/// Luminance of the original in `0..=1`, Gaussian-blurred with the shared
/// radius convention.
fn blurred_luma(data: &[u8], width: u32, height: u32, radius: u32) -> Vec<f64> {
    let n = width as usize * height as usize;
    let luma = luma_plane(data, n);
    let bytes: Vec<u8> = luma
        .iter()
        .map(|&v| v.round().clamp(0.0, 255.0) as u8)
        .collect();
    let mut mask = PixelBuffer {
        width,
        height,
        channels: 1,
        data: bytes.into(),
    };
    gaussian_blur_planes(&mut mask, sigma_from_radius(radius as f64));
    mask.data.iter().map(|&b| b as f64 / 255.0).collect()
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge0 == edge1 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharpen::tests::{gray_row, planar, plane_range, total_deviation};
    use crate::sharpen::unsharp_mask;

    fn removes() -> [(SharpenRemove, f64); 3] {
        [
            (SharpenRemove::GaussianBlur, 0.0),
            (SharpenRemove::LensBlur, 0.0),
            (SharpenRemove::MotionBlur, 30.0),
        ]
    }

    fn sharpen_with(
        base: &PixelBuffer,
        amount: f64,
        radius: f64,
        remove: SharpenRemove,
        angle: f64,
        more_accurate: bool,
    ) -> PixelBuffer {
        let mut out = base.clone();
        smart_sharpen(
            &mut out,
            amount,
            radius,
            0.0,
            remove,
            angle,
            more_accurate,
            TonalFade::default(),
            TonalFade::default(),
        )
        .unwrap();
        out
    }

    fn faded(base: &PixelBuffer, shadow: TonalFade, highlight: TonalFade) -> PixelBuffer {
        let mut out = base.clone();
        smart_sharpen(
            &mut out,
            200.0,
            3.0,
            0.0,
            SharpenRemove::GaussianBlur,
            0.0,
            false,
            shadow,
            highlight,
        )
        .unwrap();
        out
    }

    fn noisy_row() -> PixelBuffer {
        gray_row(&[
            16, 166, 37, 161, 234, 239, 6, 82, 72, 215, 225, 144, 10, 77, 243, 57,
        ])
    }

    #[test]
    fn smart_sharpen_increases_edge_contrast() {
        let base = gray_row(&[100, 100, 100, 100, 140, 140, 140, 140]);
        for (remove, angle) in removes() {
            let out = sharpen_with(&base, 200.0, 3.0, remove, angle, false);
            assert!(plane_range(&out) > plane_range(&base), "{remove:?}");
        }
    }

    #[test]
    fn smart_sharpen_remove_changes_the_result() {
        let base = gray_row(&[100, 100, 100, 100, 140, 140, 140, 140]);
        let g = sharpen_with(&base, 200.0, 3.0, SharpenRemove::GaussianBlur, 0.0, false);
        let l = sharpen_with(&base, 200.0, 3.0, SharpenRemove::LensBlur, 0.0, false);
        let m = sharpen_with(&base, 200.0, 3.0, SharpenRemove::MotionBlur, 30.0, false);
        assert_ne!(g.data, l.data, "lens differs from gaussian");
        assert_ne!(g.data, m.data, "motion differs from gaussian");
    }

    #[test]
    fn deconvolution_lifts_fine_detail_beyond_unsharp_mask() {
        // Inverting the pixel-scale core is what sets Smart Sharpen apart:
        // a one-pixel alternation gains far more than under an unsharp mask
        // at the same settings.
        let base = gray_row(&[110, 130, 110, 130, 110, 130, 110, 130, 110, 130, 110, 130]);
        let smart = sharpen_with(&base, 100.0, 1.0, SharpenRemove::GaussianBlur, 0.0, false);
        let mut usm = base.clone();
        unsharp_mask(&mut usm, 100.0, 1.0, 0).unwrap();
        assert!(
            plane_range(&smart) > 2 * plane_range(&usm),
            "smart {} vs unsharp {}",
            plane_range(&smart),
            plane_range(&usm)
        );
    }

    #[test]
    fn radius_widens_the_halo() {
        let mut values = vec![100u8; 32];
        values[16..].fill(140);
        let base = gray_row(&values);
        let reach = |radius: f64| {
            let out = sharpen_with(
                &base,
                200.0,
                radius,
                SharpenRemove::GaussianBlur,
                0.0,
                false,
            );
            (0..32).filter(|&i| out.data[i] != base.data[i]).count()
        };
        assert!(reach(6.0) > reach(1.0), "{} !> {}", reach(6.0), reach(1.0));
    }

    #[test]
    fn more_accurate_differs_and_is_deterministic() {
        // A low amount keeps this busy signal off the clip rails, where the
        // two estimates would saturate identically.
        let base = noisy_row();
        for (remove, angle) in removes() {
            let coarse = sharpen_with(&base, 20.0, 3.0, remove, angle, false);
            let accurate = sharpen_with(&base, 20.0, 3.0, remove, angle, true);
            assert_ne!(
                coarse.data, accurate.data,
                "{remove:?}: More Accurate must change the estimate"
            );
            let again = sharpen_with(&base, 20.0, 3.0, remove, angle, true);
            assert_eq!(accurate.data, again.data, "{remove:?}: deterministic");
        }
    }

    #[test]
    fn core_inverse_undoes_the_core_blur() {
        let w = 40;
        let mut signal = vec![50f32; w];
        for (i, v) in signal.iter_mut().enumerate().skip(8).take(24) {
            *v = 50.0 + ((i * 37) % 23) as f32 * 7.0;
        }
        let c = CORE_SPREAD as f32;
        let mut blurred: Vec<f32> = (0..w)
            .map(|x| {
                let at = |i: isize| signal[clamp_index(i, w)];
                c * at(x as isize - 1) + (1.0 - 2.0 * c) * at(x as isize) + c * at(x as isize + 1)
            })
            .collect();
        invert_core(&mut blurred, w, 1);
        for (got, want) in blurred.iter().zip(&signal) {
            assert!((got - want).abs() < 1e-3, "{got} vs {want}");
        }
    }

    #[test]
    fn halo_solve_converges() {
        let (w, h) = (24, 24);
        let observed: Vec<f32> = (0..w * h)
            .map(|i| {
                if (i % w) < 12 && (i / w) > 6 {
                    40.0
                } else {
                    200.0
                }
            })
            .collect();
        for (remove, angle) in removes() {
            let halo = Halo::new(remove, 4.0, angle);
            let x = invert_halo(&observed, w, h, &halo, ACCURATE_ITERATIONS);
            let mut blurred = vec![0f32; x.len()];
            halo.apply(&x, &mut blurred, w, h);
            let worst = (0..x.len())
                .map(|i| {
                    let model = CORE_WEIGHT * x[i] + (1.0 - CORE_WEIGHT) * blurred[i];
                    (model - observed[i]).abs()
                })
                .fold(0f32, f32::max);
            assert!(worst < 0.5, "{remove:?}: residual {worst}");
        }
    }

    #[test]
    fn disc_matches_naive_fractional_span_kernel() {
        let (w, h, radius) = (11usize, 11usize, 2.6f64);
        let mut src = vec![0f32; w * h];
        src[5 * w + 5] = 1000.0;
        let mut dst = vec![0f32; w * h];
        disc_plane(&src, &mut dst, w, h, radius);
        let mut total = 0f64;
        let mut want = vec![0f64; w * h];
        for dy in -2isize..=2 {
            let half = (radius * radius - (dy * dy) as f64).sqrt();
            for dx in -4isize..=4 {
                let d = dx.unsigned_abs() as f64;
                let weight = if d <= half.floor() {
                    1.0
                } else if d == half.floor() + 1.0 {
                    half.fract()
                } else {
                    0.0
                };
                want[(5 + dy) as usize * w + (5 + dx) as usize] = weight;
                total += weight;
            }
        }
        for (got, want) in dst.iter().zip(&want) {
            assert!((*got as f64 - 1000.0 * want / total).abs() < 1e-3);
        }
    }

    #[test]
    fn motion_follows_the_photoshop_angle() {
        // +45° runs lower-left to upper-right: the deconvolution acts along
        // that diagonal, not the other one.
        let mut data = vec![128u8; 3 * 81];
        for c in 0..3 {
            data[c * 81 + 4 * 9 + 4] = 200;
        }
        let base = PixelBuffer {
            width: 9,
            height: 9,
            channels: 3,
            data: data.into(),
        };
        let out = sharpen_with(&base, 100.0, 2.0, SharpenRemove::MotionBlur, 45.0, false);
        let dev = |x: usize, y: usize| (out.data[y * 9 + x] as i32 - 128).abs();
        assert!(dev(6, 2) > dev(6, 6), "{} !> {}", dev(6, 2), dev(6, 6));
        assert!(dev(2, 6) > dev(2, 2), "{} !> {}", dev(2, 6), dev(2, 2));
    }

    #[test]
    fn smart_sharpen_reduce_noise_holds_back_low_contrast() {
        let base = gray_row(&[100, 100, 100, 100, 104, 104, 104, 104]);
        let run = |noise: f64| {
            let mut out = base.clone();
            smart_sharpen(
                &mut out,
                200.0,
                2.0,
                noise,
                SharpenRemove::GaussianBlur,
                0.0,
                false,
                TonalFade::default(),
                TonalFade::default(),
            )
            .unwrap();
            out
        };
        assert!(total_deviation(&base, &run(100.0)) < total_deviation(&base, &run(0.0)));
    }

    #[test]
    fn smart_sharpen_rejects_invalid_params() {
        let base = gray_row(&[10, 20, 30, 40]);
        for (amount, radius, noise, angle) in [
            (-1.0, 2.0, 0.0, 0.0),
            (0.0, 2.0, 0.0, 0.0),
            (0.5, 2.0, 0.0, 0.0),
            (600.0, 2.0, 0.0, 0.0),
            (100.0, 0.0, 0.0, 0.0),
            (100.0, 0.05, 0.0, 0.0),
            (100.0, 65.0, 0.0, 0.0),
            (100.0, 2.0, 101.0, 0.0),
            (100.0, 2.0, 0.0, 500.0),
            (100.0, 2.0, 0.0, f64::NAN),
        ] {
            let mut out = base.clone();
            assert!(smart_sharpen(
                &mut out,
                amount,
                radius,
                noise,
                SharpenRemove::GaussianBlur,
                angle,
                false,
                TonalFade::default(),
                TonalFade::default(),
            )
            .is_err());
            assert_eq!(out, base, "rejected parameters must not modify the buffer");
        }
    }

    #[test]
    fn tonal_fade_rejects_invalid_params() {
        let base = gray_row(&[10, 20, 30, 40]);
        let good = TonalFade {
            amount: 100,
            width: 100,
            radius: 1,
        };
        let bad_fades = [
            TonalFade {
                amount: 101,
                width: 50,
                radius: 1,
            },
            TonalFade {
                amount: 50,
                width: 101,
                radius: 1,
            },
            TonalFade {
                amount: 50,
                width: 50,
                radius: 0,
            },
            TonalFade {
                amount: 50,
                width: 50,
                radius: 101,
            },
        ];
        for bad in bad_fades {
            for (shadow, highlight) in [(bad, good), (good, bad)] {
                let mut out = base.clone();
                assert!(smart_sharpen(
                    &mut out,
                    100.0,
                    2.0,
                    0.0,
                    SharpenRemove::GaussianBlur,
                    0.0,
                    false,
                    shadow,
                    highlight,
                )
                .is_err());
                assert_eq!(out, base, "rejected tonal fade must not modify the buffer");
            }
        }
    }

    fn side_deviation(base: &PixelBuffer, out: &PixelBuffer, range: std::ops::Range<usize>) -> i64 {
        range
            .map(|i| (base.data[i] as i64 - out.data[i] as i64).abs())
            .sum()
    }

    const FULL_FADE: TonalFade = TonalFade {
        amount: 100,
        width: 100,
        radius: 1,
    };

    #[test]
    fn shadow_fade_damps_the_dark_side() {
        let base = gray_row(&[10, 10, 10, 10, 60, 60, 60, 60]);
        let open = faded(&base, TonalFade::default(), TonalFade::default());
        let damped = faded(&base, FULL_FADE, TonalFade::default());
        let dark_open = side_deviation(&base, &open, 0..4);
        let dark_damped = side_deviation(&base, &damped, 0..4);
        assert!(dark_open > 0, "baseline sharpen must change the dark side");
        assert!(dark_damped < dark_open, "{dark_damped} !< {dark_open}");
    }

    #[test]
    fn highlight_fade_damps_the_bright_side() {
        let base = gray_row(&[200, 200, 200, 200, 250, 250, 250, 250]);
        let open = faded(&base, TonalFade::default(), TonalFade::default());
        let damped = faded(&base, TonalFade::default(), FULL_FADE);
        let bright_open = side_deviation(&base, &open, 4..8);
        let bright_damped = side_deviation(&base, &damped, 4..8);
        assert!(
            bright_open > 0,
            "baseline sharpen must change the bright side"
        );
        assert!(
            bright_damped < bright_open,
            "{bright_damped} !< {bright_open}"
        );
    }

    #[test]
    fn zero_tonal_amount_is_transparent() {
        let base = noisy_row();
        let zero = TonalFade {
            amount: 0,
            width: 100,
            radius: 50,
        };
        let default = faded(&base, TonalFade::default(), TonalFade::default());
        assert_eq!(default.data, faded(&base, zero, zero).data);
    }

    #[test]
    fn smart_sharpen_uniform_is_bit_identical() {
        let base = gray_row(&[123; 16]);
        for (remove, angle) in removes() {
            for more_accurate in [false, true] {
                let out = sharpen_with(&base, 500.0, 64.0, remove, angle, more_accurate);
                assert_eq!(out.data, base.data, "{remove:?} accurate={more_accurate}");
            }
        }
    }

    #[test]
    fn smart_sharpen_filters_alpha_and_is_deterministic() {
        let r = vec![10, 60, 120, 200, 250, 30, 80, 90];
        let g = vec![20, 70, 130, 210, 240, 40, 90, 100];
        let b = vec![30, 80, 140, 220, 230, 50, 100, 110];
        let a = vec![5, 95, 128, 170, 200, 255, 60, 70];
        let base = planar(8, 1, 4, &[r, g, b, a.clone()]);
        let n = base.pixel_count();
        let x = sharpen_with(&base, 200.0, 3.0, SharpenRemove::MotionBlur, 30.0, true);
        let y = sharpen_with(&base, 200.0, 3.0, SharpenRemove::MotionBlur, 30.0, true);
        assert_eq!(x.data, y.data, "no seed, so re-apply is byte-identical");
        assert_ne!(
            &x.data[3 * n..4 * n],
            &a[..],
            "alpha runs through the kernel"
        );
    }
}
