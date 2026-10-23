//! Blend Text Colors Using Gamma (`Edit > Color Settings > More Options`): a
//! curve on a type layer's coverage × opacity × Fill at composite time. The
//! setting is application-wide, not document or history state, so it lives
//! here rather than on the `Document` the compositor reads.
//!
//! The curve models dark type over a light backdrop blended in a γ space
//! instead of the document's ≈2.2: `1 − (1 − a)^(γ / 2.2)`. CS6 documents the
//! blend only qualitatively, so this is an approximation.
//!
//! ponytail: colour-independent, so light type on a dark backdrop thins too,
//! where a true gamma-space blend would thicken it; blend in γ space per
//! channel if a CS6 reference render shows the difference matters.
//!
//! The compositor reads the curve per pixel, so it is a lookup table: one per
//! gamma at the dialog's 0.01 resolution, built the first time that gamma is
//! used and kept for the process.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

/// CS6's default gamma.
pub const DEFAULT_TEXT_GAMMA: f32 = 1.45;
/// The document-space gamma the curve is relative to; γ equal to it is the
/// identity.
const DOCUMENT_GAMMA: f32 = 2.2;
/// The setting's range in hundredths: 1.00…2.20.
const MIN_HUNDREDTHS: u32 = 100;
const MAX_HUNDREDTHS: u32 = 220;
/// The stored value while the setting is off.
const OFF: u32 = 0;
/// Table steps; linear interpolation between them stays within an 8-bit level
/// except at γ near 1.00, where the curve is steep next to full coverage.
const STEPS: usize = 4096;

/// The gamma in hundredths, or [`OFF`].
static TEXT_GAMMA: AtomicU32 = AtomicU32::new(hundredths(DEFAULT_TEXT_GAMMA));

const fn hundredths(gamma: f32) -> u32 {
    (gamma * 100.0 + 0.5) as u32
}

/// The text gamma curve for one gamma value, sampled for interpolation.
pub(crate) struct TextCurve {
    table: Box<[f32]>,
}

impl TextCurve {
    fn new(gamma: f32) -> Self {
        let table = (0..=STEPS)
            .map(|i| text_gamma_alpha(i as f32 / STEPS as f32, gamma))
            .collect();
        Self { table }
    }

    /// `alpha` (0…1) through the curve.
    pub(crate) fn apply(&self, alpha: f32) -> f32 {
        let at = alpha.clamp(0.0, 1.0) * STEPS as f32;
        let i = (at as usize).min(STEPS - 1);
        let t = at - i as f32;
        self.table[i] + (self.table[i + 1] - self.table[i]) * t
    }
}

fn curve(hundredths: u32) -> &'static TextCurve {
    static CURVES: [OnceLock<TextCurve>; (MAX_HUNDREDTHS - MIN_HUNDREDTHS + 1) as usize] =
        [const { OnceLock::new() }; (MAX_HUNDREDTHS - MIN_HUNDREDTHS + 1) as usize];
    CURVES[(hundredths - MIN_HUNDREDTHS) as usize]
        .get_or_init(|| TextCurve::new(hundredths as f32 / 100.0))
}

/// Turn the gamma blend on at `gamma` (clamped to 1.00–2.20, kept to two
/// decimals) or off (`None`).
pub fn set_text_gamma(gamma: Option<f32>) {
    let value = gamma.map_or(OFF, |g| {
        hundredths(g.clamp(1.0, DOCUMENT_GAMMA)).clamp(MIN_HUNDREDTHS, MAX_HUNDREDTHS)
    });
    TEXT_GAMMA.store(value, Ordering::Relaxed);
}

/// The gamma type layers blend with; `None` while the setting is off.
pub fn text_gamma() -> Option<f32> {
    match TEXT_GAMMA.load(Ordering::Relaxed) {
        OFF => None,
        value => Some(value as f32 / 100.0),
    }
}

/// The current setting's curve; `None` while it is off.
pub(crate) fn current_curve() -> Option<&'static TextCurve> {
    match TEXT_GAMMA.load(Ordering::Relaxed) {
        OFF => None,
        value => Some(curve(value)),
    }
}

/// `alpha` (0…1) through the exact curve for `gamma`; the tables sample it.
pub fn text_gamma_alpha(alpha: f32, gamma: f32) -> f32 {
    let a = alpha.clamp(0.0, 1.0);
    1.0 - (1.0 - a).powf(gamma / DOCUMENT_GAMMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_maps_coverage() {
        assert_eq!(text_gamma_alpha(0.0, DEFAULT_TEXT_GAMMA), 0.0);
        assert!((text_gamma_alpha(1.0, DEFAULT_TEXT_GAMMA) - 1.0).abs() < 1e-6);
        let half = text_gamma_alpha(0.5, DEFAULT_TEXT_GAMMA);
        assert!(half > 0.36 && half < 0.38, "0.5 maps to {half}");
        for i in 0..=255 {
            let a = i as f32 / 255.0;
            assert!(
                text_gamma_alpha(a, DEFAULT_TEXT_GAMMA) <= a + 1e-6,
                "at {a}"
            );
            assert!(
                (text_gamma_alpha(a, DOCUMENT_GAMMA) - a).abs() < 1e-6,
                "identity at {a}"
            );
        }
    }

    #[test]
    fn the_table_matches_the_curve() {
        for gamma in [1.0, DEFAULT_TEXT_GAMMA, 1.8, DOCUMENT_GAMMA] {
            let table = curve(hundredths(gamma));
            for i in 0..=1000 {
                let a = i as f32 / 1000.0;
                let error = (table.apply(a) - text_gamma_alpha(a, gamma)).abs();
                let bound = if gamma <= 1.0 && a > 0.999 {
                    0.01
                } else {
                    0.5 / 255.0
                };
                assert!(error <= bound, "γ {gamma} at {a}: error {error}");
            }
        }
        assert_eq!(curve(hundredths(DEFAULT_TEXT_GAMMA)).apply(0.0), 0.0);
        assert!((curve(hundredths(DEFAULT_TEXT_GAMMA)).apply(1.0) - 1.0).abs() < 1e-6);
    }
}
