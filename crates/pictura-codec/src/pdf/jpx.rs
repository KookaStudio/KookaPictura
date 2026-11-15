//! JPEG 2000 codestream encoding for the page image (`interop/pdf-export`).
//!
//! The PDF `/JPXDecode` filter carries a raw JPEG 2000 codestream, not a JP2
//! container, so this module encodes with `Image::encode` rather than
//! `encode_jp2`. The encoder is a pure-Rust OpenJPEG port, keeping the codec
//! free of a native library or the Qt/KDE image plugins.

use jpeg2k::{ColorSpace, CoreImage, CoreImageComp, EncodeParameters, Image};

use super::image::Raster;
use super::options::Quality;
use crate::error::PsdError;

/// `Quality` as a compression ratio passed to OpenJPEG's rate allocation.
///
/// ponytail: inferred from Photoshop's five Image Quality names; there is no
/// published Distiller QFactor-to-JPEG-2000 mapping. Tune these five numbers
/// if measured output needs to match a reference more closely.
fn ratio(quality: Quality) -> f32 {
    match quality {
        Quality::Minimum => 40.0,
        Quality::Low => 20.0,
        Quality::Medium => 10.0,
        Quality::High => 5.0,
        Quality::Maximum => 2.0,
    }
}

fn color_space(components: usize) -> ColorSpace {
    match components {
        1 => ColorSpace::Gray,
        3 => ColorSpace::Srgb,
        4 => ColorSpace::Cmyk,
        _ => ColorSpace::Unknown,
    }
}

/// Encode the 8-bit interleaved raster as a JPEG 2000 codestream with `tile`
/// square tiles at the chosen image quality.
pub(crate) fn encode(raster: &Raster, tile: u32, quality: Quality) -> Result<Vec<u8>, PsdError> {
    if raster.bpc != 8 {
        return Err(PsdError::Unsupported(
            "JPEG 2000 PDF images must be 8-bit".into(),
        ));
    }
    let components = raster.space.components();
    let (width, height) = (raster.width, raster.height);
    let pixels = width as usize * height as usize;
    if pixels == 0 || raster.samples.len() < pixels * components {
        return Err(PsdError::Invalid("JPEG 2000 raster is incomplete".into()));
    }
    let mut comps = Vec::with_capacity(components);
    for c in 0..components {
        let data: Vec<i32> = (0..pixels)
            .map(|i| i32::from(raster.samples[i * components + c]))
            .collect();
        comps.push(CoreImageComp {
            dx: 1,
            dy: 1,
            w: width,
            h: height,
            x0: 0,
            y0: 0,
            prec: 8,
            bpp: 8,
            sgnd: 0,
            resno_decoded: 0,
            factor: 0,
            alpha: 0,
            data,
        });
    }
    let core = CoreImage {
        x0: 0,
        y0: 0,
        x1: width,
        y1: height,
        numcomps: components as u32,
        color_space: color_space(components),
        comps,
        icc_profile_buf: Vec::new(),
        icc_profile_is_cielab: false,
    };
    let params = EncodeParameters::new()
        .tile_size(tile.max(1), tile.max(1))
        .irreversible(true)
        .layer_rate(ratio(quality));
    Image::from_core_image(core)
        .encode(params)
        .map_err(|e| PsdError::Unsupported(format!("JPEG 2000 encoding failed: {e}")))
}
