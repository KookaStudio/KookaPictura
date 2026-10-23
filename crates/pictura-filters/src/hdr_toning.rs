//! HDR Toning (Local Adaptation): a neighborhood tone-mapping operator.
//!
//! Ported from perfecto25/photorust's `Document::apply_hdr_toning`. A Gaussian
//! blur of the colour planes supplies the local base layer; the per-pixel
//! residual is the detail layer. Tone compression, exposure, shadow/highlight
//! recovery, a filmic shoulder/toe, and an HSL vibrance/saturation pass all run
//! in the log domain, then write back as an additive luminance delta so
//! chrominance is preserved. Alpha is never modified.
//!
//! Source: https://github.com/perfecto25/photorust
//!
//! Only the base's luminance is read, so one luminance plane is blurred, by
//! `pictura_core::blur::approx_gaussian` (three box passes): its cost does not
//! grow with the radius, where photorust leans on a GPU blur to stay usable.

use pictura_core::PixelBuffer;

use crate::{validate, FilterError};

pub use pictura_core::HdrToningParams;

fn validate_params(p: &HdrToningParams) -> Result<(), FilterError> {
    pictura_core::validate_hdr_toning_params(p).map_err(FilterError::InvalidParams)
}

/// Apply HDR Toning's Local Adaptation to `buf` (planar 8-bit; alpha untouched).
pub fn hdr_toning(buf: &mut PixelBuffer, p: &HdrToningParams) -> Result<(), FilterError> {
    let n = validate(buf)?;
    validate_params(p)?;
    let channels = buf.channels as usize;

    // photorust's blur treats its radius as the Gaussian sigma, so the preset
    // radii transfer unchanged.
    let luminance = |i: usize| {
        (0.299 * buf.data[i] as f64
            + 0.587 * buf.data[n + i] as f64
            + 0.114 * buf.data[2 * n + i] as f64)
            / 255.0
    };
    let mut local = (0..n).map(luminance).collect::<Vec<_>>();
    pictura_core::blur::approx_gaussian(
        &mut local,
        buf.width as usize,
        buf.height as usize,
        p.radius,
    );

    let opaque = |data: &[u8], i: usize| channels < 4 || data[3 * n + i] != 0;

    // Anchor tone compression on the image's log-average luminance (the key),
    // skipping the alpha=0 pixels photorust skips.
    const EPS: f64 = 1e-3;
    let pivot = {
        let mut sum = 0.0f64;
        let mut count = 0u64;
        for i in 0..n {
            if !opaque(&buf.data, i) {
                continue;
            }
            let l = (0.299 * buf.data[i] as f64
                + 0.587 * buf.data[n + i] as f64
                + 0.114 * buf.data[2 * n + i] as f64)
                / 255.0;
            sum += (l + EPS).ln();
            count += 1;
        }
        if count > 0 {
            sum / count as f64
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

    // Clamp so extreme gamma cannot blow the log deviation into exp() overflow.
    let compress = (1.0 / gamma).clamp(0.1, 4.0);
    // Strength modulates how much of the detail layer survives.
    let detail_scale = (1.0 + detail_amt * (strength * 0.5).min(1.0)).max(0.0);
    let exposure_ln = p.exposure.clamp(-5.0, 5.0) * std::f64::consts::LN_2;
    const DETAIL_KNEE: f64 = 0.06;

    let tone = |r: u8, g: u8, b: u8, local_lum: f64| -> [u8; 3] {
        let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;

        // 1. Split into base (blurred) and detail (residual) in the log domain.
        let log_lum = (lum + EPS).ln();
        let log_base = (local_lum + EPS).ln();
        let log_detail = log_lum - log_base;

        // 2. Shrink very small residuals: gating keeps genuine edges and drops
        //    the JPEG block-noise floor a uniform boost would expose.
        let t = (log_detail.abs() / DETAIL_KNEE).min(1.0);
        let gate = t * t * (3.0 - 2.0 * t);

        // 3. Compress the base around the key, scale the detail, add exposure.
        let base_dev = ((log_base - pivot) * compress).clamp(-8.0, 8.0);
        let log_out = pivot + base_dev + log_detail * gate * detail_scale + exposure_ln;
        let mut target = log_out.exp();

        // Shadow/highlight recovery.
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

        // 4. Filmic S-curve: soft shoulder + toe simulate 32-bit headroom, so
        //    out-of-range values compress smoothly instead of hard-clipping.
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

        // Additive delta preserves chrominance exactly.
        let delta = target - lum;
        let mut nr = (r + delta).clamp(0.0, 1.0);
        let mut ng = (g + delta).clamp(0.0, 1.0);
        let mut nb = (b + delta).clamp(0.0, 1.0);

        // 5. Vibrance + saturation in HSL, gated so near-gray pixels (unreliable
        //    hue) skip the boost and do not amplify JPEG noise.
        if vib_amt.abs() > 1e-4 || sat_amt.abs() > 1e-4 {
            let (h_hsl, mut s_hsl, l_hsl) = rgb_to_hsl(nr, ng, nb);
            let orig_s = s_hsl;

            if vib_amt.abs() > 1e-4 {
                s_hsl = (s_hsl + (1.0 - s_hsl) * vib_amt).clamp(0.0, 1.0);
            }
            if sat_amt >= 0.0 {
                s_hsl += (1.0 - s_hsl) * sat_amt;
            } else {
                s_hsl += s_hsl * sat_amt;
            }
            s_hsl = s_hsl.clamp(0.0, 1.0);

            let gate = (orig_s / 0.15).min(1.0);
            s_hsl = orig_s + (s_hsl - orig_s) * gate;

            let (r2, g2, b2) = hsl_to_rgb(h_hsl, s_hsl, l_hsl);
            nr = r2;
            ng = g2;
            nb = b2;
        }

        [to_u8(nr), to_u8(ng), to_u8(nb)]
    };

    // Every pixel is independent: split the planes into one band per core.
    let (colour, alpha) = buf.data.split_at_mut(3 * n);
    let alpha: &[u8] = if channels >= 4 { &alpha[..n] } else { &[] };
    let (red, rest) = colour.split_at_mut(n);
    let (green, blue) = rest.split_at_mut(n);
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get());
    let band = n.div_ceil(threads).max(4096);
    std::thread::scope(|scope| {
        let bands = red
            .chunks_mut(band)
            .zip(green.chunks_mut(band))
            .zip(blue.chunks_mut(band))
            .zip(local.chunks(band))
            .enumerate();
        for (index, (((red, green), blue), local)) in bands {
            let start = index * band;
            let tone = &tone;
            scope.spawn(move || {
                for k in 0..red.len() {
                    if alpha.get(start + k) == Some(&0) {
                        continue;
                    }
                    [red[k], green[k], blue[k]] = tone(red[k], green[k], blue[k], local[k]);
                }
            });
        }
    });
    Ok(())
}

fn to_u8(v: f64) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

// Ported from pictura-adjust's private `common` helpers (not public there).
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-12 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } * 60.0;
    (h.rem_euclid(360.0), s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s <= 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hk = h / 360.0;
    (
        hue2rgb(p, q, hk + 1.0 / 3.0),
        hue2rgb(p, q, hk),
        hue2rgb(p, q, hk - 1.0 / 3.0),
    )
}

fn hue2rgb(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_uniform() -> PixelBuffer {
        let (w, h) = (16u32, 16u32);
        let n = (w * h) as usize;
        let mut data = vec![0u8; n * 4];
        for i in 0..n {
            let v = if (i % w as usize) < 8 { 30u8 } else { 220 };
            data[i] = v;
            data[n + i] = v / 2;
            data[2 * n + i] = 255 - v;
            data[3 * n + i] = if i % 5 == 0 { 0 } else { 200 };
        }
        PixelBuffer {
            width: w,
            height: h,
            channels: 4,
            data: data.into(),
        }
    }

    #[test]
    fn changes_non_uniform_and_preserves_alpha() {
        let mut buf = non_uniform();
        let before = buf.data.to_vec();
        let alpha: Vec<u8> = buf.data[3 * buf.pixel_count()..].to_vec();
        hdr_toning(
            &mut buf,
            &HdrToningParams {
                radius: 2.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_ne!(
            buf.data.to_vec(),
            before,
            "a non-uniform buffer must change"
        );
        assert_eq!(
            &buf.data[3 * buf.pixel_count()..],
            alpha.as_slice(),
            "alpha is untouched"
        );
    }

    #[test]
    fn rejects_out_of_range_params() {
        let mut buf = non_uniform();
        let bad = [
            HdrToningParams {
                radius: 0.0,
                ..Default::default()
            },
            HdrToningParams {
                radius: 501.0,
                ..Default::default()
            },
            HdrToningParams {
                strength: 0.0,
                ..Default::default()
            },
            HdrToningParams {
                gamma: 10.0,
                ..Default::default()
            },
            HdrToningParams {
                exposure: 6.0,
                ..Default::default()
            },
            HdrToningParams {
                detail: 301.0,
                ..Default::default()
            },
            HdrToningParams {
                saturation: -101.0,
                ..Default::default()
            },
        ];
        for p in bad {
            assert!(
                matches!(hdr_toning(&mut buf, &p), Err(FilterError::InvalidParams(_))),
                "expected refusal for {p:?}"
            );
        }
    }
}
