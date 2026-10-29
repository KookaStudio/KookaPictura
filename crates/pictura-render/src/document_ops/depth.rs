//! `Image > Mode > 32 -> 16/8` HDR Conversion (`docs/04-image-ops/32-bit-hdr.md`,
//! IMG-009). The document gate, quantization, store rebuild, and layer-store
//! clearing live here; the four tone-mapping kernels live in `pictura-adjust`.
//!
//! The retained `f32` store is a clean-open snapshot. It is used only while it
//! still matches the current composite; after an edit the current composite is
//! the source, so a conversion never tone-maps pre-edit pixels.
//!
//! `ponytail:` a document already edited while 32-bit was clamped to `[0, 1]`
//! by `emit_native` cannot recover its HDR range; only a clean open keeps values
//! above white to tone-map. A layered document is tone-mapped from its merged
//! composite and flattened to one converted Background, because no pre-existing
//! layer tree can reproduce the tone-mapped merge on the next recomposite.

use pictura_adjust::{tone_map, ExposureGamma, HdrToneMap};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerStack, PixelBuffer, PsdRect, Samples,
    SourceChannels, SourcePlanes,
};
use pictura_ops::OpsError;

/// Convert a 32-bit Grayscale/RGB `doc` to `out` (16 or 8) with `method`.
///
/// The color planes of the current source are tone-mapped; alpha/extra planes
/// are copied untoned. The store, depth fields, and composite are rebuilt so a
/// save writes the new depth. Every pre-existing layer's native store is
/// dropped; a layered document is flattened to one Background whose 16-bit
/// output carries the converted color planes.
///
/// Returns an `OpsError` without mutating `doc` outside the gate: a non-32-bit
/// source, a converted source mode, a mode other than RGB/Grayscale, or an
/// output other than 16/8.
pub fn convert_depth(
    doc: &mut Document,
    out: BitDepth,
    method: HdrToneMap,
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

    let plane = (doc.width as usize)
        .checked_mul(doc.height as usize)
        .ok_or_else(|| OpsError::InvalidParams("HDR conversion size overflows".into()))?;
    let color_channels = doc.mode.color_channels() as usize;
    let color_len = plane
        .checked_mul(color_channels)
        .ok_or_else(|| OpsError::InvalidParams("HDR conversion size overflows".into()))?;
    if color_len == 0 {
        return Err(OpsError::InvalidParams("empty document".into()));
    }

    let mut samples = source_samples(doc, color_len)?;
    // A layered document is displayed by recompositing its layers, so the
    // tone-mapped merge has no layer tree that can reproduce it. Keep the
    // display alpha, then replace the tree with one converted Background.
    let display_alpha = if doc.layers.is_empty() {
        None
    } else {
        crate::composite_rgba(doc)
            .data
            .get(3 * plane..4 * plane)
            .map(<[u8]>::to_vec)
    };
    let toned = tone_map(
        &samples[..color_len],
        doc.width as usize,
        doc.height as usize,
        color_channels,
        method,
    )
    .map_err(|e| OpsError::InvalidParams(e.to_string()))?;
    samples[..color_len].copy_from_slice(&toned);

    let (store, composite) = match out {
        BitDepth::Sixteen => {
            let store = SourcePlanes {
                depth: BitDepth::Sixteen,
                width: doc.width,
                height: doc.height,
                samples: Samples::U16(Samples::F32(samples.into()).to_u16().into()),
            };
            let narrowed = store.samples.narrow_to_u8();
            (Some(store), narrowed[..color_len].to_vec())
        }
        _ => {
            let narrowed = Samples::F32(samples.into()).narrow_to_u8();
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
    flatten_converted(doc, display_alpha.as_deref());
    Ok(())
}

/// Replace a converted document's layer tree with one Background carrying the
/// tone-mapped composite. The tone map runs on the merged pixels, so no layer
/// stack can reproduce the result; a later recomposite must use this layer
/// rather than the pre-conversion layers. A 16-bit output keeps the converted
/// store's native colour planes on the layer, so a save does not widen them.
fn flatten_converted(doc: &mut Document, display_alpha: Option<&[u8]>) {
    if doc.layers.is_empty() {
        return;
    }
    let plane = doc.width as usize * doc.height as usize;
    let colors = doc.mode.color_channels() as usize;
    let alpha = display_alpha
        .filter(|alpha| alpha.len() == plane)
        .map_or_else(|| vec![255; plane], <[u8]>::to_vec);
    let mut channels: Vec<Channel> = (0..colors)
        .map(|c| Channel {
            id: c as i16,
            data: doc.composite.data[c * plane..(c + 1) * plane]
                .to_vec()
                .into(),
        })
        .collect();
    channels.push(Channel {
        id: -1,
        data: alpha.clone().into(),
    });
    let rect = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let source_channels = doc.source_planes.as_ref().and_then(|store| {
        let mut planes: Vec<(i16, Samples)> = (0..colors)
            .filter_map(|c| {
                store
                    .samples
                    .slice(c * plane..(c + 1) * plane)
                    .map(|samples| (c as i16, samples))
            })
            .collect();
        if planes.len() != colors {
            return None;
        }
        let native_alpha = match store.depth {
            BitDepth::ThirtyTwo => {
                Samples::F32(alpha.iter().map(|&v| f32::from(v) / 255.0).collect())
            }
            _ => Samples::U16(alpha.iter().map(|&v| u16::from(v) * 257).collect()),
        };
        planes.push((-1, native_alpha));
        Some(SourceChannels::new(store.depth, rect, planes))
    });
    doc.layers = LayerStack::from_layers(vec![Layer {
        name: "Background".to_string(),
        rect,
        channels,
        source_channels,
        background: true,
        ..Default::default()
    }]);
}

/// Convert a 32-bit Grayscale/RGB `doc` with the Exposure & Gamma method.
///
/// A convenience wrapper over [`convert_depth`]; the behavior and errors are the
/// same.
pub fn convert_depth_exposure_gamma(
    doc: &mut Document,
    out: BitDepth,
    params: ExposureGamma,
) -> Result<(), OpsError> {
    convert_depth(doc, out, HdrToneMap::ExposureGamma(params))
}

/// The current source samples as `f32`.
///
/// The retained store is used only while it still matches the composite; a
/// stale store falls back to the current composite. A document with layers is
/// recomposited at native depth; a layerless document reads its composite.
fn source_samples(doc: &Document, color_len: usize) -> Result<Vec<f32>, OpsError> {
    if let Some(Samples::F32(v)) = doc.source_planes.as_ref().map(|s| &s.samples) {
        if v.len() >= color_len && store_matches_composite(doc, &v[..color_len]) {
            return Ok(v.to_vec());
        }
    }
    current_composite_samples(doc, color_len)
}

/// Whether the retained store's narrowed color planes still equal the document
/// composite (the clean-open state). The read path narrows with the same
/// `Samples::narrow_to_u8` rule, so an unedited file matches exactly.
fn store_matches_composite(doc: &Document, store: &[f32]) -> bool {
    let color_len = store.len();
    if doc.composite.data.len() < color_len {
        return false;
    }
    let narrowed = Samples::F32(store.to_vec().into()).narrow_to_u8();
    narrowed[..color_len] == doc.composite.data[..color_len]
}

fn current_composite_samples(doc: &Document, color_len: usize) -> Result<Vec<f32>, OpsError> {
    let mut samples = if doc.layers.is_empty() {
        if doc.composite.data.len() >= color_len {
            doc.composite
                .data
                .iter()
                .map(|&v| f32::from(v) / 255.0)
                .collect()
        } else {
            Vec::new()
        }
    } else if let Some(Samples::F32(v)) = crate::composite_native(doc) {
        if v.len() >= color_len {
            v.to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    if samples.is_empty() {
        if doc.composite.data.len() < color_len {
            return Err(OpsError::InvalidParams(
                "32-bit source has no usable samples".into(),
            ));
        }
        samples = doc
            .composite
            .data
            .iter()
            .map(|&v| f32::from(v) / 255.0)
            .collect();
    }
    Ok(samples)
}

/// Clear every layer's retained native store, recursing into groups, so a 32-bit
/// store cannot be re-emitted into the converted document.
fn drop_layer_source(layers: &mut pictura_core::LayerStack) {
    for layer in layers {
        layer.source_channels = None;
        drop_layer_source(&mut layer.children);
    }
}
