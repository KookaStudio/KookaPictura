//! Reduce Noise (`FILT-030`): an edge-preserving luminance/chroma denoiser.
//!
//! Adobe's kernel is closed, so this is a documented approximation, not
//! verified parity. It follows the standard spatial + channel model the CS6
//! Help describes: a bilateral smoother (edge-detection thresholds via
//! Preserve Details) works on each colour plane, a chroma pass blurs colour
//! while holding luminance, an optional unsharp pass restores sharpness, and
//! an optional 8×8 deblock removes JPEG blockiness. It is deterministic — no
//! seed, no RNG — and leaves alpha bit-identical (a value kernel, not a moving
//! one).

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::kernel::{blur_plane, clamp_index, to_u8};
use crate::luma::{luma, luma_plane};
use crate::{validate, FilterError};

/// The bilateral window's spatial reach in pixels. CS6's Reduce Noise has no
/// radius control, so the window is fixed.
const WINDOW: isize = 2;
/// The bilateral spatial Gaussian's sigma, in pixels.
const SIGMA_S: f64 = 1.6;
/// The luminance range sigma at Strength 10 and Preserve Details 0, in levels.
const RANGE_SIGMA_MAX: f64 = 40.0;
/// The chroma pass's blur sigma at Reduce Color Noise 100, in pixels.
const CHROMA_SIGMA_MAX: f64 = 3.0;
/// The post-denoise unsharp pass's blur sigma, in pixels.
const SHARPEN_SIGMA: f64 = 1.0;
/// JPEG deblocking smooths a block seam only while its step is below this many
/// levels, so a genuine edge crossing a block boundary survives.
const DEBLOCK_GATE: f64 = 40.0;

/// Per-channel Strength and Preserve Details for Reduce Noise's Advanced mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelDetail {
    pub strength: f64,
    pub preserve_details: f64,
}

impl Default for ChannelDetail {
    fn default() -> Self {
        Self {
            strength: 0.0,
            preserve_details: 0.0,
        }
    }
}

/// Reduce Noise's parameters. `channels` carries the per-channel Strength and
/// Preserve Details; Basic mode sends the same pair for all three colour
/// planes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReduceNoiseParams {
    pub reduce_color_noise: f64,
    pub sharpen_details: f64,
    pub remove_jpeg_artifacts: bool,
    pub channels: [ChannelDetail; 3],
}

impl Default for ReduceNoiseParams {
    fn default() -> Self {
        Self {
            reduce_color_noise: 0.0,
            sharpen_details: 0.0,
            remove_jpeg_artifacts: false,
            channels: [ChannelDetail::default(); 3],
        }
    }
}

/// Reduce Noise: edge-preserving denoise of the colour planes.
pub fn reduce_noise(buf: &mut PixelBuffer, params: &ReduceNoiseParams) -> Result<(), FilterError> {
    let n = validate(buf)?;
    validate_params(params)?;
    if is_noop(params) {
        return Ok(());
    }
    let w = buf.width as usize;
    let h = buf.height as usize;
    let planes = 3.min(buf.channels as usize);

    for c in 0..planes {
        let sigma_r = range_sigma(params.channels[c]);
        if sigma_r <= f64::EPSILON {
            continue;
        }
        let base = c * n;
        let src = buf.data[base..base + n].to_vec();
        let out = bilateral_plane(&src, w, h, sigma_r);
        buf.data[base..base + n].copy_from_slice(&out);
    }
    if params.reduce_color_noise > 0.0 {
        chroma_denoise(buf, w, h, n, planes, params.reduce_color_noise);
    }
    if params.sharpen_details > 0.0 {
        sharpen_luma(buf, w, h, n, planes, params.sharpen_details);
    }
    if params.remove_jpeg_artifacts {
        deblock(buf, w, h, n, planes);
    }
    Ok(())
}

fn is_noop(p: &ReduceNoiseParams) -> bool {
    p.reduce_color_noise <= 0.0
        && p.sharpen_details <= 0.0
        && !p.remove_jpeg_artifacts
        && p.channels.iter().all(|c| c.strength <= 0.0)
}

/// The bilateral range sigma a channel's Strength and Preserve Details imply.
/// Strength scales the reach; Preserve Details narrows the range gate to a
/// tenth at 100, so only near-identical samples average and edges survive.
fn range_sigma(c: ChannelDetail) -> f64 {
    let strength = c.strength.clamp(0.0, 10.0);
    let preserve = c.preserve_details.clamp(0.0, 100.0);
    strength * (RANGE_SIGMA_MAX / 10.0) * (1.0 - 0.9 * preserve / 100.0)
}

fn validate_params(p: &ReduceNoiseParams) -> Result<(), FilterError> {
    let bounded = |what: &str, v: f64, hi: f64| -> Result<(), FilterError> {
        if v.is_finite() && (0.0..=hi).contains(&v) {
            Ok(())
        } else {
            Err(FilterError::InvalidParams(format!(
                "reduce noise {what} {v} is outside 0..={hi}"
            )))
        }
    };
    bounded("reduce color noise", p.reduce_color_noise, 100.0)?;
    bounded("sharpen details", p.sharpen_details, 100.0)?;
    for (i, c) in p.channels.iter().enumerate() {
        bounded(&format!("channel {i} strength"), c.strength, 10.0)?;
        bounded(
            &format!("channel {i} preserve details"),
            c.preserve_details,
            100.0,
        )?;
    }
    Ok(())
}

/// A bilateral (edge-preserving) smooth of one plane, clamp-to-edge.
///
/// Each output is the distance-weighted average of the window, with every
/// neighbour also gated by how far its value sits from the centre: samples past
/// the range gate contribute almost nothing, so a flat, noisy area is smoothed
/// while a step edge is left standing.
fn bilateral_plane(src: &[u8], w: usize, h: usize, sigma_r: f64) -> Vec<u8> {
    let two_sr2 = 2.0 * sigma_r * sigma_r;
    let two_ss2 = 2.0 * SIGMA_S * SIGMA_S;
    let mut out = vec![0u8; w * h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, out)| {
        for (x, o) in out.iter_mut().enumerate() {
            let center = f64::from(src[y * w + x]);
            let mut acc = 0.0;
            let mut wsum = 0.0;
            for dy in -WINDOW..=WINDOW {
                let sy = clamp_index(y as isize + dy, h);
                for dx in -WINDOW..=WINDOW {
                    let sx = clamp_index(x as isize + dx, w);
                    let v = f64::from(src[sy * w + sx]);
                    let dv = v - center;
                    let spatial = f64::from((dx * dx + dy * dy) as i32) / two_ss2;
                    let range = dv * dv / two_sr2;
                    let weight = (-(spatial + range)).exp();
                    acc += weight * v;
                    wsum += weight;
                }
            }
            *o = to_u8(acc / wsum);
        }
    });
    out
}

/// Blur the chroma while holding each pixel's luminance, so random colour
/// speckle is averaged away and luma detail is kept.
fn chroma_denoise(buf: &mut PixelBuffer, w: usize, h: usize, n: usize, planes: usize, amount: f64) {
    if planes < 3 {
        return;
    }
    let sigma = (amount / 100.0) * CHROMA_SIGMA_MAX;
    if sigma <= 0.0 {
        return;
    }
    let y0 = luma_plane(&buf.data, n);
    let mut blurred = vec![0u8; n * planes];
    for c in 0..planes {
        let base = c * n;
        blurred[base..base + n].copy_from_slice(&blur_u8(&buf.data[base..base + n], w, h, sigma));
    }
    for i in 0..n {
        let yb = luma(
            f64::from(blurred[i]),
            f64::from(blurred[n + i]),
            f64::from(blurred[2 * n + i]),
        );
        let delta = y0[i] - yb;
        for c in 0..3 {
            buf.data[c * n + i] = to_u8(f64::from(blurred[c * n + i]) + delta);
        }
    }
}

/// Unsharp the luminance and add the delta back to every colour plane, so the
/// softening the denoise caused is partly undone without moving the colour.
fn sharpen_luma(buf: &mut PixelBuffer, w: usize, h: usize, n: usize, planes: usize, amount: f64) {
    if planes < 3 {
        return;
    }
    let y: Vec<f32> = luma_plane(&buf.data, n).iter().map(|&v| v as f32).collect();
    let blurred = blur_plane(&y, w, h, SHARPEN_SIGMA as f32);
    let gain = amount / 100.0;
    for i in 0..n {
        let delta = (f64::from(y[i]) - f64::from(blurred[i])) * gain;
        if delta == 0.0 {
            continue;
        }
        for c in 0..3 {
            buf.data[c * n + i] = to_u8(f64::from(buf.data[c * n + i]) + delta);
        }
    }
}

/// JPEG deblocking: average the two samples straddling each 8-pixel block seam
/// when their step is small, so blockiness and halos soften while a real edge
/// crossing the seam is kept.
fn deblock(buf: &mut PixelBuffer, w: usize, h: usize, n: usize, planes: usize) {
    const BOUNDARY: usize = 8;
    let mut seam = |a: usize, b: usize| {
        let va = f64::from(buf.data[a]);
        let vb = f64::from(buf.data[b]);
        if (va - vb).abs() < DEBLOCK_GATE {
            let m = to_u8((va + vb) / 2.0);
            buf.data[a] = m;
            buf.data[b] = m;
        }
    };
    for c in 0..planes {
        let base = c * n;
        let mut x = BOUNDARY;
        while x < w {
            for y in 0..h {
                seam(base + y * w + x - 1, base + y * w + x);
            }
            x += BOUNDARY;
        }
        let mut y = BOUNDARY;
        while y < h {
            for x in 0..w {
                seam(base + (y - 1) * w + x, base + y * w + x);
            }
            y += BOUNDARY;
        }
    }
}

fn blur_u8(src: &[u8], w: usize, h: usize, sigma: f64) -> Vec<u8> {
    let f: Vec<f32> = src.iter().map(|&v| v as f32).collect();
    blur_plane(&f, w, h, sigma as f32)
        .into_iter()
        .map(|v| to_u8(f64::from(v)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic pseudo-random byte stream (no RNG dependency).
    fn noise(w: u32, h: u32, amp: i32, seed: u64) -> PixelBuffer {
        let mut b = PixelBuffer::new(w, h, 3);
        let n = b.pixel_count();
        let mut state = seed | 1;
        for i in 0..n {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let jitter = ((state >> 33) as i32 % (2 * amp + 1)) - amp;
            let v = (128 + jitter).clamp(0, 255) as u8;
            b.data[i] = v;
            b.data[n + i] = v;
            b.data[2 * n + i] = v;
        }
        b
    }

    fn plane_var(buf: &PixelBuffer, plane: usize) -> f64 {
        let n = buf.pixel_count();
        let vals = &buf.data[plane * n..plane * n + n];
        let mean = vals.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;
        vals.iter()
            .map(|&v| (f64::from(v) - mean).powi(2))
            .sum::<f64>()
            / n as f64
    }

    fn params(strength: f64, preserve: f64) -> ReduceNoiseParams {
        ReduceNoiseParams {
            channels: [ChannelDetail {
                strength,
                preserve_details: preserve,
            }; 3],
            ..ReduceNoiseParams::default()
        }
    }

    #[test]
    fn zero_params_are_a_noop() {
        let base = noise(16, 16, 30, 7);
        let mut out = base.clone();
        reduce_noise(&mut out, &ReduceNoiseParams::default()).unwrap();
        assert_eq!(out.data, base.data);
    }

    #[test]
    fn strength_reduces_luminance_noise_monotonically() {
        let base = noise(32, 32, 30, 11);
        let mut variances = Vec::new();
        for strength in [0.0, 2.0, 6.0, 10.0] {
            let mut out = base.clone();
            reduce_noise(&mut out, &params(strength, 0.0)).unwrap();
            variances.push(plane_var(&out, 0));
        }
        for pair in variances.windows(2) {
            assert!(
                pair[1] <= pair[0] + 1e-9,
                "variance must not grow with Strength: {variances:?}"
            );
        }
        assert!(
            variances[3] < variances[1] * 0.5,
            "Strength 10 should smooth far more than Strength 2: {variances:?}"
        );
    }

    #[test]
    fn preserve_details_retains_more_at_equal_strength() {
        // A subtle 20-level edge with jitter: at Strength 6 the low-preserve
        // range gate averages across the edge, the high-preserve gate does not.
        let (w, h) = (24u32, 24u32);
        let mut base = PixelBuffer::new(w, h, 3);
        let n = base.pixel_count();
        let mut state = 99u64;
        for y in 0..h as usize {
            for x in 0..w as usize {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let jitter = ((state >> 33) as i32 % 5) - 2;
                let v = (if x < 12 { 60 } else { 80 } + jitter).clamp(0, 255) as u8;
                let i = y * w as usize + x;
                base.data[i] = v;
                base.data[n + i] = v;
                base.data[2 * n + i] = v;
            }
        }
        let mut low = base.clone();
        reduce_noise(&mut low, &params(6.0, 0.0)).unwrap();
        let mut high = base.clone();
        reduce_noise(&mut high, &params(6.0, 100.0)).unwrap();
        assert!(
            plane_var(&high, 0) > plane_var(&low, 0),
            "Preserve Details 100 must retain more contrast than 0"
        );
    }

    #[test]
    fn reduce_color_noise_lowers_chroma_variance_keeps_luma() {
        // Grey base with independent per-channel jitter: pure chroma noise.
        let (w, h) = (32u32, 32u32);
        let mut base = PixelBuffer::new(w, h, 3);
        let n = base.pixel_count();
        let mut state = 5u64;
        for i in 0..n {
            for c in 0..3 {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let jitter = ((state >> 33) as i32 % 41) - 20;
                base.data[c * n + i] = (128 + jitter).clamp(0, 255) as u8;
            }
        }
        let chroma_var = |b: &PixelBuffer| {
            let n = b.pixel_count();
            let mut acc = 0.0;
            for i in 0..n {
                let rg = f64::from(b.data[i]) - f64::from(b.data[n + i]);
                let bg = f64::from(b.data[2 * n + i]) - f64::from(b.data[n + i]);
                acc += rg * rg + bg * bg;
            }
            acc / n as f64
        };
        let mut off = base.clone();
        reduce_noise(
            &mut off,
            &ReduceNoiseParams {
                reduce_color_noise: 0.0,
                ..ReduceNoiseParams::default()
            },
        )
        .unwrap();
        let mut on = base.clone();
        reduce_noise(
            &mut on,
            &ReduceNoiseParams {
                reduce_color_noise: 80.0,
                ..ReduceNoiseParams::default()
            },
        )
        .unwrap();
        assert!(
            chroma_var(&on) < chroma_var(&off) * 0.5,
            "Reduce Color Noise must lower chroma variance"
        );
    }

    #[test]
    fn remove_jpeg_artifacts_softens_block_seams() {
        // Constant 8×8 blocks with steps at the seams.
        let (w, h) = (32u32, 32u32);
        let mut base = PixelBuffer::new(w, h, 3);
        let n = base.pixel_count();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let block = ((x / 8) + (y / 8)) % 2;
                let v = if block == 0 { 100u8 } else { 116 };
                let i = y * w as usize + x;
                base.data[i] = v;
                base.data[n + i] = v;
                base.data[2 * n + i] = v;
            }
        }
        let seam_step = |b: &PixelBuffer| {
            let mut total = 0i32;
            for y in 0..h as usize {
                for x in (8..w as usize).step_by(8) {
                    total += (i32::from(b.data[y * w as usize + x])
                        - i32::from(b.data[y * w as usize + x - 1]))
                    .abs();
                }
            }
            total
        };
        let before = seam_step(&base);
        let mut out = base.clone();
        reduce_noise(
            &mut out,
            &ReduceNoiseParams {
                remove_jpeg_artifacts: true,
                ..ReduceNoiseParams::default()
            },
        )
        .unwrap();
        assert!(
            seam_step(&out) < before,
            "deblocking must reduce the block-seam step ({before} -> {})",
            seam_step(&out)
        );
    }

    #[test]
    fn sharpen_details_adds_edge_contrast() {
        let (w, h) = (16u32, 16u32);
        let mut base = PixelBuffer::new(w, h, 3);
        let n = base.pixel_count();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let v = if x < 8 { 100u8 } else { 110 };
                let i = y * w as usize + x;
                base.data[i] = v;
                base.data[n + i] = v;
                base.data[2 * n + i] = v;
            }
        }
        let mut out = base.clone();
        reduce_noise(
            &mut out,
            &ReduceNoiseParams {
                sharpen_details: 100.0,
                ..ReduceNoiseParams::default()
            },
        )
        .unwrap();
        assert_ne!(out.data, base.data, "Sharpen Details must change the image");
    }

    #[test]
    fn alpha_is_preserved_and_reapply_is_deterministic() {
        let mut base = PixelBuffer::new(24, 20, 4);
        let n = base.pixel_count();
        let mut state = 3u64;
        for i in 0..n * 4 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            base.data[i] = (state >> 40) as u8;
        }
        let alpha = base.data[3 * n..].to_vec();
        let p = ReduceNoiseParams {
            reduce_color_noise: 40.0,
            sharpen_details: 30.0,
            remove_jpeg_artifacts: true,
            channels: [ChannelDetail {
                strength: 7.0,
                preserve_details: 25.0,
            }; 3],
        };
        let mut a = base.clone();
        reduce_noise(&mut a, &p).unwrap();
        let mut b = base.clone();
        reduce_noise(&mut b, &p).unwrap();
        assert_eq!(a.data, b.data, "no RNG, so re-apply is byte-identical");
        assert_eq!(&a.data[3 * n..], &alpha[..], "alpha must be bit-identical");
    }

    #[test]
    fn out_of_range_is_rejected_untouched() {
        let base = noise(8, 8, 10, 1);
        let cases = [
            ReduceNoiseParams {
                reduce_color_noise: -1.0,
                ..ReduceNoiseParams::default()
            },
            ReduceNoiseParams {
                reduce_color_noise: 101.0,
                ..ReduceNoiseParams::default()
            },
            ReduceNoiseParams {
                sharpen_details: 101.0,
                ..ReduceNoiseParams::default()
            },
            ReduceNoiseParams {
                channels: [ChannelDetail {
                    strength: 11.0,
                    preserve_details: 0.0,
                }; 3],
                ..ReduceNoiseParams::default()
            },
            ReduceNoiseParams {
                channels: [ChannelDetail {
                    strength: 1.0,
                    preserve_details: 101.0,
                }; 3],
                ..ReduceNoiseParams::default()
            },
        ];
        for p in cases {
            let mut out = base.clone();
            assert!(reduce_noise(&mut out, &p).is_err(), "{p:?}");
            assert_eq!(out.data, base.data, "{p:?} mutated the buffer");
        }
    }

    #[test]
    fn tiny_images_do_not_panic() {
        let p = ReduceNoiseParams {
            reduce_color_noise: 50.0,
            sharpen_details: 50.0,
            remove_jpeg_artifacts: true,
            channels: [ChannelDetail {
                strength: 8.0,
                preserve_details: 0.0,
            }; 3],
        };
        for &(w, h) in &[(1u32, 1u32), (2, 2), (1, 9), (9, 1)] {
            let mut buf = noise(w, h, 20, 2);
            assert!(reduce_noise(&mut buf, &p).is_ok(), "{w}x{h}");
        }
    }
}
