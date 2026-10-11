//! Shared 1-D kernel helpers for the filter family.
//!
//! The UI radius maps to a Gaussian with the 3σ support convention from
//! `FILT-010`; every spatial filter builds its weights here.

use pictura_core::PixelBuffer;
use rand_chacha::{rand_core::RngCore, ChaCha8Rng};
use rayon::prelude::*;

/// One uniform `f64` in `[0, 1)` from 53 random bits.
pub(crate) fn unit_f64(rng: &mut ChaCha8Rng) -> f64 {
    (rng.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// UI radius is the 3σ support per `FILT-010`; floor σ at 0.1.
pub fn sigma_from_radius(radius: f64) -> f64 {
    (radius / 3.0).max(0.1)
}

/// Normalized 1-D FIR kernel, support `⌈3σ⌉` each side.
pub fn gaussian_kernel(sigma: f64) -> Vec<f64> {
    if sigma <= 0.0 {
        return vec![1.0];
    }
    let support = (3.0 * sigma).ceil() as isize;
    let size = (2 * support + 1) as usize;
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut weights = Vec::with_capacity(size);
    let mut sum = 0.0;
    for i in -support..=support {
        let x = i as f64;
        let w = (-(x * x) / two_sigma_sq).exp();
        sum += w;
        weights.push(w);
    }
    for w in &mut weights {
        *w /= sum;
    }
    weights
}

/// Clamp-to-edge index access (border policy per `FILT-010`).
pub fn clamp_index(i: isize, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    i.clamp(0, n as isize - 1) as usize
}

pub(crate) fn to_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Separable Gaussian blur of one `w`×`h` plane, clamp-to-edge.
pub(crate) fn blur_plane(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    let kernel: Vec<f32> = gaussian_kernel(f64::from(sigma))
        .into_iter()
        .map(|k| k as f32)
        .collect();
    let support = (kernel.len() / 2) as isize;
    let mut across = vec![0f32; w * h];
    across.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let line = &src[y * w..y * w + w];
        for (x, out) in row.iter_mut().enumerate() {
            *out = kernel
                .iter()
                .enumerate()
                .map(|(k, &kw)| kw * line[clamp_index(x as isize + k as isize - support, w)])
                .sum();
        }
    });
    let mut out = vec![0f32; w * h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            *o = kernel
                .iter()
                .enumerate()
                .map(|(k, &kw)| {
                    kw * across[clamp_index(y as isize + k as isize - support, h) * w + x]
                })
                .sum();
        }
    });
    out
}

/// Separable Gaussian blur of the color planes of a 3/4-channel planar buffer
/// (alpha untouched), clamp-to-edge. Shared by Gaussian Blur and Unsharp Mask.
pub fn gaussian_blur_planes(buf: &mut PixelBuffer, sigma: f64) {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = w * h;
    if sigma <= 0.0 || n == 0 {
        return;
    }
    let kernel: Vec<f32> = gaussian_kernel(sigma).iter().map(|&kw| kw as f32).collect();
    let support = kernel.len() / 2;
    let planes = (buf.channels as usize).min(3);
    // Each pass accumulates a whole row one tap at a time, so it vectorizes;
    // every pixel still sums its taps in kernel order from zero, so the result
    // is byte-identical to a per-pixel serial loop (`blur_matches_the_per_pixel_sum`).
    for c in 0..planes {
        let base = c * n;
        let mut across = vec![0f32; n];
        let src = &buf.data[base..base + n];
        across.par_chunks_mut(w).enumerate().for_each_init(
            || vec![0f32; w + 2 * support],
            |padded, (y, row)| {
                let line = &src[y * w..y * w + w];
                for (i, p) in padded.iter_mut().enumerate() {
                    *p = f32::from(line[clamp_index(i as isize - support as isize, w)]);
                }
                for (k, &kw) in kernel.iter().enumerate() {
                    for (out, &v) in row.iter_mut().zip(&padded[k..k + w]) {
                        *out += kw * v;
                    }
                }
            },
        );
        buf.data[base..base + n]
            .par_chunks_mut(w)
            .enumerate()
            .for_each_init(
                || vec![0f32; w],
                |acc, (y, row)| {
                    acc.fill(0.0);
                    for (k, &kw) in kernel.iter().enumerate() {
                        let sy = clamp_index((y + k) as isize - support as isize, h);
                        for (a, &v) in acc.iter_mut().zip(&across[sy * w..sy * w + w]) {
                            *a += kw * v;
                        }
                    }
                    for (out, &a) in row.iter_mut().zip(acc.iter()) {
                        *out = a.round().clamp(0.0, 255.0) as u8;
                    }
                },
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-pixel serial blur the row-wise passes replaced.
    fn per_pixel_blur(buf: &mut PixelBuffer, sigma: f64) {
        let (w, h) = (buf.width as usize, buf.height as usize);
        let n = w * h;
        let kernel: Vec<f32> = gaussian_kernel(sigma).iter().map(|&k| k as f32).collect();
        let support = (kernel.len() / 2) as isize;
        for c in 0..(buf.channels as usize).min(3) {
            let src = buf.data[c * n..c * n + n].to_vec();
            let mut tmp = vec![0f32; n];
            for y in 0..h {
                for x in 0..w {
                    let mut acc = 0f32;
                    for (k, &kw) in kernel.iter().enumerate() {
                        let sx = clamp_index(x as isize + k as isize - support, w);
                        acc += kw * src[y * w + sx] as f32;
                    }
                    tmp[y * w + x] = acc;
                }
            }
            for y in 0..h {
                for x in 0..w {
                    let mut acc = 0f32;
                    for (k, &kw) in kernel.iter().enumerate() {
                        let sy = clamp_index(y as isize + k as isize - support, h);
                        acc += kw * tmp[sy * w + x];
                    }
                    buf.data[c * n + y * w + x] = acc.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    #[test]
    fn blur_matches_the_per_pixel_sum() {
        for (w, h) in [(37u32, 23u32), (3, 41), (1, 1), (64, 2)] {
            let mut buf = PixelBuffer::new(w, h, 4);
            for (i, v) in buf.data.as_mut_slice().iter_mut().enumerate() {
                *v = ((i * 7919) ^ (i >> 3)) as u8;
            }
            for sigma in [0.1, 0.4, 1.3, 4.0, 11.0] {
                let mut fast = buf.clone();
                let mut slow = buf.clone();
                gaussian_blur_planes(&mut fast, sigma);
                per_pixel_blur(&mut slow, sigma);
                assert_eq!(fast.data, slow.data, "{w}x{h} sigma {sigma}");
            }
        }
    }

    #[test]
    fn sigma_floor() {
        assert_eq!(sigma_from_radius(0.0), 0.1);
        assert_eq!(sigma_from_radius(0.3), 0.1);
        assert!((sigma_from_radius(3.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn kernel_is_normalized_and_symmetric() {
        let k = gaussian_kernel(1.0);
        assert_eq!(k.len(), 7);
        let sum: f64 = k.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!((k[0] - k[6]).abs() < 1e-12);
    }

    #[test]
    fn non_positive_sigma_is_single_sample() {
        assert_eq!(gaussian_kernel(0.0), vec![1.0]);
    }

    #[test]
    fn clamp_index_clamps_to_edges() {
        assert_eq!(clamp_index(-5, 4), 0);
        assert_eq!(clamp_index(2, 4), 2);
        assert_eq!(clamp_index(99, 4), 3);
        assert_eq!(clamp_index(0, 0), 0);
    }

    #[test]
    fn gaussian_blur_planes_is_noop_on_uniform_and_preserves_alpha() {
        let mut buf = PixelBuffer {
            width: 3,
            height: 1,
            channels: 4,
            data: vec![50, 50, 50, 100, 100, 100, 150, 150, 150, 10, 20, 30].into(),
        };
        gaussian_blur_planes(&mut buf, 1.0);
        assert_eq!(&buf.data[0..3], &[50, 50, 50]);
        assert_eq!(&buf.data[3..6], &[100, 100, 100]);
        assert_eq!(&buf.data[6..9], &[150, 150, 150]);
        assert_eq!(&buf.data[9..12], &[10, 20, 30], "alpha untouched");
    }
}
