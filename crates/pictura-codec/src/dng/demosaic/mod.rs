//! The demosaic stage (#397): a DNG's CFA plane to linear camera-native RGB.
//!
//! Published algorithms only (bilinear; AHD from Hirakawa & Parks, 2005), so
//! parity with Adobe Camera Raw is approximate. White balance and the colour
//! matrix are the colour stage's job (#398); default crop and orientation are
//! applied after this stage.

use crate::error::PsdError;
use crate::probe::ImageBudget;

use super::{CfaPattern, DngImage};

mod ahd;
mod bilinear;

/// The interpolation used to fill each pixel's missing channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemosaicMethod {
    /// Same-colour neighbour averaging; any red-green-blue pattern.
    Bilinear,
    /// Adaptive homogeneity-directed; 2×2 Bayer, bilinear otherwise.
    Ahd,
}

/// Linear camera-native RGB, 0–65535, the size of the DNG's active area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraRgb {
    pub width: usize,
    pub height: usize,
    /// `width × height × 3`, row-major, interleaved red, green, blue.
    pub rgb: Vec<u16>,
}

/// The pattern and plane the passes read, with the colour lookup they share.
pub(super) struct Mosaic<'a> {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) samples: &'a [u16],
    pub(super) cfa: &'a CfaPattern,
}

impl Mosaic<'_> {
    pub(super) fn color(&self, x: usize, y: usize) -> usize {
        self.cfa.colors[(y % self.cfa.height) * self.cfa.width + x % self.cfa.width] as usize
    }
}

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("DNG demosaic: {what}"))
}

/// Demosaics `image` with `method`, charging the output against `budget`
/// before it is allocated.
pub fn demosaic(
    image: &DngImage,
    method: DemosaicMethod,
    budget: &ImageBudget,
) -> Result<CameraRgb, PsdError> {
    let (width, height, cfa) = (image.width, image.height, &image.cfa);
    if cfa.width == 0 || cfa.height == 0 || cfa.colors.len() != cfa.width * cfa.height {
        return Err(invalid("CFA pattern size"));
    }
    if (0..3u8).any(|c| !cfa.colors.contains(&c)) || cfa.colors.iter().any(|&c| c > 2) {
        return Err(invalid("CFA pattern lacks red, green or blue"));
    }
    if image.samples.len() != width * height {
        return Err(invalid("sample count"));
    }
    let out = (width as u64)
        .checked_mul(height as u64)
        .and_then(|n| n.checked_mul(6))
        .ok_or_else(|| invalid("output size"))?;
    let input = (image.samples.len() * 2) as u64;
    if out > budget.max_alloc_bytes || out + input > budget.max_document_bytes {
        return Err(invalid("output exceeds the allocation budget"));
    }
    let mosaic = Mosaic {
        width,
        height,
        samples: &image.samples,
        cfa,
    };
    let rgb = match method {
        DemosaicMethod::Ahd if ahd::applies(&mosaic) => ahd::demosaic(&mosaic),
        _ => bilinear::demosaic(&mosaic),
    };
    Ok(CameraRgb { width, height, rgb })
}
