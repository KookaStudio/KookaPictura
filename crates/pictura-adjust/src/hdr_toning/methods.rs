//! The parameter-free HDR Conversion methods and the fixed Exposure & Gamma
//! operator.
//!
//! Equalize Histogram and Highlight Compression are inferred reconstructions of
//! closed CS6 kernels: the first applies a luminance CDF gain, the second rolls
//! values above a fixed knee toward white. The spec only claims their observable
//! behavior, not byte parity.

use crate::AdjustError;

use super::ExposureGamma;

/// Rec. 601 luminance of pixel `i` in a planar 1- or 3-channel buffer.
pub(super) fn pixel_luma(samples: &[f32], i: usize, n: usize, channels: usize) -> f64 {
    if channels == 1 {
        return f64::from(samples[i]);
    }
    let r = f64::from(samples[i]);
    let g = f64::from(samples[n + i]);
    let b = f64::from(samples[2 * n + i]);
    0.299 * r + 0.587 * g + 0.114 * b
}

/// Apply Exposure & Gamma to linear-light `f32` samples.
///
/// `gain = 2^exposure_ev`; each output sample is
/// `sign(x·gain)·|x·gain|^(1/gamma)`. This is the documented
/// `(x·2^EV)^(1/gamma)` on the non-negative domain, keeps values above `1.0`
/// unclamped, maps `0.0` to `0.0`, and preserves the sign of a negative sample.
///
/// Returns [`AdjustError::InvalidParams`] when `exposure_ev` is not finite, or
/// `gamma` is not finite / not strictly positive.
pub fn exposure_gamma(samples: &[f32], params: ExposureGamma) -> Result<Vec<f32>, AdjustError> {
    if !params.exposure_ev.is_finite() {
        return Err(AdjustError::InvalidParams("exposure must be finite".into()));
    }
    if !params.gamma.is_finite() || params.gamma <= 0.0 {
        return Err(AdjustError::InvalidParams(
            "gamma must be finite and > 0".into(),
        ));
    }
    let gain = 2f64.powf(params.exposure_ev);
    if !gain.is_finite() {
        return Err(AdjustError::InvalidParams(
            "exposure overflows the gain".into(),
        ));
    }
    let inv_gamma = 1.0 / params.gamma;
    Ok(samples
        .iter()
        .map(|&x| {
            let g = f64::from(x) * gain;
            (g.signum() * g.abs().powf(inv_gamma)) as f32
        })
        .collect())
}

/// Equalize Histogram: map each pixel's luminance through its own CDF and apply
/// the resulting gain to the colour planes. A uniform input is returned
/// unchanged so the zero-span case cannot divide by zero.
pub(super) fn equalize_histogram(
    samples: &[f32],
    n: usize,
    channels: usize,
) -> Result<Vec<f32>, AdjustError> {
    let mut lums = vec![0.0f64; n];
    let mut positive = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for (i, lum) in lums.iter_mut().enumerate() {
        let value = pixel_luma(samples, i, n, channels);
        if !value.is_finite() || value <= 0.0 {
            continue;
        }
        *lum = value;
        positive += 1;
        min = min.min(value);
        max = max.max(value);
    }
    if positive < 2 || max - min < 1e-9 {
        return Ok(samples.to_vec());
    }

    const BINS: usize = 4096;
    let span = max - min;
    let bin = |l: f64| {
        (((l - min) / span) * (BINS - 1) as f64)
            .round()
            .clamp(0.0, (BINS - 1) as f64) as usize
    };
    let mut hist = vec![0u32; BINS];
    for &lum in &lums {
        if lum > 0.0 {
            hist[bin(lum)] += 1;
        }
    }
    let mut cdf = vec![0.0f64; BINS];
    let mut acc = 0u32;
    for (i, count) in hist.iter().enumerate() {
        acc += count;
        cdf[i] = f64::from(acc) / positive as f64;
    }

    let mut out = samples.to_vec();
    for (i, &old) in lums.iter().enumerate() {
        if old <= 0.0 {
            continue;
        }
        let gain = cdf[bin(old)] / old;
        for c in 0..channels {
            let at = c * n + i;
            out[at] = (f64::from(samples[at]) * gain).clamp(0.0, 1.0) as f32;
        }
    }
    Ok(out)
}

/// Highlight Compression: leave luminance at or below `0.8` unchanged and roll
/// values above the knee asymptotically toward `1.0`.
pub(super) fn highlight_compression(
    samples: &[f32],
    n: usize,
    channels: usize,
) -> Result<Vec<f32>, AdjustError> {
    const KNEE: f64 = 0.8;
    // The exponential shoulder reaches 1.0 faster than f32 can distinguish it
    // from the next value down, so cap at the largest f32 below white; the
    // spec requires a highlight above white to stay below 1.0.
    const BELOW_WHITE: f64 = f32::from_bits(1.0f32.to_bits() - 1) as f64;
    let mut out = samples.to_vec();
    for i in 0..n {
        let old = pixel_luma(samples, i, n, channels).max(0.0);
        if old <= KNEE {
            continue;
        }
        let new =
            (KNEE + (1.0 - KNEE) * (1.0 - (-(old - KNEE) / (1.0 - KNEE)).exp())).min(BELOW_WHITE);
        let gain = new / old;
        for c in 0..channels {
            let at = c * n + i;
            out[at] = (f64::from(samples[at]) * gain).clamp(0.0, 1.0) as f32;
        }
    }
    Ok(out)
}
