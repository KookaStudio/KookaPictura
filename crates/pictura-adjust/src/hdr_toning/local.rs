//! Local Adaptation on unbounded f32 samples.
//!
//! Ported from the 8-bit HDR Toning filter: a Gaussian blur of the luminance
//! supplies the local base, the log residual is the detail layer, and the base
//! is compressed around the log-average pivot with a filmic shoulder. Unlike
//! the 8-bit filter, the input is not clamped to `1.0` before the tone map; the
//! output is an LDR result, so the post-tone channels are clamped to `[0, 1]`.
//!
//! `ponytail:` negative HDR samples are treated as zero; the 8-bit filter has
//! no negative path and HDR light is non-negative.

use pictura_core::{blur::approx_gaussian, validate_hdr_toning_params, HdrToningParams};

use super::methods::pixel_luma;
use crate::AdjustError;

pub(super) fn local_adaptation(
    samples: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    p: &HdrToningParams,
) -> Result<Vec<f32>, AdjustError> {
    validate_hdr_toning_params(p).map_err(AdjustError::InvalidParams)?;
    let n = width
        .checked_mul(height)
        .ok_or_else(|| AdjustError::InvalidParams("HDR tone-map size overflows".into()))?;
    let color_len = n
        .checked_mul(channels)
        .ok_or_else(|| AdjustError::InvalidParams("HDR tone-map size overflows".into()))?;

    let mut colors = samples[..color_len].to_vec();
    for value in &mut colors {
        if *value < 0.0 {
            *value = 0.0;
        }
    }

    let mut local: Vec<f64> = (0..n)
        .map(|i| pixel_luma(&colors, i, n, channels))
        .collect();
    approx_gaussian(&mut local, width, height, p.radius);

    const EPS: f64 = 1e-3;
    const DETAIL_KNEE: f64 = 0.06;
    let pivot = {
        let mut sum = 0.0f64;
        for i in 0..n {
            sum += (pixel_luma(&colors, i, n, channels) + EPS).ln();
        }
        if n > 0 {
            sum / n as f64
        } else {
            0.18f64.ln()
        }
    };

    let strength = p.strength.clamp(0.01, 4.0);
    let gamma = p.gamma.clamp(0.01, 9.99);
    let detail_amt = p.detail.clamp(-100.0, 300.0) / 100.0;
    let shadow_amt = p.shadow.clamp(-100.0, 100.0) / 100.0;
    let highlight_amt = p.highlight.clamp(-100.0, 100.0) / 100.0;
    let vib_amt = p.vibrance.clamp(-100.0, 100.0) / 100.0;
    let sat_amt = p.saturation.clamp(-100.0, 100.0) / 100.0;

    let compress = (1.0 / gamma).clamp(0.1, 4.0);
    let detail_scale = (1.0 + detail_amt * (strength * 0.5).min(1.0)).max(0.0);
    let exposure_ln = p.exposure.clamp(-5.0, 5.0) * std::f64::consts::LN_2;

    let tone = |lum: f64, local_lum: f64| -> f64 {
        let log_lum = (lum + EPS).ln();
        let log_base = (local_lum + EPS).ln();
        let log_detail = log_lum - log_base;

        let t = (log_detail.abs() / DETAIL_KNEE).min(1.0);
        let gate = t * t * (3.0 - 2.0 * t);

        let base_dev = ((log_base - pivot) * compress).clamp(-8.0, 8.0);
        let log_out = pivot + base_dev + log_detail * gate * detail_scale + exposure_ln;
        let mut target = log_out.exp();

        if shadow_amt.abs() > 1e-4 {
            let st = (1.0 - target * 2.0).clamp(0.0, 1.0);
            let sw = st * st * (3.0 - 2.0 * st);
            target += sw * shadow_amt * 0.15;
        }
        if highlight_amt.abs() > 1e-4 {
            let ht = ((target - 0.5) * 2.0).clamp(0.0, 1.0);
            let hw = ht * ht * (3.0 - 2.0 * ht);
            target += hw * highlight_amt * 0.15;
        }

        if target > 1.0 {
            target = 1.0 - (-(target - 1.0) * 2.0).exp() * 0.15;
        } else if target > 0.85 {
            let t = (target - 0.85) / 0.15;
            target = 0.85 + 0.15 * (1.0 - (1.0 - t).powi(2));
        }
        if target < 0.0 {
            target = (-(-target) * 2.0).exp() * 0.05;
        } else if target < 0.10 {
            let t = target / 0.10;
            target = 0.10 * t * t;
        }
        target
    };

    let mut out = colors.clone();
    for i in 0..n {
        let (r, g, b) = if channels == 1 {
            let l = f64::from(colors[i]);
            (l, l, l)
        } else {
            (
                f64::from(colors[i]),
                f64::from(colors[n + i]),
                f64::from(colors[2 * n + i]),
            )
        };
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        let target = tone(lum, local[i]);
        let delta = target - lum;
        let mut nr = (r + delta).clamp(0.0, 1.0);
        let mut ng = (g + delta).clamp(0.0, 1.0);
        let mut nb = (b + delta).clamp(0.0, 1.0);

        if channels == 3 && (vib_amt.abs() > 1e-4 || sat_amt.abs() > 1e-4) {
            let (h, mut s, l) = crate::common::rgb_to_hsl(nr, ng, nb);
            let orig_s = s;
            if vib_amt.abs() > 1e-4 {
                s = (s + (1.0 - s) * vib_amt).clamp(0.0, 1.0);
            }
            if sat_amt >= 0.0 {
                s += (1.0 - s) * sat_amt;
            } else {
                s += s * sat_amt;
            }
            s = s.clamp(0.0, 1.0);
            let gate = (orig_s / 0.15).min(1.0);
            s = orig_s + (s - orig_s) * gate;
            let (r2, g2, b2) = crate::common::hsl_to_rgb(h, s, l);
            nr = r2;
            ng = g2;
            nb = b2;
        }

        if channels == 1 {
            out[i] = nr as f32;
        } else {
            out[i] = nr as f32;
            out[n + i] = ng as f32;
            out[2 * n + i] = nb as f32;
        }
    }
    Ok(out)
}
