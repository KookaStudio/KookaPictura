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

use std::sync::atomic::{AtomicU32, Ordering};

/// CS6's default gamma.
pub const DEFAULT_TEXT_GAMMA: f32 = 1.45;
/// The document-space gamma the curve is relative to; γ equal to it is the
/// identity.
const DOCUMENT_GAMMA: f32 = 2.2;
/// The bits of 0.0, which no valid gamma has: the setting is off.
const OFF: u32 = 0;

static TEXT_GAMMA: AtomicU32 = AtomicU32::new(DEFAULT_TEXT_GAMMA.to_bits());

/// Turn the gamma blend on at `gamma` (clamped to 1.00–2.20) or off (`None`).
pub fn set_text_gamma(gamma: Option<f32>) {
    let bits = gamma.map_or(OFF, |g| g.clamp(1.0, DOCUMENT_GAMMA).to_bits());
    TEXT_GAMMA.store(bits, Ordering::Relaxed);
}

/// The gamma type layers blend with; `None` while the setting is off.
pub fn text_gamma() -> Option<f32> {
    match TEXT_GAMMA.load(Ordering::Relaxed) {
        OFF => None,
        bits => Some(f32::from_bits(bits)),
    }
}

/// `alpha` (0…1) through the curve for `gamma`.
pub fn text_gamma_alpha(alpha: f32, gamma: f32) -> f32 {
    let a = alpha.clamp(0.0, 1.0);
    1.0 - (1.0 - a).powf(gamma / DOCUMENT_GAMMA)
}

/// `alpha` through the current setting's curve; unchanged while it is off.
pub(crate) fn current_text_alpha(alpha: f32) -> f32 {
    text_gamma().map_or(alpha, |gamma| text_gamma_alpha(alpha, gamma))
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
}
