//! `Image > Mode > 32 -> 16/8` HDR Conversion (`docs/04-image-ops/32-bit-hdr.md`,
//! IMG-009). CS6's HDR Conversion dialog offers four tone-mapping methods; only
//! **Exposure & Gamma** is grounded in a fixed formula
//! (`pictura_adjust::hdr_toning`), so this ships that one.
//!
//! ponytail: Local Adaptation, Equalize Histogram, Highlight Compression, and
//! the Method combo are absent (closed kernels); the Exposure/Gamma ranges
//! beyond `0`/`1.0` are inferred; 32-bit Lab/CMYK are out of scope
//! (`source_mode` refuses them); layered documents tone-map the merged composite
//! and drop each layer's native store; and a document already *edited* while
//! 32-bit was clamped to `[0, 1]` by `emit_native`, so only a clean open keeps
//! values above white to tone-map.

use pictura_adjust::{exposure_gamma, ExposureGamma};
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, Samples, SourcePlanes};
use pictura_ops::OpsError;

/// Convert a 32-bit Grayscale/RGB `doc` to `out` (16 or 8) with the CS6 HDR
/// Conversion "Exposure & Gamma" method.
///
/// The retained `f32` color planes are tone-mapped (values above `1.0` survive
/// until the output quantization); alpha/extra planes are copied untoned. The
/// store, depth fields, and composite are rebuilt so a save writes the new
/// depth, and every layer's native store is dropped.
///
/// Returns an `OpsError` without mutating `doc` outside the gate: a non-32-bit
/// source, a converted source mode, a mode other than RGB/Grayscale, or an
/// output other than 16/8.
pub fn convert_depth_exposure_gamma(
    doc: &mut Document,
    out: BitDepth,
    params: ExposureGamma,
) -> Result<(), OpsError> {
    if doc.source_depth != Some(BitDepth::ThirtyTwo) {
        return Err(OpsError::InvalidParams(
            "HDR conversion needs a 32-bit source".into(),
        ));
    }
    if doc.source_mode.is_some() {
        return Err(OpsError::Unsupported(
            "HDR conversion is limited to a native Grayscale/RGB source".into(),
        ));
    }
    if !matches!(doc.mode, ColorMode::Rgb | ColorMode::Grayscale) {
        return Err(OpsError::Unsupported(format!(
            "HDR conversion of {:?} is unsupported",
            doc.mode
        )));
    }
    if !matches!(out, BitDepth::Sixteen | BitDepth::Eight) {
        return Err(OpsError::InvalidParams(
            "HDR conversion output must be 16 or 8 bits".into(),
        ));
    }

    let plane = doc.width as usize * doc.height as usize;
    let color_channels = doc.mode.color_channels() as usize;
    let color_len = color_channels * plane;
    if color_len == 0 {
        return Err(OpsError::InvalidParams("empty document".into()));
    }

    // Prefer the retained 32-bit store (a clean open keeps values above 1.0);
    // an edited document's fresh composite is already clamped by `emit_native`.
    let mut samples = source_samples(doc, color_len)?;
    let toned = exposure_gamma(&samples[..color_len], params)
        .map_err(|e| OpsError::InvalidParams(e.to_string()))?;
    samples[..color_len].copy_from_slice(&toned);

    let (store, composite) = match out {
        BitDepth::Sixteen => {
            let store = SourcePlanes {
                depth: BitDepth::Sixteen,
                width: doc.width,
                height: doc.height,
                samples: Samples::U16(Samples::F32(samples).to_u16()),
            };
            let narrowed = store.samples.narrow_to_u8();
            (Some(store), narrowed[..color_len].to_vec())
        }
        _ => {
            let narrowed = Samples::F32(samples).narrow_to_u8();
            (None, narrowed[..color_len].to_vec())
        }
    };

    // The working model stays 8-bit (`write_psd` rejects a non-8 working depth);
    // the converted store and `source_depth` are what a save re-emits.
    doc.depth = BitDepth::Eight;
    doc.source_depth = store.as_ref().map(|_| BitDepth::Sixteen);
    doc.source_planes = store;
    doc.composite = PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: color_channels as u8,
        data: composite.into(),
    };
    drop_layer_source(&mut doc.layers);
    Ok(())
}

/// The document's source samples as `f32`: the retained store when present,
/// else a fresh native composite restricted to the color planes.
fn source_samples(doc: &Document, color_len: usize) -> Result<Vec<f32>, OpsError> {
    if let Some(Samples::F32(v)) = doc.source_planes.as_ref().map(|s| &s.samples) {
        if v.len() >= color_len {
            return Ok(v.clone());
        }
    }
    match crate::composite_native(doc) {
        Some(Samples::F32(v)) if v.len() >= color_len => Ok(v[..color_len].to_vec()),
        _ => Err(OpsError::InvalidParams(
            "32-bit source has no f32 samples".into(),
        )),
    }
}

/// Clear every layer's retained native store, recursing into groups, so a 32-bit
/// store cannot be re-emitted into the converted document.
fn drop_layer_source(layers: &mut pictura_core::LayerStack) {
    for layer in layers {
        layer.source_channels = None;
        drop_layer_source(&mut layer.children);
    }
}
