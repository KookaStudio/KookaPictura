//! The demosaic stage (#397): a DNG's CFA plane to linear camera-native RGB.
//!
//! Bilinear and AHD (Hirakawa & Parks, 2005) are published algorithms; the
//! X-Trans method (`xtrans.rs`, #1473) is Pictura's own; a SuperCCD grid is
//! then turned upright as LibRaw does (`superccd.rs`, #1621). Parity with Adobe
//! Camera Raw is approximate. White balance and the colour
//! matrix are the colour stage's job (#398); default crop and orientation are
//! applied after this stage.

use crate::error::PsdError;
use crate::probe::ImageBudget;

use super::{CfaPattern, DngImage};

mod ahd;
mod bilinear;
mod superccd;
mod xtrans;

pub(super) use superccd::rotated_size;

/// The interpolation used to fill each pixel's missing channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemosaicMethod {
    /// Same-colour neighbour averaging; any red-green-blue pattern.
    Bilinear,
    /// Edge-adaptive: adaptive homogeneity-directed on 2×2 Bayer, the
    /// edge-directed X-Trans method on 6×6 X-Trans, bilinear otherwise.
    Ahd,
}

/// Linear camera-native RGB, 0–65535, the size of the DNG's active area (or,
/// for a SuperCCD grid, of the upright image).
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
    let rgb_bytes = |w: usize, h: usize| {
        (w as u64)
            .checked_mul(h as u64)
            .and_then(|n| n.checked_mul(6))
            .ok_or_else(|| invalid("output size"))
    };
    let out = rgb_bytes(width, height)?;
    let upright = match image.fuji_width {
        None => None,
        Some(fw) => Some(
            superccd::rotated_size(fw, height)
                .filter(|_| fw <= width)
                .ok_or_else(|| invalid("SuperCCD grid size"))?,
        ),
    };
    let rotated = upright.map_or(Ok(0), |(w, h)| rgb_bytes(w, h))?;
    let input = (image.samples.len() * 2) as u64;
    if out.max(rotated) > budget.max_alloc_bytes
        || out + rotated + input > budget.max_document_bytes
    {
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
        DemosaicMethod::Ahd if xtrans::applies(&mosaic) => xtrans::demosaic(&mosaic),
        _ => bilinear::demosaic(&mosaic),
    };
    match (image.fuji_width, upright) {
        (Some(fw), Some((wide, high))) => Ok(CameraRgb {
            width: wide,
            height: high,
            rgb: superccd::rotate(&rgb, (width, height), fw, (wide, high)),
        }),
        _ => Ok(CameraRgb { width, height, rgb }),
    }
}
