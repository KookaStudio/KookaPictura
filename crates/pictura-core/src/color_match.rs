//! The shared colour-match kernel for Color Range and Replace Color.
//!
//! Both dialogs match a pixel by the best Chebyshev distance to a set of
//! eyedropper samples, optionally localised around where each sample was
//! taken. The exact Photoshop kernels are unpublished, so this is a
//! behavioural approximation.

use crate::ReplaceColorSample;

/// Spatial falloff width for Localized Color Clusters, in pixels: 18% of the
/// image diagonal.
pub fn spatial_sigma(diagonal_px: f32) -> f32 {
    0.18 * diagonal_px.max(1.0)
}

/// The spatial strength for a 0..=100 Range: 100 means no proximity penalty,
/// 0 is a tight cluster around each positioned sample.
pub fn localization_strength(range: u32) -> f64 {
    1.0 - range.min(100) as f64 / 100.0
}

/// Selection weight for one pixel: the best match over all samples, so adding
/// a sample widens the selection rather than averaging it away.
///
/// `localization` is 0 (global) to 1 (a tight cluster); `sigma_sq` is the
/// spatial Gaussian width squared, in pixels. A sample with a negative
/// coordinate has no document position and carries no proximity weight.
#[allow(clippy::too_many_arguments)]
pub fn match_weight(
    r: u8,
    g: u8,
    b: u8,
    x: i32,
    y: i32,
    samples: &[ReplaceColorSample],
    fuzziness: f64,
    localization: f64,
    sigma_sq: f64,
) -> f64 {
    let localization = localization.clamp(0.0, 1.0);
    let sigma_sq = sigma_sq.max(f64::MIN_POSITIVE);
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
        if localization > 0.0 && w > 0.0 && s.x >= 0 && s.y >= 0 {
            let dx = (x - s.x) as f64;
            let dy = (y - s.y) as f64;
            let gaussian = (-(dx * dx + dy * dy) / (2.0 * sigma_sq)).exp();
            w *= 1.0 - localization * (1.0 - gaussian);
        }
        best = best.max(w);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(x: i32, y: i32, rgb: [u8; 3]) -> ReplaceColorSample {
        ReplaceColorSample { x, y, rgb }
    }

    #[test]
    fn range_strength_is_full_at_zero_and_none_at_hundred() {
        assert_eq!(localization_strength(0), 1.0);
        assert_eq!(localization_strength(100), 0.0);
        assert_eq!(localization_strength(500), 0.0);
        assert!((localization_strength(50) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sigma_is_eighteen_percent_of_the_diagonal() {
        assert!((spatial_sigma(100.0) - 18.0).abs() < 1e-6);
        assert!((spatial_sigma(0.0) - 0.18).abs() < 1e-6);
    }

    #[test]
    fn localized_weight_falls_off_with_distance() {
        let samples = [sample(0, 0, [255, 0, 0])];
        let at = |x| match_weight(255, 0, 0, x, 0, &samples, 200.0, 1.0, 100.0);
        assert!(at(0) > 0.99);
        assert!(at(10) > at(30));
        assert!(at(100) < 0.01);
    }

    #[test]
    fn range_strength_blends_the_falloff() {
        let samples = [sample(0, 0, [255, 0, 0])];
        let global = match_weight(255, 0, 0, 100, 0, &samples, 200.0, 0.0, 100.0);
        let tight = match_weight(255, 0, 0, 100, 0, &samples, 200.0, 1.0, 100.0);
        let half = match_weight(255, 0, 0, 100, 0, &samples, 200.0, 0.5, 100.0);
        assert_eq!(global, 1.0);
        assert!(half > tight && half < global);
    }

    #[test]
    fn a_positionless_sample_keeps_its_colour_weight() {
        let samples = [sample(-1, -1, [255, 0, 0])];
        let weight = match_weight(255, 0, 0, 500, 500, &samples, 200.0, 1.0, 1.0);
        assert_eq!(weight, 1.0);
    }

    #[test]
    fn the_best_sample_wins() {
        let samples = [sample(0, 0, [255, 0, 0]), sample(0, 0, [0, 0, 255])];
        assert_eq!(
            match_weight(0, 0, 255, 0, 0, &samples, 40.0, 0.0, 100.0),
            1.0
        );
        assert_eq!(
            match_weight(0, 255, 0, 0, 0, &samples, 40.0, 0.0, 100.0),
            0.0
        );
    }
}
