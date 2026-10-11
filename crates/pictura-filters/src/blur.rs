//! Blur family: Gaussian, Box, Motion, Radial, Average, Blur/Blur More.
//! Surface Blur runs on the photorust engine.
//!
//! Planar 8-bit buffers (channels 3 or 4); only the color planes are touched,
//! alpha is never modified. Borders clamp-to-edge per `FILT-010`.

use pictura_core::PixelBuffer;

use crate::kernel::{clamp_index, gaussian_blur_planes, sigma_from_radius};
use crate::{validate, FilterError, Quality, RadialMethod};
use rayon::prelude::*;

/// Blur's and Blur More's fixed Gaussian radii: σ 0.7 and σ 2.0 under the
/// 3σ radius convention, photorust's CS6-tuned strengths (#222).
pub const BLUR_RADIUS: f64 = 2.1;
pub const BLUR_MORE_RADIUS: f64 = 6.0;

pub fn gaussian(buf: &mut PixelBuffer, radius: f64) -> Result<(), FilterError> {
    validate(buf)?;
    if !radius.is_finite() || radius < 0.0 {
        return Err(FilterError::InvalidParams(format!(
            "gaussian radius {radius} must be finite and non-negative"
        )));
    }
    if radius == 0.0 {
        return Ok(());
    }
    gaussian_blur_planes(buf, sigma_from_radius(radius));
    Ok(())
}

pub fn r#box(buf: &mut PixelBuffer, radius: u32) -> Result<(), FilterError> {
    validate(buf)?;
    if radius == 0 {
        return Ok(());
    }
    box_blur_planes(buf, radius);
    Ok(())
}

pub fn motion(buf: &mut PixelBuffer, angle_deg: f64, distance: u32) -> Result<(), FilterError> {
    validate(buf)?;
    if !angle_deg.is_finite() || !(-360.0..=360.0).contains(&angle_deg) {
        return Err(FilterError::InvalidParams(format!(
            "motion angle {angle_deg} is outside -360..=360"
        )));
    }
    if !(1..=999).contains(&distance) {
        return Err(FilterError::InvalidParams(format!(
            "motion distance {distance} is outside 1..=999"
        )));
    }
    if distance == 1 {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let angle = angle_deg.to_radians();
    // Photoshop angles run counter-clockwise on screen (0 = east, +90 = up);
    // this buffer's y grows downward, so the vertical component is negated.
    let (dx, dy) = (angle.cos(), -angle.sin());
    let taps = distance as isize;
    let half = (distance as f64 - 1.0) / 2.0;
    let divisor = distance as f64;
    let steps: Vec<(isize, isize)> = (0..taps)
        .map(|k| {
            let f = k as f64 - half;
            ((f * dx).round() as isize, (f * dy).round() as isize)
        })
        .collect();
    for c in 0..planes {
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, out) in row.iter_mut().enumerate() {
                    let mut acc = 0f64;
                    for &(sx, sy) in &steps {
                        let sx = clamp_index(x as isize + sx, w);
                        let sy = clamp_index(y as isize + sy, h);
                        acc += src[sy * w + sx] as f64;
                    }
                    *out = (acc / divisor).round().clamp(0.0, 255.0) as u8;
                }
            });
    }
    Ok(())
}

pub fn radial(
    buf: &mut PixelBuffer,
    method: RadialMethod,
    amount: f64,
    quality: Quality,
) -> Result<(), FilterError> {
    validate(buf)?;
    if !amount.is_finite() || amount < 0.0 {
        return Err(FilterError::InvalidParams(format!(
            "radial amount {amount} must be finite and non-negative"
        )));
    }
    if amount == 0.0 {
        return Ok(());
    }
    if matches!(method, RadialMethod::Zoom) && amount > 100.0 {
        return Err(FilterError::InvalidParams(format!(
            "radial zoom amount {amount} is outside 1..=100"
        )));
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let samples: usize = match quality {
        Quality::Draft => 8,
        Quality::Good => 16,
        Quality::Best => 32,
    };
    let cx = (w as f64 - 1.0) / 2.0;
    let cy = (h as f64 - 1.0) / 2.0;
    // The planes interleaved, so a tap reads one cache line however far
    // along its arc the sample lands.
    let mut pixels = vec![[0u8; 4]; n];
    pixels
        .par_chunks_mut(w.max(1))
        .enumerate()
        .for_each(|(y, row)| {
            for c in 0..planes {
                let plane = &buf.data[c * n + y * w..c * n + y * w + w];
                for (px, &v) in row.iter_mut().zip(plane) {
                    px[c] = v;
                }
            }
        });
    // Spin rotates each pixel's offset from the centre by the sample angles,
    // whose sines and cosines are found once: the same point as
    // `r·(cos(θ+off), sin(θ+off))`, without trigonometry per sample.
    let spin: Vec<(f64, f64)> = (0..samples)
        .map(|k| {
            let dt = amount.to_radians() / samples as f64;
            let off = (k as f64 - (samples as f64 - 1.0) / 2.0) * dt;
            (off.cos(), off.sin())
        })
        .collect();
    // ponytail: polar resample + midpoint 1-D smear, not the reference's kernel.
    // Upgrade to supersampled/jittered sampling if Draft banding is visible.
    // Tiles run in parallel and each sample point and its bilinear weights are
    // found once for every plane (`kernel_digests` pins the bytes).
    let value = |x: usize, y: usize, acc: &mut [f64]| -> [u8; 4] {
        let px = x as f64 - cx;
        let py = y as f64 - cy;
        acc.fill(0.0);
        let mut sample = |sx: f64, sy: f64| {
            let tap = Bilinear::at(w, h, sx, sy);
            let corners = [
                pixels[tap.i00],
                pixels[tap.i10],
                pixels[tap.i01],
                pixels[tap.i11],
            ];
            for (c, a) in acc.iter_mut().enumerate() {
                *a += tap.mix(corners.map(|p| p[c]));
            }
        };
        match method {
            RadialMethod::Spin => {
                for &(cos, sin) in &spin {
                    sample(cx + px * cos - py * sin, cy + px * sin + py * cos);
                }
            }
            RadialMethod::Zoom => {
                let radius = (px * px + py * py).sqrt();
                let theta = py.atan2(px);
                let (cos, sin) = (theta.cos(), theta.sin());
                let k_amt = (amount / 100.0).clamp(0.0, 1.0);
                for k in 0..samples {
                    let t = k as f64 / (samples as f64 - 1.0);
                    let frac = 1.0 - k_amt / 2.0 + k_amt * t;
                    let rr = radius * frac;
                    sample(cx + rr * cos, cy + rr * sin);
                }
            }
        }
        let mut out = [0u8; 4];
        for (o, &a) in out.iter_mut().zip(acc.iter()) {
            *o = (a / samples as f64).round().clamp(0.0, 255.0) as u8;
        }
        out
    };
    // Square tiles rather than rows: a spin sample of a row sweeps a diagonal
    // across thousands of rows and misses the cache at nearly every tap, while
    // a tile's samples at one angle stay as compact as the tile.
    const RADIAL_TILE: usize = 64;
    let tiles_x = w.div_ceil(RADIAL_TILE);
    let tiles: Vec<Vec<[u8; 4]>> = (0..tiles_x * h.div_ceil(RADIAL_TILE))
        .into_par_iter()
        .map_init(
            || vec![0f64; planes],
            |acc, t| {
                let (x0, y0) = ((t % tiles_x) * RADIAL_TILE, (t / tiles_x) * RADIAL_TILE);
                let (x1, y1) = ((x0 + RADIAL_TILE).min(w), (y0 + RADIAL_TILE).min(h));
                let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
                for y in y0..y1 {
                    for x in x0..x1 {
                        out.push(value(x, y, acc));
                    }
                }
                out
            },
        )
        .collect();
    for c in 0..planes {
        buf.data[c * n..c * n + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                let (band, dy) = (y / RADIAL_TILE, y % RADIAL_TILE);
                for tx in 0..tiles_x {
                    let x0 = tx * RADIAL_TILE;
                    let tw = RADIAL_TILE.min(w - x0);
                    let tile = &tiles[band * tiles_x + tx];
                    for (dx, out) in row[x0..x0 + tw].iter_mut().enumerate() {
                        *out = tile[dy * tw + dx][c];
                    }
                }
            });
    }
    Ok(())
}

pub fn average(buf: &mut PixelBuffer) -> Result<(), FilterError> {
    validate(buf)?;
    let n = buf.pixel_count();
    let planes = (buf.channels as usize).min(3);
    for c in 0..planes {
        let base = c * n;
        let sum: u64 = buf.data[base..base + n].iter().map(|&v| v as u64).sum();
        let mean = (sum as f64 / n as f64).round().clamp(0.0, 255.0) as u8;
        buf.data[base..base + n].fill(mean);
    }
    Ok(())
}

/// Blur and Blur More: a Gaussian at a fixed radius, gentle and stronger.
pub fn simple(buf: &mut PixelBuffer, more: bool) -> Result<(), FilterError> {
    gaussian(buf, if more { BLUR_MORE_RADIUS } else { BLUR_RADIUS })
}

/// A clamp-to-edge bilinear tap: the four sample indices and the weights,
/// found once and mixed for each plane.
struct Bilinear {
    i00: usize,
    i10: usize,
    i01: usize,
    i11: usize,
    fx: f64,
    fy: f64,
}

impl Bilinear {
    fn at(w: usize, h: usize, x: f64, y: f64) -> Self {
        let x0 = x.floor();
        let y0 = y.floor();
        let xi = clamp_index(x0 as isize, w);
        let yi = clamp_index(y0 as isize, h);
        let xi1 = clamp_index(x0 as isize + 1, w);
        let yi1 = clamp_index(y0 as isize + 1, h);
        Self {
            i00: yi * w + xi,
            i10: yi * w + xi1,
            i01: yi1 * w + xi,
            i11: yi1 * w + xi1,
            fx: x - x0,
            fy: y - y0,
        }
    }

    /// The tap over one plane's corner values `[p00, p10, p01, p11]`.
    fn mix(&self, [p00, p10, p01, p11]: [u8; 4]) -> f64 {
        let (fx, fy) = (self.fx, self.fy);
        let (p00, p10, p01, p11) = (p00 as f64, p10 as f64, p01 as f64, p11 as f64);
        (p00 * (1.0 - fx) + p10 * fx) * (1.0 - fy) + (p01 * (1.0 - fx) + p11 * fx) * fy
    }
}

fn box_blur_planes(buf: &mut PixelBuffer, radius: u32) {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    let planes = (buf.channels as usize).min(3);
    let r = radius as isize;
    for c in 0..planes {
        let base = c * n;
        let src: Vec<f64> = buf.data[base..base + n].iter().map(|&v| v as f64).collect();
        let mut tmp = vec![0f64; n];
        box_blur_axis(&src, &mut tmp, w, h, r, true);
        let mut out = vec![0f64; n];
        box_blur_axis(&tmp, &mut out, w, h, r, false);
        for (dst, v) in buf.data[base..base + n].iter_mut().zip(out.iter()) {
            *dst = v.round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Columns a vertical box pass gathers at once, so each worker reads whole
/// row runs instead of striding the plane one sample at a time.
const BOX_STRIP: usize = 64;

/// Separable moving average over `(2r+1)` taps, clamp-to-edge. Prefix sums keep
/// it O(1) per pixel even when the radius exceeds the image. Rows (or column
/// strips) run in parallel; each line sums in the same order as a serial pass.
fn box_blur_axis(src: &[f64], dst: &mut [f64], w: usize, h: usize, r: isize, horizontal: bool) {
    let taps = (2 * r + 1) as f64;
    if horizontal {
        dst.par_chunks_mut(w).enumerate().for_each_init(
            || vec![0f64; w + 1],
            |pre, (y, out)| {
                let line = &src[y * w..y * w + w];
                for (k, &v) in line.iter().enumerate() {
                    pre[k + 1] = pre[k] + v;
                }
                for (x, out) in out.iter_mut().enumerate() {
                    let a = x as isize - r;
                    let b = x as isize + r;
                    *out = (prefix_at(pre, line, b) - prefix_at(pre, line, a - 1)) / taps;
                }
            },
        );
    } else {
        let strips: Vec<Vec<f64>> = (0..w)
            .step_by(BOX_STRIP)
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|x0| {
                let sw = BOX_STRIP.min(w - x0);
                let mut out = vec![0f64; h * sw];
                let mut line = vec![0f64; h];
                let mut pre = vec![0f64; h + 1];
                for dx in 0..sw {
                    for (y, v) in line.iter_mut().enumerate() {
                        *v = src[y * w + x0 + dx];
                    }
                    for (k, &v) in line.iter().enumerate() {
                        pre[k + 1] = pre[k] + v;
                    }
                    for y in 0..h {
                        let a = y as isize - r;
                        let b = y as isize + r;
                        out[y * sw + dx] =
                            (prefix_at(&pre, &line, b) - prefix_at(&pre, &line, a - 1)) / taps;
                    }
                }
                out
            })
            .collect();
        dst.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (i, strip) in strips.iter().enumerate() {
                let x0 = i * BOX_STRIP;
                let sw = BOX_STRIP.min(w - x0);
                row[x0..x0 + sw].copy_from_slice(&strip[y * sw..y * sw + sw]);
            }
        });
    }
}

/// `H(t) = Σ_{i=0..t} clamp(i, n)` where `pre[k] = Σ_{i<k} v[i]`; `H(-1) = 0`.
fn prefix_at(pre: &[f64], v: &[f64], t: isize) -> f64 {
    let n = v.len() as isize;
    if t < 0 {
        (t + 1) as f64 * v[0]
    } else if t >= n {
        pre[v.len()] + (t - n + 1) as f64 * v[v.len() - 1]
    } else {
        pre[t as usize + 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Surface Blur runs on the photorust engine, so it is reached through `apply`.
    fn surface(buf: &mut PixelBuffer, radius: u32, threshold: u8) -> Result<(), FilterError> {
        crate::apply(&crate::Filter::SurfaceBlur { radius, threshold }, buf)
    }

    fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
        let n = (w * h) as usize;
        assert_eq!(px.len(), n);
        let mut data = vec![0u8; n * 3];
        for (i, p) in px.iter().enumerate() {
            data[i] = p[0];
            data[n + i] = p[1];
            data[2 * n + i] = p[2];
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        }
    }

    fn buf4(w: u32, h: u32, px: &[[u8; 4]]) -> PixelBuffer {
        let n = (w * h) as usize;
        assert_eq!(px.len(), n);
        let mut data = vec![0u8; n * 4];
        for (i, p) in px.iter().enumerate() {
            data[i] = p[0];
            data[n + i] = p[1];
            data[2 * n + i] = p[2];
            data[3 * n + i] = p[3];
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    fn px3(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 3] {
        let w = buf.width as usize;
        let n = buf.pixel_count();
        let i = y as usize * w + x as usize;
        [buf.data[i], buf.data[n + i], buf.data[2 * n + i]]
    }

    fn solid4(w: u32, h: u32, rgb: [u8; 3], a: u8) -> PixelBuffer {
        let px: Vec<[u8; 4]> = (0..w * h).map(|_| [rgb[0], rgb[1], rgb[2], a]).collect();
        buf4(w, h, &px)
    }

    #[test]
    fn gaussian_box_simple_are_near_noop_on_solid() {
        let base = solid4(6, 5, [40, 90, 160], 123);
        let mut g = base.clone();
        gaussian(&mut g, 3.0).unwrap();
        let mut b = base.clone();
        r#box(&mut b, 3).unwrap();
        let mut s = base.clone();
        simple(&mut s, true).unwrap();
        for y in 0..5 {
            for x in 0..6 {
                for buf in [&g, &b, &s] {
                    let p = px3(buf, x, y);
                    assert!((p[0] as i32 - 40).abs() <= 1);
                    assert!((p[1] as i32 - 90).abs() <= 1);
                    assert!((p[2] as i32 - 160).abs() <= 1);
                }
            }
        }
    }

    #[test]
    fn gaussian_edge_transition_widens_with_radius() {
        let px: Vec<[u8; 3]> = (0..16)
            .map(|x| if x < 8 { [0, 0, 0] } else { [255, 255, 255] })
            .collect();
        let base = buf3(16, 1, &px);
        let mut small = base.clone();
        gaussian(&mut small, 1.0).unwrap();
        let mut large = base.clone();
        gaussian(&mut large, 3.0).unwrap();
        let partial = |b: &PixelBuffer| {
            (0..16)
                .filter(|&x| b.data[x] > 0 && b.data[x] < 255)
                .count()
        };
        assert!(
            partial(&large) > partial(&small),
            "large={} small={}",
            partial(&large),
            partial(&small)
        );
    }

    #[test]
    fn motion_at_zero_streaks_horizontally() {
        let mut b = PixelBuffer::new(9, 9, 3);
        b.data[4 * 9 + 4] = 255;
        motion(&mut b, 0.0, 5).unwrap();
        assert_eq!(px3(&b, 4, 4)[0], 51);
        assert_eq!(px3(&b, 2, 4)[0], 51);
        assert_eq!(px3(&b, 6, 4)[0], 51);
        assert_eq!(px3(&b, 4, 3)[0], 0);
        assert_eq!(px3(&b, 4, 5)[0], 0);
    }

    #[test]
    fn motion_at_ninety_streaks_vertically() {
        let mut b = PixelBuffer::new(9, 9, 3);
        b.data[4 * 9 + 4] = 255;
        motion(&mut b, 90.0, 5).unwrap();
        assert_eq!(px3(&b, 4, 4)[0], 51);
        assert_eq!(px3(&b, 4, 2)[0], 51);
        assert_eq!(px3(&b, 4, 6)[0], 51);
        assert_eq!(px3(&b, 3, 4)[0], 0);
        assert_eq!(px3(&b, 5, 4)[0], 0);
    }

    #[test]
    fn motion_at_forty_five_smears_along_the_photoshop_diagonal() {
        let mut b = PixelBuffer::new(9, 9, 3);
        b.data[4 * 9 + 4] = 255;
        motion(&mut b, 45.0, 5).unwrap();
        // Photoshop's +45° runs lower-left to upper-right: the bright pixel
        // smears up-right and down-left, not down-right.
        assert_eq!(px3(&b, 5, 3)[0], 102);
        assert_eq!(px3(&b, 3, 5)[0], 102);
        assert_eq!(px3(&b, 5, 5)[0], 0);
        assert_eq!(px3(&b, 3, 3)[0], 0);
    }

    #[test]
    fn average_equals_region_mean() {
        let px = [[10, 20, 30], [20, 30, 40], [30, 40, 50], [40, 50, 60]];
        let mut b = buf3(2, 2, &px);
        average(&mut b).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(px3(&b, x, y), [25, 35, 45]);
            }
        }
    }

    #[test]
    fn blur_and_blur_more_are_fixed_gaussians() {
        let mut base = PixelBuffer::new(9, 9, 3);
        base.data[4 * 9 + 4] = 255;
        for (more, radius) in [(false, BLUR_RADIUS), (true, BLUR_MORE_RADIUS)] {
            let mut got = base.clone();
            simple(&mut got, more).unwrap();
            let mut want = base.clone();
            gaussian(&mut want, radius).unwrap();
            assert_eq!(got, want, "more={more}");
        }
    }

    #[test]
    fn simple_more_changes_more_than_simple() {
        let mut base = PixelBuffer::new(9, 9, 3);
        base.data[4 * 9 + 4] = 255;
        let mut less = base.clone();
        simple(&mut less, false).unwrap();
        let mut more = base.clone();
        simple(&mut more, true).unwrap();
        let diff = |a: &PixelBuffer, b: &PixelBuffer| {
            a.data
                .iter()
                .zip(b.data.iter())
                .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64)
                .sum::<u64>()
        };
        assert!(diff(&base, &more) > diff(&base, &less));
    }

    #[test]
    fn surface_preserves_edge_but_higher_threshold_smooths_across_it() {
        let px: Vec<[u8; 3]> = (0..16)
            .map(|x| if x < 8 { [50, 50, 50] } else { [200, 200, 200] })
            .collect();
        let base = buf3(16, 1, &px);
        let mut tight = base.clone();
        surface(&mut tight, 3, 10).unwrap();
        let mut loose = base.clone();
        surface(&mut loose, 3, 255).unwrap();
        assert!(
            (px3(&tight, 2, 0)[0] as i32 - 50).abs() <= 1,
            "flat region drifted"
        );
        assert!(
            px3(&tight, 7, 0)[0] < 60,
            "tight threshold leaked across edge"
        );
        assert!(px3(&loose, 7, 0)[0] > px3(&tight, 7, 0)[0]);
    }

    #[test]
    fn radial_changes_image_and_zero_amount_is_noop() {
        let mut base = PixelBuffer::new(11, 11, 3);
        base.data[2 * 11 + 2] = 255;
        let mut zoom = base.clone();
        radial(&mut zoom, RadialMethod::Zoom, 50.0, Quality::Good).unwrap();
        assert!(base.data.iter().zip(zoom.data.iter()).any(|(a, b)| a != b));
        let mut spin = base.clone();
        radial(&mut spin, RadialMethod::Spin, 20.0, Quality::Good).unwrap();
        assert!(base.data.iter().zip(spin.data.iter()).any(|(a, b)| a != b));
        let mut noop = base.clone();
        radial(&mut noop, RadialMethod::Spin, 0.0, Quality::Best).unwrap();
        assert_eq!(noop.data, base.data);
    }

    #[test]
    fn alpha_is_untouched_by_every_blur() {
        let expected: Vec<u8> = (0..20u32).map(|i| (i * 7) as u8).collect();
        let px: Vec<[u8; 4]> = expected.iter().map(|&a| [30, 60, 90, a]).collect();
        let base = buf4(5, 4, &px);
        let check = |b: &PixelBuffer| {
            let n = b.pixel_count();
            for (i, &a) in expected.iter().enumerate() {
                assert_eq!(b.data[3 * n + i], a);
            }
        };
        let mut b1 = base.clone();
        gaussian(&mut b1, 2.0).unwrap();
        check(&b1);
        let mut b2 = base.clone();
        r#box(&mut b2, 2).unwrap();
        check(&b2);
        let mut b3 = base.clone();
        motion(&mut b3, 30.0, 5).unwrap();
        check(&b3);
        let mut b4 = base.clone();
        radial(&mut b4, RadialMethod::Spin, 10.0, Quality::Good).unwrap();
        check(&b4);
        let mut b5 = base.clone();
        average(&mut b5).unwrap();
        check(&b5);
        let mut b6 = base.clone();
        simple(&mut b6, true).unwrap();
        check(&b6);
        let mut b7 = base.clone();
        surface(&mut b7, 2, 30).unwrap();
        check(&b7);
    }

    #[test]
    fn blur_parameter_ranges_are_validated() {
        let mut b = PixelBuffer::new(4, 4, 3);
        assert!(gaussian(&mut b, -1.0).is_err());
        assert!(gaussian(&mut b, f64::NAN).is_err());
        assert!(motion(&mut b, 400.0, 3).is_err());
        assert!(motion(&mut b, 0.0, 0).is_err());
        assert!(motion(&mut b, 0.0, 1000).is_err());
        assert!(radial(&mut b, RadialMethod::Zoom, 101.0, Quality::Good).is_err());
        assert!(radial(&mut b, RadialMethod::Spin, f64::NAN, Quality::Good).is_err());
        assert!(surface(&mut b, 0, 10).is_err());
        assert!(surface(&mut b, 101, 10).is_err());
        assert!(surface(&mut b, 2, 0).is_err());
        assert!(gaussian(&mut b, 0.0).is_ok());
        assert!(motion(&mut b, 0.0, 1).is_ok());
        assert!(radial(&mut b, RadialMethod::Spin, 0.0, Quality::Good).is_ok());
    }

    #[test]
    fn tiny_images_do_not_panic() {
        for (w, h) in [(1, 1), (1, 7), (7, 1)] {
            let n = (w * h) as usize;
            let px3v: Vec<[u8; 3]> = (0..n)
                .map(|i| [(i * 3) as u8, (i * 5) as u8, (i * 7) as u8])
                .collect();
            let mut a = buf3(w, h, &px3v);
            gaussian(&mut a, 5.0).unwrap();
            r#box(&mut a, 4).unwrap();
            simple(&mut a, true).unwrap();
            average(&mut a).unwrap();
            motion(&mut a, 45.0, 7).unwrap();
            radial(&mut a, RadialMethod::Zoom, 50.0, Quality::Draft).unwrap();
            surface(&mut a, 3, 20).unwrap();

            let px4v: Vec<[u8; 4]> = (0..n)
                .map(|i| [(i * 3) as u8, (i * 5) as u8, (i * 7) as u8, 200])
                .collect();
            let mut b = buf4(w, h, &px4v);
            gaussian(&mut b, 5.0).unwrap();
            r#box(&mut b, 4).unwrap();
            simple(&mut b, true).unwrap();
            average(&mut b).unwrap();
            motion(&mut b, 45.0, 7).unwrap();
            radial(&mut b, RadialMethod::Spin, 10.0, Quality::Best).unwrap();
            surface(&mut b, 3, 20).unwrap();
        }
    }
}
