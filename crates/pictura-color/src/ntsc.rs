//! NTSC video-safe gamut clamping.
//!
//! Adobe's NTSC Colors kernel is closed, so this is the project's recorded
//! approximation: convert sRGB to YIQ, scale the chroma vector into the
//! video-safe disc, convert back. Scaling `(I, Q)` uniformly preserves its
//! direction (hue), and `Y` is untouched (luma).

/// The largest `(I, Q)` magnitude the NTSC video-safe gamut allows.
pub const NTSC_MAX_CHROMA: f64 = 0.5;

/// sRGB, normalised to `0..=1`, to YIQ with the SMPTE/NTSC matrix.
pub fn rgb_to_yiq(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb;
    [
        0.299 * r + 0.587 * g + 0.114 * b,
        0.595_716 * r - 0.274_453 * g - 0.321_263 * b,
        0.211_456 * r - 0.522_591 * g + 0.311_135 * b,
    ]
}

/// YIQ to sRGB with the inverse SMPTE/NTSC matrix. The result is not clamped.
pub fn yiq_to_rgb(yiq: [f64; 3]) -> [f64; 3] {
    let [y, i, q] = yiq;
    [
        y + 0.956_296 * i + 0.621_024 * q,
        y - 0.272_122 * i - 0.647_380 * q,
        y - 1.106_989 * i + 1.704_615 * q,
    ]
}

/// Clamp an sRGB colour, normalised to `0..=1`, into the NTSC video-safe
/// gamut: a chroma vector longer than [`NTSC_MAX_CHROMA`] is scaled back
/// toward the achromatic axis, leaving luma and hue unchanged.
pub fn clamp_to_ntsc_gamut(rgb: [f64; 3]) -> [f64; 3] {
    let mut yiq = rgb_to_yiq(rgb);
    let chroma = yiq[1].hypot(yiq[2]);
    if chroma > NTSC_MAX_CHROMA {
        let scale = NTSC_MAX_CHROMA / chroma;
        yiq[1] *= scale;
        yiq[2] *= scale;
    }
    let out = yiq_to_rgb(yiq);
    [
        out[0].clamp(0.0, 1.0),
        out[1].clamp(0.0, 1.0),
        out[2].clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARIES: [[f64; 3]; 4] = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
    ];

    fn chroma(rgb: [f64; 3]) -> f64 {
        let yiq = rgb_to_yiq(rgb);
        yiq[1].hypot(yiq[2])
    }

    #[test]
    fn rgb_yiq_round_trips() {
        for rgb in [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.2, 0.6, 0.9],
            [0.123, 0.456, 0.789],
        ] {
            let back = yiq_to_rgb(rgb_to_yiq(rgb));
            for (a, b) in rgb.iter().zip(back) {
                assert!((a - b).abs() < 1e-6, "{rgb:?} round-tripped to {back:?}");
            }
        }
    }

    #[test]
    fn saturated_primary_chroma_is_clamped() {
        for rgb in PRIMARIES {
            assert!(chroma(rgb) > NTSC_MAX_CHROMA, "{rgb:?} is not saturated");
            let out = clamp_to_ntsc_gamut(rgb);
            assert!(
                chroma(out) <= NTSC_MAX_CHROMA + 1e-9,
                "{rgb:?} kept chroma {}",
                chroma(out)
            );
            for channel in out {
                assert!((0.0..=1.0).contains(&channel), "{out:?} left the cube");
            }
        }
    }

    #[test]
    fn clamp_preserves_luma_and_hue() {
        for rgb in PRIMARIES {
            let before = rgb_to_yiq(rgb);
            let after = rgb_to_yiq(clamp_to_ntsc_gamut(rgb));
            assert!(
                (before[0] - after[0]).abs() < 1e-6,
                "{rgb:?} changed luma from {} to {}",
                before[0],
                after[0]
            );
            // `I` and `Q` keep their ratio, so the hue direction is unchanged.
            let before_angle = before[1].atan2(before[2]);
            let after_angle = after[1].atan2(after[2]);
            assert!(
                (before_angle - after_angle).abs() < 1e-5,
                "{rgb:?} changed hue from {before_angle} to {after_angle}"
            );
        }
    }

    #[test]
    fn in_gamut_colour_is_unchanged() {
        for rgb in [
            [0.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.588, 0.392, 0.471],
        ] {
            assert!(chroma(rgb) <= NTSC_MAX_CHROMA, "{rgb:?} is not in gamut");
            let out = clamp_to_ntsc_gamut(rgb);
            for (a, b) in rgb.iter().zip(out) {
                assert!((a - b).abs() < 1e-6, "{rgb:?} changed to {out:?}");
            }
        }
    }
}
