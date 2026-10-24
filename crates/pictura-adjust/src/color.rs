use pictura_core::PixelBuffer;

use crate::common::{hsl_to_rgb, luma, planes_mut, rgb_to_hsl};
use crate::types::{
    AdjustError, BlackWhiteParams, ChannelMixerParams, ColorBalanceParams, HueSaturationParams,
    PhotoFilterParams, SelectiveColorMethod, SelectiveColorParams, SelectiveRange, VibranceParams,
};

// ---------------------------------------------------------------------------
// Colour adjustments
// ---------------------------------------------------------------------------

/// Reject out-of-range Master or range settings.
pub(crate) fn validate_hue_saturation(p: &HueSaturationParams) -> Result<(), AdjustError> {
    let triples = std::iter::once((p.hue, p.saturation, p.lightness))
        .chain(p.ranges.iter().map(|r| (r.hue, r.saturation, r.lightness)));
    for (hue, saturation, lightness) in triples {
        if !(-180..=180).contains(&hue) {
            return Err(AdjustError::InvalidParams("hue must be -180..=180".into()));
        }
        if !(-100..=100).contains(&saturation) {
            return Err(AdjustError::InvalidParams(
                "saturation must be -100..=100".into(),
            ));
        }
        if !(-100..=100).contains(&lightness) {
            return Err(AdjustError::InvalidParams(
                "lightness must be -100..=100".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn hue_saturation_is_identity(p: &HueSaturationParams) -> bool {
    std::iter::once((p.hue, p.saturation, p.lightness))
        .chain(p.ranges.iter().map(|r| (r.hue, r.saturation, r.lightness)))
        .all(|t| t == (0, 0, 0))
}

/// One pixel (`0.0..=1.0` RGB) through Hue/Saturation. Each colour range adds
/// its settings scaled by its weight at the pixel's hue.
/// ponytail: the range blend (additive, faded out toward gray by chroma so a
/// neutral is not read as red) approximates Photoshop's unpublished one; no
/// baseline carries a range edit.
pub(crate) fn hue_saturation_rgb(p: &HueSaturationParams, r: f64, g: f64, b: f64) -> [f64; 3] {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    let mut dh = p.hue as f64;
    let mut ds = p.saturation as f64 / 100.0;
    let mut dl = p.lightness as f64 / 100.0;
    if !p.ranges.is_empty() {
        let presence = ((r.max(g).max(b) - r.min(g).min(b)) * 4.0).min(1.0);
        for range in &p.ranges {
            let w = range.weight(h) * presence;
            dh += w * range.hue as f64;
            ds += w * range.saturation as f64 / 100.0;
            dl += w * range.lightness as f64 / 100.0;
        }
        ds = ds.clamp(-1.0, 1.0);
        dl = dl.clamp(-1.0, 1.0);
    }
    let h = (h + dh).rem_euclid(360.0);
    let s = (s * (1.0 + ds)).clamp(0.0, 1.0);
    let (nr, ng, nb) = hsl_to_rgb(h, s, l);
    // Lightness blends each channel toward white or black after the hue and
    // saturation move, as Photoshop does: scaling HSL L instead keeps S and
    // turns near-neutral dark noise into saturated patches.
    let lighten = |c: f64| {
        if dl >= 0.0 {
            c + dl * (1.0 - c)
        } else {
            c * (1.0 + dl)
        }
        .clamp(0.0, 1.0)
    };
    [lighten(nr), lighten(ng), lighten(nb)]
}

pub(crate) fn hue_saturation(
    p: &HueSaturationParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    validate_hue_saturation(p)?;
    if hue_saturation_is_identity(p) {
        return Ok(());
    }
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let [nr, ng, nb] = hue_saturation_rgb(
            p,
            *rv as f64 / 255.0,
            *gv as f64 / 255.0,
            *bv as f64 / 255.0,
        );
        *rv = (nr * 255.0).round().clamp(0.0, 255.0) as u8;
        *gv = (ng * 255.0).round().clamp(0.0, 255.0) as u8;
        *bv = (nb * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    Ok(())
}

pub(crate) fn vibrance(
    p: &VibranceParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    if !(-100..=100).contains(&p.vibrance) {
        return Err(AdjustError::InvalidParams(
            "vibrance must be -100..=100".into(),
        ));
    }
    if !(-100..=100).contains(&p.saturation) {
        return Err(AdjustError::InvalidParams(
            "saturation must be -100..=100".into(),
        ));
    }
    if p.vibrance == 0 && p.saturation == 0 {
        return Ok(());
    }
    let kv = p.vibrance as f64 / 100.0;
    let ks = p.saturation as f64 / 100.0;
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let (h, s, l) = rgb_to_hsl(*rv as f64 / 255.0, *gv as f64 / 255.0, *bv as f64 / 255.0);
        // Approximation: boost falls off as (1 - S) (the spec's `S*(1-S)` text
        // contradicts its own acceptance criteria) and is damped in the skin
        // hue band. The exact falloff/skin model is closed.
        let keep = 1.0 - 0.5 * skin_bump(h);
        // A neutral pixel has no hue to boost; HSL's H=0 would otherwise turn
        // it red. The 1-S falloff is the documented approximation.
        let s = if s == 0.0 {
            0.0
        } else {
            (s + kv * (1.0 - s) * keep).clamp(0.0, 1.0)
        };
        let s = (s * (1.0 + ks)).clamp(0.0, 1.0);
        let (nr, ng, nb) = hsl_to_rgb(h, s, l);
        *rv = (nr * 255.0).round().clamp(0.0, 255.0) as u8;
        *gv = (ng * 255.0).round().clamp(0.0, 255.0) as u8;
        *bv = (nb * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    Ok(())
}

/// Smooth bump over the orange/red skin band (~30° centre, 30° half-width).
pub(crate) fn skin_bump(h: f64) -> f64 {
    let d = ((h - 30.0 + 540.0).rem_euclid(360.0) - 180.0).abs();
    if d >= 30.0 {
        0.0
    } else {
        0.5 * (1.0 + (std::f64::consts::PI * d / 30.0).cos())
    }
}

pub(crate) fn color_balance(
    p: &ColorBalanceParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    for band in [&p.shadows, &p.midtones, &p.highlights] {
        if band.iter().any(|v| !v.is_finite() || v.abs() > 100.0) {
            return Err(AdjustError::InvalidParams(
                "color balance values must be -100..=100".into(),
            ));
        }
    }
    if p.shadows == [0.0; 3] && p.midtones == [0.0; 3] && p.highlights == [0.0; 3] {
        return Ok(());
    }
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let (ir, ig, ib) = (*rv as f64, *gv as f64, *bv as f64);
        let y = luma(ir, ig, ib) / 255.0;
        // Overlapping parabola windows: shadow=1 at Y=0, highlight=1 at Y=1,
        // midtone peaks at Y=0.5. The exact falloffs are closed.
        let sh = (1.0 - y) * (1.0 - y);
        let hi = y * y;
        let mid = (1.0 - sh - hi).max(0.0);
        let w = [sh, mid, hi];
        let mut out = [ir, ig, ib];
        for (c, o) in out.iter_mut().enumerate() {
            let delta = w[0] * p.shadows[c] + w[1] * p.midtones[c] + w[2] * p.highlights[c];
            *o = (*o + delta).clamp(0.0, 255.0);
        }
        if p.preserve_luminosity {
            let y1 = luma(out[0], out[1], out[2]);
            if y1 > 0.0 {
                let scale = luma(ir, ig, ib) / y1;
                for o in &mut out {
                    *o = (*o * scale).clamp(0.0, 255.0);
                }
            }
        }
        *rv = out[0].round() as u8;
        *gv = out[1].round() as u8;
        *bv = out[2].round() as u8;
    }
    Ok(())
}

pub(crate) fn black_white(
    p: &BlackWhiteParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    let weights = [p.red, p.yellow, p.green, p.cyan, p.blue, p.magenta];
    if weights
        .iter()
        .any(|v| !v.is_finite() || !(-200.0..=300.0).contains(v))
    {
        return Err(AdjustError::InvalidParams(
            "black & white weights must be -200..=300 percent".into(),
        ));
    }
    if p.tint && p.tint_color == [0, 0, 0] {
        // A black tint is a no-op; avoid tinting everything to black.
        return Err(AdjustError::InvalidParams(
            "tint colour must not be black".into(),
        ));
    }
    let (rw, yw, gw, cw, bw, mw) = (
        p.red / 100.0,
        p.yellow / 100.0,
        p.green / 100.0,
        p.cyan / 100.0,
        p.blue / 100.0,
        p.magenta / 100.0,
    );
    let tint_hsl = if p.tint {
        let (h, s, _) = rgb_to_hsl(
            p.tint_color[0] as f64 / 255.0,
            p.tint_color[1] as f64 / 255.0,
            p.tint_color[2] as f64 / 255.0,
        );
        Some((h, s))
    } else {
        None
    };
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let (mut cr, mut cg, mut cb) = (*rv as f64, *gv as f64, *bv as f64);
        let neutral = cr.min(cg).min(cb);
        cr -= neutral;
        cg -= neutral;
        cb -= neutral;
        let mut gray = neutral;
        if cr == 0.0 {
            let cyan = cg.min(cb);
            cg -= cyan;
            cb -= cyan;
            gray += cyan * cw + cg * gw + cb * bw;
        } else if cg == 0.0 {
            let magenta = cr.min(cb);
            cr -= magenta;
            cb -= magenta;
            gray += magenta * mw + cr * rw + cb * bw;
        } else {
            let yellow = cr.min(cg);
            cr -= yellow;
            cg -= yellow;
            gray += yellow * yw + cr * rw + cg * gw;
        }
        let out = gray.round().clamp(0.0, 255.0) as u8;
        let (nr, ng, nb) = match tint_hsl {
            Some((h, s)) => {
                let (tr, tg, tb) = hsl_to_rgb(h, s, out as f64 / 255.0);
                (
                    (tr * 255.0).round() as u8,
                    (tg * 255.0).round() as u8,
                    (tb * 255.0).round() as u8,
                )
            }
            None => (out, out, out),
        };
        *rv = nr;
        *gv = ng;
        *bv = nb;
    }
    Ok(())
}

pub(crate) fn photo_filter(
    p: &PhotoFilterParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    if !p.density.is_finite() || !(0.0..=100.0).contains(&p.density) {
        return Err(AdjustError::InvalidParams(
            "photo filter density must be 0..=100".into(),
        ));
    }
    if p.density == 0.0 {
        return Ok(());
    }
    let d = p.density / 100.0;
    let f = [p.color[0] as f64, p.color[1] as f64, p.color[2] as f64];
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let (ir, ig, ib) = (*rv as f64, *gv as f64, *bv as f64);
        let mut out = [
            ir * (1.0 - d) + f[0] * d,
            ig * (1.0 - d) + f[1] * d,
            ib * (1.0 - d) + f[2] * d,
        ];
        if p.preserve_luminosity {
            let y1 = luma(out[0], out[1], out[2]);
            if y1 > 0.0 {
                let scale = luma(ir, ig, ib) / y1;
                for o in &mut out {
                    *o = (*o * scale).clamp(0.0, 255.0);
                }
            }
        }
        *rv = out[0].round().clamp(0.0, 255.0) as u8;
        *gv = out[1].round().clamp(0.0, 255.0) as u8;
        *bv = out[2].round().clamp(0.0, 255.0) as u8;
    }
    Ok(())
}

pub(crate) fn channel_mixer(
    p: &ChannelMixerParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    let mixes = [&p.red, &p.green, &p.blue];
    if mixes
        .iter()
        .chain(std::iter::once(&&p.constant))
        .any(|m| m.iter().any(|v| !v.is_finite() || v.abs() > 200.0))
    {
        return Err(AdjustError::InvalidParams(
            "channel mixer weights must be -200..=200 percent".into(),
        ));
    }
    let (r, g, b) = planes_mut(buf, n);
    if p.monochrome {
        // Monochrome applies one source mix to every output; use the Red row.
        let w = p.red;
        let c = p.constant[0] / 100.0 * 255.0;
        for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
            let (ir, ig, ib) = (*rv as f64, *gv as f64, *bv as f64);
            let v = ((w[0] * ir + w[1] * ig + w[2] * ib) / 100.0 + c)
                .round()
                .clamp(0.0, 255.0) as u8;
            *rv = v;
            *gv = v;
            *bv = v;
        }
    } else {
        for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
            let (ir, ig, ib) = (*rv as f64, *gv as f64, *bv as f64);
            let mix = |w: &[f64; 3], k: f64| {
                ((w[0] * ir + w[1] * ig + w[2] * ib) / 100.0 + k / 100.0 * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            *rv = mix(&p.red, p.constant[0]);
            *gv = mix(&p.green, p.constant[1]);
            *bv = mix(&p.blue, p.constant[2]);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Selective Color (libpsd's integer RGB -> CMYK -> RGB pipeline)
// ---------------------------------------------------------------------------

pub(crate) fn selective_color(
    p: &SelectiveColorParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    if p.ranges
        .iter()
        .flat_map(|r| [r.c, r.m, r.y, r.k])
        .any(|v| !(-100..=100).contains(&v))
    {
        return Err(AdjustError::InvalidParams(
            "selective color corrections must be -100..=100".into(),
        ));
    }
    // ponytail: libpsd always runs the lossy profile-free RGB -> CMYK -> RGB
    // round-trip even with zero corrections (only 256 of 2^24 triples survive).
    // The reference's zero-slider adjustment is a no-op, so return early; drop this
    // if byte parity with libpsd's round-trip is ever wanted.
    if p.ranges.iter().all(|r| *r == SelectiveRange::default()) {
        return Ok(());
    }
    let (r, g, b) = planes_mut(buf, n);
    for ((rv, gv), bv) in r.iter_mut().zip(g.iter_mut()).zip(b.iter_mut()) {
        let (nr, ng, nb) = selective_color_pixel(p, *rv as i32, *gv as i32, *bv as i32);
        *rv = nr;
        *gv = ng;
        *bv = nb;
    }
    Ok(())
}

fn selective_color_pixel(p: &SelectiveColorParams, r: i32, g: i32, b: i32) -> (u8, u8, u8) {
    let (sc, sm, sy, sk) = rgb_to_intcmyk(r, g, b);
    let src = [sc, sm, sy, sk];
    let mut dst = [sc, sm, sy, sk];
    let hue = rgb_to_int_hue(r, g, b);

    for (index, range) in p.ranges.iter().take(6).enumerate() {
        let i = index as i32 + 1;
        let r0 = -105 + i * 60;
        let (r1, r2, r3) = (r0 + 30, r0 + 60, r0 + 90);
        if hue >= r0 && hue < r3 {
            let opacity = if hue < r1 {
                (hue - r0) * 255 / 30
            } else if hue < r2 {
                255
            } else {
                (r3 - hue) * 255 / 30
            };
            add_correction(&p.method, range, opacity, src, &mut dst, 25500);
        }
    }

    for (index, range) in p.ranges.iter().enumerate().skip(6) {
        let selected = match index {
            6 => sk == 0,
            7 => sk > 0 && sk < 255,
            _ => sk == 255,
        };
        if selected {
            add_correction(&p.method, range, 1, src, &mut dst, 100);
        }
    }

    for ink in &mut dst {
        *ink = (*ink).clamp(0, 255);
    }
    intcmyk_to_rgb(dst[0], dst[1], dst[2], dst[3])
}

/// Add one family's corrections. `weight` is the hue opacity (`opacity` over
/// `25500`) or `1` over `100` for the tonal whites/neutrals/blacks.
fn add_correction(
    method: &SelectiveColorMethod,
    range: &SelectiveRange,
    weight: i32,
    src: [i32; 4],
    dst: &mut [i32; 4],
    divisor: i32,
) {
    for (i, (ink, correction)) in dst
        .iter_mut()
        .zip([range.c, range.m, range.y, range.k])
        .enumerate()
    {
        let correction = correction as i32;
        if correction == 0 {
            continue;
        }
        let amount = match method {
            SelectiveColorMethod::Relative => src[i],
            SelectiveColorMethod::Absolute => 255,
        };
        *ink += amount * correction * weight / divisor;
    }
}

/// `psd_rgb_to_intcmyk`: all divisions truncate toward zero.
fn rgb_to_intcmyk(r: i32, g: i32, b: i32) -> (i32, i32, i32, i32) {
    let (dc, dm, dy) = (255 - r, 255 - g, 255 - b);
    let k = dc.min(dm).min(dy);
    if k < 255 {
        let d = 255 - k;
        (
            (dc - k) * 255 / d,
            (dm - k) * 255 / d,
            (dy - k) * 255 / d,
            k,
        )
    } else {
        (0, 0, 0, k)
    }
}

/// `psd_rgb_to_inthsb`'s hue, in `0..=359`.
fn rgb_to_int_hue(r: i32, g: i32, b: i32) -> i32 {
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    if cmax == cmin {
        return 0;
    }
    let d = cmax - cmin;
    let h = if r == cmax {
        (g - b) * 60 / d
    } else if g == cmax {
        120 + (b - r) * 60 / d
    } else {
        240 + (r - g) * 60 / d
    };
    (h + 360) % 360
}

/// `psd_intcmyk_to_rgb`; the shifted value is always non-negative.
fn intcmyk_to_rgb(c: i32, m: i32, y: i32, k: i32) -> (u8, u8, u8) {
    let channel = |ink: i32| ((65535 - (ink * (255 - k) + (k << 8))) >> 8) as u8;
    (channel(c), channel(m), channel(y))
}
