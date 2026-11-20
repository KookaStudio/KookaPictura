//! Image > Calculations (#610): blend two single-channel sources, with an
//! optional mask, into one grayscale plane. The app owns the three result
//! targets (a new document, a new channel, or the selection), so this module
//! only computes the plane.
//!
//! The arithmetic runs in the unit `f32` domain at every source depth, then
//! narrows to one byte per pixel. Layer modes use the shared compositor blend;
//! Add and Subtract are the calculations-only arithmetic.

use std::sync::Arc;

use pictura_core::{Plane, Sample};
use rayon::prelude::*;

use crate::apply_image::{ApplyImageMode, ApplyImagePlanes};
use crate::blend::blend;

/// The dialog's choices. Planes are shared, so a preview copies nothing.
#[derive(Clone, Debug)]
pub struct CalculationsParams {
    pub source1: Arc<ApplyImagePlanes>,
    pub invert1: bool,
    pub source2: Arc<ApplyImagePlanes>,
    pub invert2: bool,
    pub mode: ApplyImageMode,
    /// `0..=1`.
    pub opacity: f32,
    pub mask: Option<Arc<ApplyImagePlanes>>,
    pub mask_invert: bool,
}

/// One pixel's blend: A is Source 1, B is Source 2.
#[inline]
fn blend_value(mode: ApplyImageMode, a: f32, b: f32) -> f32 {
    match mode {
        ApplyImageMode::Layer(mode) => blend(mode, [a; 3], [b; 3])[0],
        ApplyImageMode::Add { scale, offset } => (a + b) / scale + offset / 255.0,
        ApplyImageMode::Subtract { scale, offset } => (a - b) / scale + offset / 255.0,
    }
}

/// Blend over the whole plane. `None` when Source 2 or the mask does not match
/// Source 1's pixel dimensions.
pub fn calculate(params: &CalculationsParams) -> Option<Plane<u8>> {
    let (width, height) = (params.source1.width, params.source1.height);
    let sized = |planes: &ApplyImagePlanes| planes.width == width && planes.height == height;
    if !sized(&params.source2) || params.mask.as_deref().is_some_and(|mask| !sized(mask)) {
        return None;
    }
    if width == 0 || height == 0 {
        return Some(Vec::new().into());
    }
    let opacity = params.opacity.clamp(0.0, 1.0);
    let mut out = vec![0u8; width * height];
    out.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        for (x, value) in row.iter_mut().enumerate() {
            let i = y * width + x;
            let mut a = params.source1.gray(i);
            if params.invert1 {
                a = 1.0 - a;
            }
            let mut b = params.source2.gray(i);
            if params.invert2 {
                b = 1.0 - b;
            }
            let blended = blend_value(params.mode, a, b);
            let mut result = a + (blended - a) * opacity;
            if let Some(mask) = params.mask.as_deref() {
                let coverage = mask.gray(i);
                result *= if params.mask_invert {
                    1.0 - coverage
                } else {
                    coverage
                };
            }
            *value = u8::from_unit(result as f64);
        }
    });
    Some(out.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::BlendMode;

    #[test]
    fn layer_modes_match_the_compositor() {
        // Dissolve needs a positional dither and Subtract is a calculations-only
        // mode in Apply Image; neither is a layer entry here.
        for mode in BlendMode::LAYER_MODES {
            if matches!(mode, BlendMode::Dissolve | BlendMode::Subtract) {
                continue;
            }
            for &(a, b) in &[(0.0f32, 0.0f32), (0.2, 0.9), (0.75, 0.35), (1.0, 1.0)] {
                let got = blend_value(ApplyImageMode::Layer(mode), a, b);
                let want = blend(mode, [a; 3], [b; 3])[0];
                assert!(
                    (got - want).abs() < 1e-6,
                    "{mode:?} a={a} b={b}: {got} vs {want}"
                );
            }
        }
    }
}
