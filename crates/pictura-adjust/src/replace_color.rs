//! Image > Adjustments > Replace Color (issue #164).
//!
//! A pointwise HSL shift feathered by a colour-match weight: each pixel's
//! weight is the best Chebyshev-distance match to any sample, optionally
//! localised around where the colour was picked. The kernel mirrors
//! photorust's Replace Color. Selection confinement is applied by the
//! renderer's coverage mask, so it is not modelled here; native-depth apply
//! refuses the operation (the [`crate::native`] catch-all).

use pictura_core::PixelBuffer;
use rayon::prelude::*;

use crate::common::{hsl_to_rgb, note_parallel, planes_mut, rgb_to_hsl, serial};
use crate::types::AdjustError;

/// One eyedropper reading: the document position it was taken at (negative
/// when it was not read off the canvas) plus its RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceColorSample {
    pub x: i32,
    pub y: i32,
    pub rgb: [u8; 3],
}

/// Replace Color's full parameter set. `fuzziness` is `0..=200`; `hue` is
/// degrees `-180..=180`; `saturation` / `lightness` are `-100..=100`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceColorParams {
    pub samples: Vec<ReplaceColorSample>,
    pub fuzziness: f64,
    pub localized: bool,
    pub hue: f64,
    pub saturation: f64,
    pub lightness: f64,
}

/// Spatial falloff width for Localized Color Clusters, as a fraction of the
/// image diagonal, squared.
pub(crate) fn sigma_sq(w: u32, h: u32) -> f64 {
    let diag = ((w as f64) * (w as f64) + (h as f64) * (h as f64))
        .sqrt()
        .max(1.0);
    let sigma = diag * 0.18;
    sigma * sigma
}

/// Selection weight for one pixel: the best match over all samples, so adding
/// a sample widens the selection rather than averaging it away.
#[allow(clippy::too_many_arguments)]
pub(crate) fn match_weight(
    r: u8,
    g: u8,
    b: u8,
    x: i32,
    y: i32,
    samples: &[ReplaceColorSample],
    fuzziness: f64,
    localized: bool,
    sigma: f64,
) -> f64 {
    let mut best = 0.0f64;
    for s in samples {
        let d = (r as i32 - s.rgb[0] as i32)
            .abs()
            .max((g as i32 - s.rgb[1] as i32).abs())
            .max((b as i32 - s.rgb[2] as i32).abs()) as f64;
        let mut w = if fuzziness <= 0.0 {
            if d == 0.0 {
                1.0
            } else {
                0.0
            }
        } else {
            (1.0 - d / fuzziness).clamp(0.0, 1.0)
        };
        // A sample with no canvas position (the foreground colour the dialog
        // opens on) has no cluster to stay near.
        if localized && w > 0.0 && s.x >= 0 && s.y >= 0 {
            let dx = (x - s.x) as f64;
            let dy = (y - s.y) as f64;
            w *= (-(dx * dx + dy * dy) / (2.0 * sigma)).exp();
        }
        best = best.max(w);
    }
    best
}

fn check_params(p: &ReplaceColorParams) -> Result<(), AdjustError> {
    if !p.fuzziness.is_finite() || !(0.0..=200.0).contains(&p.fuzziness) {
        return Err(AdjustError::InvalidParams(
            "fuzziness must be 0..=200".into(),
        ));
    }
    if !p.hue.is_finite() || !(-180.0..=180.0).contains(&p.hue) {
        return Err(AdjustError::InvalidParams("hue must be -180..=180".into()));
    }
    if !p.saturation.is_finite() || !(-100.0..=100.0).contains(&p.saturation) {
        return Err(AdjustError::InvalidParams(
            "saturation must be -100..=100".into(),
        ));
    }
    if !p.lightness.is_finite() || !(-100.0..=100.0).contains(&p.lightness) {
        return Err(AdjustError::InvalidParams(
            "lightness must be -100..=100".into(),
        ));
    }
    Ok(())
}

/// One pixel (`0.0..=1.0` RGB) through the replacement shift, as Photoshop's
/// Hue/Saturation does it: the hue turns in HSL; saturation scales the
/// pixel's chroma about its HSL lightness (by `1 / (1 - sat)` raising, by
/// `1 + sat` lowering), so a near-gray pixel stays near gray instead of its
/// JPEG noise blowing up into vivid blocks, and a pure gray cannot take a
/// colour; lightness then blends each channel toward white or black.
/// ponytail: the reverse-engineered model in common use; Photoshop's own
/// kernel is closed.
pub(crate) fn shift(rgb: [f64; 3], hue: f64, sat: f64, light: f64) -> [f64; 3] {
    let (h, s, l) = rgb_to_hsl(rgb[0], rgb[1], rgb[2]);
    let (r, g, b) = hsl_to_rgb((h + hue).rem_euclid(360.0), s, l);
    let gain = if sat >= 0.0 {
        1.0 / (1.0 - sat).max(1e-3)
    } else {
        1.0 + sat
    };
    [r, g, b].map(|c| {
        let c = (l + (c - l) * gain).clamp(0.0, 1.0);
        if light >= 0.0 {
            c + (1.0 - c) * light
        } else {
            c * (1.0 + light)
        }
    })
}

/// The colour `sample` becomes at `hue` degrees and `saturation` /
/// `lightness` percent: the dialog's Result swatch.
pub fn replace_color_result(sample: [u8; 3], hue: f64, saturation: f64, lightness: f64) -> [u8; 3] {
    shift(
        sample.map(|v| f64::from(v) / 255.0),
        hue,
        saturation / 100.0,
        lightness / 100.0,
    )
    .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The `(hue, saturation, lightness)` shift (degrees, percent, percent) that
/// takes `sample` closest to `result`: the inverse of
/// [`replace_color_result`] for the dialog's Result colour picker. A gray
/// sample cannot take a hue or chroma, so only its lightness moves.
pub fn replace_color_shift_for(sample: [u8; 3], result: [u8; 3]) -> (f64, f64, f64) {
    let unit = |c: [u8; 3]| c.map(|v| f64::from(v) / 255.0);
    let chroma = |c: [f64; 3]| c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);
    let (from, to) = (unit(sample), unit(result));
    let (h0, _, l0) = rgb_to_hsl(from[0], from[1], from[2]);
    let (h1, _, l1) = rgb_to_hsl(to[0], to[1], to[2]);
    // Lightness moves every channel (and so L) linearly toward white or black.
    let light = if l1 >= l0 {
        if l0 >= 1.0 {
            0.0
        } else {
            (l1 - l0) / (1.0 - l0)
        }
    } else if l0 <= 0.0 {
        0.0
    } else {
        l1 / l0 - 1.0
    };
    let (c0, c1) = (chroma(from), chroma(to));
    // The chroma gain before the lightness blend, which scales chroma by
    // `1 - |light|`.
    let room = 1.0 - light.abs();
    let (hue, sat) = if c0 <= 0.0 || c1 <= 0.0 || room <= 0.0 {
        (0.0, if c1 <= 0.0 && c0 > 0.0 { -1.0 } else { 0.0 })
    } else {
        let gain = c1 / (c0 * room);
        let sat = if gain >= 1.0 {
            1.0 - 1.0 / gain
        } else {
            gain - 1.0
        };
        ((h1 - h0 + 540.0).rem_euclid(360.0) - 180.0, sat)
    };
    (
        hue,
        (sat * 100.0).clamp(-100.0, 100.0),
        (light * 100.0).clamp(-100.0, 100.0),
    )
}

/// Apply Replace Color in place to a planar buffer (alpha untouched). `n` is
/// the pixel count from [`crate::common::validate`]. Empty samples is a no-op.
pub(crate) fn replace_color(
    p: &ReplaceColorParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    check_params(p)?;
    if p.samples.is_empty() {
        return Ok(());
    }
    let sigma = sigma_sq(buf.width, buf.height);
    let width = buf.width as usize;
    let (r, g, b) = planes_mut(buf, n);
    let run = |y: usize, r: &mut [u8], g: &mut [u8], b: &mut [u8]| {
        for x in 0..width {
            let (rv, gv, bv) = (r[x], g[x], b[x]);
            let weight = match_weight(
                rv,
                gv,
                bv,
                x as i32,
                y as i32,
                &p.samples,
                p.fuzziness,
                p.localized,
                sigma,
            );
            if weight <= 0.0 {
                continue;
            }
            let [nr, ng, nb] = shift(
                [rv, gv, bv].map(|v| v as f64 / 255.0),
                p.hue,
                p.saturation / 100.0,
                p.lightness / 100.0,
            );
            let blend = |orig: u8, new: f64| -> u8 {
                let o = orig as f64 / 255.0;
                ((o + (new - o) * weight).clamp(0.0, 1.0) * 255.0 + 0.5) as u8
            };
            r[x] = blend(rv, nr);
            g[x] = blend(gv, ng);
            b[x] = blend(bv, nb);
        }
    };
    if serial() {
        for (y, ((rc, gc), bc)) in r
            .chunks_mut(width)
            .zip(g.chunks_mut(width))
            .zip(b.chunks_mut(width))
            .enumerate()
        {
            run(y, rc, gc, bc);
        }
        return Ok(());
    }
    note_parallel();
    r.par_chunks_mut(width)
        .zip(g.par_chunks_mut(width))
        .zip(b.par_chunks_mut(width))
        .enumerate()
        .for_each(|(y, ((rc, gc), bc))| run(y, rc, gc, bc));
    Ok(())
}

/// The Replace Color selection mask, fitted into a `size` box for the dialog's
/// preview. White is fully selected, black not. A one-channel buffer; empty
/// for degenerate input.
pub fn replace_color_mask(buf: &PixelBuffer, p: &ReplaceColorParams, size: usize) -> PixelBuffer {
    let (sw, sh) = (buf.width as usize, buf.height as usize);
    if sw == 0 || sh == 0 || size == 0 {
        return PixelBuffer {
            width: 0,
            height: 0,
            channels: 1,
            data: Vec::new().into(),
        };
    }
    let scale = (size as f64 / sw as f64).min(size as f64 / sh as f64);
    let tw = ((sw as f64 * scale).round() as usize).max(1);
    let th = ((sh as f64 * scale).round() as usize).max(1);
    let sigma = sigma_sq(buf.width, buf.height);
    let plane = sw * sh;
    let mut out = vec![0u8; tw * th];
    for y in 0..th {
        let sy = ((y as f64 / scale) as usize).min(sh - 1);
        for x in 0..tw {
            let sx = ((x as f64 / scale) as usize).min(sw - 1);
            let at = sy * sw + sx;
            let transparent = buf.channels == 4 && buf.data[3 * plane + at] == 0;
            let w = if transparent {
                0.0
            } else {
                match_weight(
                    buf.data[at],
                    buf.data[plane + at],
                    buf.data[2 * plane + at],
                    sx as i32,
                    sy as i32,
                    &p.samples,
                    p.fuzziness,
                    p.localized,
                    sigma,
                )
            };
            out[y * tw + x] = (w * 255.0 + 0.5) as u8;
        }
    }
    PixelBuffer {
        width: tw as u32,
        height: th as u32,
        channels: 1,
        data: out.into(),
    }
}
