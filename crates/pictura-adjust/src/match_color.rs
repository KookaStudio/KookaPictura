//! Image > Adjustments > Match Color (#352).
//!
//! A Reinhard-style statistics transfer: per-channel Lab mean and standard
//! deviation of a source image are imposed on the target, with `neutralize`
//! (drop the source's chroma mean), `luminance` and `color_intensity` scales,
//! and a `fade` blend toward the original. Adobe's kernel is closed; the
//! transfer approximates it (the docs contract, ADJ-031, proposes this model).
//! ponytail: the exact colour space is our choice; consolidating the Lab
//! conversion into `pictura-color` can follow #269.
//!
//! The statistics are computed by the caller over a whole layer or document
//! (never the preview section, which the shared apply path crops), so this
//! module stays pointwise. Selection confinement is applied by the renderer's
//! coverage mask; native-depth apply refuses the operation (the
//! [`crate::native`] catch-all).

use pictura_core::PixelBuffer;

use crate::common::{linear_to_srgb, map_rgb, srgb_to_linear};
use crate::types::AdjustError;

/// Per-channel Lab statistics (mean, then standard deviation) of an image,
/// a layer, or a selection of one. Computed by [`match_color_stats`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchColorStats {
    pub mean: [f64; 3],
    pub std: [f64; 3],
}

impl MatchColorStats {
    /// Zeroed statistics: no pixel contributed weight. As the *source* the
    /// kernel short-circuits to an identity (nothing to match); as the
    /// *target* the flat-channel guard keeps the buffer's contrast and only
    /// the source's mean transfers.
    pub const ZERO: MatchColorStats = MatchColorStats {
        mean: [0.0; 3],
        std: [0.0; 3],
    };
}

/// Match Color's full parameter set: the target's and the source's Lab
/// statistics plus the four dialog modifiers. `luminance` and
/// `color_intensity` span `1..=200` percent; `fade` spans `0..=100` percent
/// of the adjustment withheld (0 is the full match, 100 the identity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchColorParams {
    pub target: MatchColorStats,
    pub source: MatchColorStats,
    pub neutralize: bool,
    pub luminance: f64,
    pub color_intensity: f64,
    pub fade: f64,
}

/// sRGB (`0..=1`) to CIELAB D65 (`L` in `0..=100`).
fn rgb_to_lab(rgb: [f64; 3]) -> [f64; 3] {
    let lin = rgb.map(srgb_to_linear);
    let x = 0.4124564 * lin[0] + 0.3575761 * lin[1] + 0.1804375 * lin[2];
    let y = 0.2126729 * lin[0] + 0.7151522 * lin[1] + 0.0721750 * lin[2];
    let z = 0.0193339 * lin[0] + 0.1191920 * lin[1] + 0.9503041 * lin[2];
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (841.0 / 108.0) * t + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x / 0.95047), f(y), f(z / 1.08883));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// The inverse of [`rgb_to_lab`]; out-of-gamut results clamp at the buffer.
fn lab_to_rgb(lab: [f64; 3]) -> [f64; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    let finv = |t: f64| {
        let t3 = t * t * t;
        if t3 > 216.0 / 24389.0 {
            t3
        } else {
            (108.0 / 841.0) * (t - 4.0 / 29.0)
        }
    };
    let x = finv(fx) * 0.95047;
    let y = finv(fy);
    let z = finv(fz) * 1.08883;
    let lin = [
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.9692660 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    ];
    lin.map(linear_to_srgb)
}

/// Lab statistics of an interleaved straight-alpha image, each pixel weighted
/// by its alpha and, when a same-length selection plane is given, by that
/// pixel's coverage — so transparent canvas and unselected pixels do not
/// skew the means. Pixels with no weight are skipped entirely.
pub fn match_color_stats(pixels: &[[u8; 4]], selection: Option<&[u8]>) -> MatchColorStats {
    let mut weight = 0.0;
    let mut sums = [0.0f64; 3];
    let mut squares = [0.0f64; 3];
    for (i, px) in pixels.iter().enumerate() {
        let mut w = f64::from(px[3]) / 255.0;
        if let Some(coverage) = selection.and_then(|data| data.get(i)) {
            w *= f64::from(*coverage) / 255.0;
        }
        if w <= 0.0 {
            continue;
        }
        let lab = rgb_to_lab([px[0], px[1], px[2]].map(|v| f64::from(v) / 255.0));
        for c in 0..3 {
            sums[c] += w * lab[c];
            squares[c] += w * lab[c] * lab[c];
        }
        weight += w;
    }
    if weight <= 0.0 {
        return MatchColorStats::ZERO;
    }
    let mean = sums.map(|s| s / weight);
    // The weighted variance is sumsq/w − mean²; a flat channel rounds to a
    // tiny negative, so clamp at zero before the square root.
    let std: [f64; 3] =
        std::array::from_fn(|c| (squares[c] / weight - mean[c] * mean[c]).max(0.0).sqrt());
    MatchColorStats { mean, std }
}

fn check_params(p: &MatchColorParams) -> Result<(), AdjustError> {
    for (name, value, range) in [
        ("luminance", p.luminance, 1.0..=200.0),
        ("color intensity", p.color_intensity, 1.0..=200.0),
        ("fade", p.fade, 0.0..=100.0),
    ] {
        if !value.is_finite() || !range.contains(&value) {
            return Err(AdjustError::InvalidParams(format!(
                "{name} must be {}..={}",
                range.start(),
                range.end()
            )));
        }
    }
    for stats in [p.target, p.source] {
        if !stats
            .mean
            .iter()
            .chain(stats.std.iter())
            .all(|v| v.is_finite())
        {
            return Err(AdjustError::InvalidParams(
                "statistics must be finite".into(),
            ));
        }
    }
    Ok(())
}

/// The effective source mean: Neutralize keeps the source's luminance but
/// imposes no colour cast, which with the target's own statistics as the
/// source is exactly a gray-world cast removal.
fn source_mean(p: &MatchColorParams) -> [f64; 3] {
    if p.neutralize {
        [p.source.mean[0], 0.0, 0.0]
    } else {
        p.source.mean
    }
}

/// A self-match with default options: the transfer is the identity, and the
/// f64 Lab round trip could otherwise shift a pixel by one LSB.
fn is_identity(p: &MatchColorParams) -> bool {
    p.fade >= 100.0
        // A zeroed source (an all-transparent source layer, or "Use
        // Selection In Source" over an empty selection) has nothing to
        // transfer; scaling by its zero std would collapse the target onto
        // its black mean instead of leaving it alone.
        || p.source == MatchColorStats::ZERO
        || (p.source == p.target
            && !p.neutralize
            && p.luminance == 100.0
            && p.color_intensity == 100.0)
}

/// Apply Match Color in place to a planar buffer (alpha untouched). `n` is the
/// pixel count from [`crate::common::validate`].
pub(crate) fn match_color(
    p: &MatchColorParams,
    buf: &mut PixelBuffer,
    n: usize,
) -> Result<(), AdjustError> {
    check_params(p)?;
    if is_identity(p) {
        return Ok(());
    }
    let mean_s = source_mean(p);
    let scale: [f64; 3] = std::array::from_fn(|c| {
        // A flat target channel has nothing to scale by; keep its contrast.
        if p.target.std[c] < 1e-9 {
            1.0
        } else {
            p.source.std[c] / p.target.std[c]
        }
    });
    let l_scale = p.luminance / 100.0;
    // 1 is exactly achromatic and 100 exactly unchanged; both endpoints land.
    let c_scale = (p.color_intensity - 1.0) / 99.0;
    let keep = p.fade / 100.0;
    let blend = |orig: u8, new: f64| -> u8 {
        let o = f64::from(orig) / 255.0;
        ((o + (new - o) * (1.0 - keep)).clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    map_rgb(buf, n, |r, g, b| {
        let lab = rgb_to_lab([r, g, b].map(|v| f64::from(v) / 255.0));
        let mut out = [0.0f64; 3];
        for c in 0..3 {
            out[c] = (lab[c] - p.target.mean[c]) * scale[c] + mean_s[c];
        }
        out[0] *= l_scale;
        out[1] *= c_scale;
        out[2] *= c_scale;
        let rgb = lab_to_rgb(out).map(|v| v.clamp(0.0, 1.0));
        [blend(r, rgb[0]), blend(g, rgb[1]), blend(b, rgb[2])]
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_round_trips_within_one_lsb() {
        for v in 0..=255u8 {
            let rgb = [f64::from(v) / 255.0; 3];
            let back = lab_to_rgb(rgb_to_lab(rgb)).map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8);
            assert!(
                back.iter().all(|c| c.abs_diff(v) <= 1),
                "gray {v} round-tripped to {back:?}"
            );
        }
    }

    #[test]
    fn lab_separates_luminance_and_chroma() {
        let gray = rgb_to_lab([0.5, 0.5, 0.5]);
        // The decimal matrix rows do not sum to the white point exactly, so a
        // neutral pixel lands a few 1e-5 off zero, not at f64 epsilon.
        assert!(gray[1].abs() < 1e-3 && gray[2].abs() < 1e-3);
        let red = rgb_to_lab([1.0, 0.0, 0.0]);
        assert!(red[1] > 10.0 && red[2] > 10.0);
    }

    #[test]
    fn stats_weight_alpha_and_coverage() {
        let pixels = [
            [255u8, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 0], // fully transparent: no contribution
        ];
        let full = match_color_stats(&pixels, None);
        let selected = match_color_stats(&pixels, Some(&[255, 0, 255]));
        // Without a selection the mean is the red/green average; with only the
        // red pixel selected it is exactly red.
        let mean_l = |stats: &MatchColorStats| stats.mean[0];
        assert!((mean_l(&full) - mean_l(&selected)).abs() > 1.0);
        let red = rgb_to_lab([1.0, 0.0, 0.0]);
        assert!((selected.mean[1] - red[1]).abs() < 1e-9);
        assert_eq!(match_color_stats(&[], None), MatchColorStats::ZERO);
    }
}
