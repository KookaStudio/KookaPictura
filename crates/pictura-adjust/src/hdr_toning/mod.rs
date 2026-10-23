//! HDR tone mapping for 32-bit conversion and display.
//!
//! Four CS6 HDR Conversion methods operate on linear-light f32 samples:
//! Exposure & Gamma (the one fixed formula), Equalize Histogram, Highlight
//! Compression, and Local Adaptation. The three closed CS6 kernels are
//! approximated; see `methods.rs` and `local.rs`.
//!
//! The Local Adaptation Toning Curve/Corner is not part of this module; it is
//! a recorded ceiling (`docs/04-image-ops/adjustments/hdr-toning.md`).

mod local;
mod methods;
#[cfg(test)]
mod tests;

pub use methods::exposure_gamma;
pub use pictura_core::HdrToningParams as LocalAdaptationParams;

use crate::AdjustError;

/// CS6 HDR Conversion "Exposure and Gamma" parameters. `Default` is the identity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureGamma {
    /// Exposure in stops (EV); `0.0` is neutral.
    pub exposure_ev: f64,
    /// Contrast gamma; `1.0` is neutral. Must be finite and `> 0`.
    pub gamma: f64,
}

impl Default for ExposureGamma {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            gamma: 1.0,
        }
    }
}

/// The four HDR Conversion tone-mapping methods.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HdrToneMap {
    ExposureGamma(ExposureGamma),
    EqualizeHistogram,
    HighlightCompression,
    LocalAdaptation(LocalAdaptationParams),
}

/// Apply `method` to the first `width * height * channels` planar f32 samples.
///
/// `channels` is 1 (Grayscale) or 3 (RGB); the conversion path passes the color
/// planes only. Trailing alpha/extra samples are copied unchanged.
pub fn tone_map(
    samples: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    method: HdrToneMap,
) -> Result<Vec<f32>, AdjustError> {
    if width == 0 || height == 0 || !(channels == 1 || channels == 3) {
        return Err(AdjustError::InvalidParams(
            "invalid HDR tone-map dimensions".into(),
        ));
    }
    let n = width
        .checked_mul(height)
        .ok_or_else(|| AdjustError::InvalidParams("HDR tone-map size overflows".into()))?;
    let color_len = n
        .checked_mul(channels)
        .ok_or_else(|| AdjustError::InvalidParams("HDR tone-map size overflows".into()))?;
    if samples.len() < color_len {
        return Err(AdjustError::InvalidParams(
            "sample buffer is shorter than width*height*channels".into(),
        ));
    }

    let mut out = match method {
        HdrToneMap::ExposureGamma(params) => exposure_gamma(&samples[..color_len], params)?,
        HdrToneMap::EqualizeHistogram => {
            methods::equalize_histogram(&samples[..color_len], n, channels)?
        }
        HdrToneMap::HighlightCompression => {
            methods::highlight_compression(&samples[..color_len], n, channels)?
        }
        HdrToneMap::LocalAdaptation(params) => {
            local::local_adaptation(&samples[..color_len], width, height, channels, &params)?
        }
    };
    out.extend_from_slice(&samples[color_len..]);
    Ok(out)
}
