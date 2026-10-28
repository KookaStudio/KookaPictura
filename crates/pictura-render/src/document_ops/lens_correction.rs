//! Lens Correction document ops: the non-destructive smart-object apply and the
//! stored record read.
//!
//! `apply_lens_correction` attaches the Lens Correction smart filter to a
//! smart-object layer so the settings round-trip on save and re-open; the
//! compositor renders the chain live over the layer's unfiltered source, so
//! disabling, clearing, or deleting the filter falls back to the source.
//! `lens_correction_slots` reads the stored record back for the dialog's prefill.

use pictura_core::{Document, PixelBuffer, SmartObjectKind};

use super::layer_ops::{resolve_path, resolve_path_mut};

/// Apply the Lens Correction slots to the smart-object layer at `path`.
///
/// Returns `false` without mutating for a missing path, a non-smart-object, a
/// group, an adjustment layer, a non-embedded object, an unrenderable rect, or
/// slots that do not map. On success the layer's color channels hold the
/// corrected source at the layer rect, alpha is preserved, and the slots are
/// attached to the Lens Correction smart filter.
pub fn apply_lens_correction(doc: &mut Document, path: &str, slots: &[f64]) -> bool {
    if pictura_filters::LensCorrectionParams::from_slots(slots).is_none() {
        return false;
    }
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group || layer.adjustment.is_some() {
        return false;
    }
    let Some(so) = layer.smart_object.as_ref() else {
        return false;
    };
    if so.kind != SmartObjectKind::Embedded {
        return false;
    }
    let rect = layer.rect;
    if rect.width() <= 0 || rect.height() <= 0 {
        return false;
    }
    // Attach the smart filter only. The layer's proxy stays the unfiltered
    // source, so the compositor renders the chain live and disabling, clearing,
    // or deleting the filter falls back to the source rather than to a baked
    // correction.
    let layer = resolve_path_mut(doc, path).expect("resolved above");
    pictura_codec::attach_lens_correction_filter(layer, slots).is_ok()
}

/// The Lens Correction smart filter's slots on the layer at `path`, or `None`
/// when it carries none. Pure read.
pub fn lens_correction_slots(doc: &Document, path: &str) -> Option<Vec<f64>> {
    let layer = resolve_path(doc, path)?;
    layer
        .smart_object
        .as_ref()?
        .smart_filters
        .iter()
        .find(|f| f.filter_id == pictura_codec::LENS_CORRECTION_FILTER_ID)
        .map(|f| pictura_codec::decode_lens_correction_slots(&f.options))
}

/// Render the embedded source of the smart-object layer at `path` at its rect,
/// as a straight-alpha RGBA buffer, so a dialog previews from the source rather
/// than the baked proxy. `None` for an ineligible target or a rect with no area.
pub fn smart_source_buffer(doc: &Document, path: &str) -> Option<PixelBuffer> {
    let layer = resolve_path(doc, path)?;
    if layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    let so = layer.smart_object.as_ref()?;
    let rect = layer.rect;
    if rect.width() <= 0 || rect.height() <= 0 {
        return None;
    }
    crate::composite::render_smart_source(so, rect, rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, Channel, ColorMode, Layer, PsdRect, SmartObject, SmartObjectKind,
    };

    /// A 4x4 RGB payload with a gradient, so a correction changes the pixels.
    fn payload() -> Vec<u8> {
        let mut d = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        for y in 0..4usize {
            for x in 0..4usize {
                let i = y * 4 + x;
                d.composite.data[i] = (x * 60) as u8;
                d.composite.data[16 + i] = (y * 60) as u8;
                d.composite.data[32 + i] = ((x + y) * 30) as u8;
            }
        }
        pictura_codec::write_psd(&d).expect("payload writes")
    }

    fn smart_doc() -> Document {
        let mut layer = Layer {
            name: "Smart".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            smart_object: Some(
                {
                    let mut so = SmartObject::default();
                    so.kind = SmartObjectKind::Embedded;
                    so
                }
                .with_payload(payload()),
            ),
            ..Default::default()
        };
        for id in 0..3i16 {
            layer.channels.push(Channel {
                id,
                data: vec![50u8; 16].into(),
            });
        }
        layer.channels.push(Channel {
            id: -1,
            data: vec![200u8; 16].into(),
        });
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        doc
    }

    fn slots(distortion: f64) -> Vec<f64> {
        let mut v = vec![
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, distortion, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0,
            100.0,
        ];
        // No sub-profile, no coefficients, then the identity model.
        v.extend([0.0; 8]);
        v.extend([1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        v.extend([0.0; 3]);
        v
    }

    fn channel(doc: &Document, id: i16) -> Vec<u8> {
        doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.data.to_vec())
            .unwrap_or_default()
    }

    #[test]
    fn smart_apply_attaches_the_filter() {
        let mut doc = smart_doc();
        let before = channel(&doc, 0);
        assert!(apply_lens_correction(&mut doc, "0", &slots(40.0)));
        assert_eq!(
            channel(&doc, 0),
            before,
            "the proxy stays the unfiltered source"
        );

        let so = doc.layers[0].smart_object.as_ref().expect("smart object");
        assert_eq!(so.smart_filters.len(), 1);
        assert_eq!(
            so.smart_filters[0].filter_id,
            pictura_codec::LENS_CORRECTION_FILTER_ID
        );
        assert_eq!(
            pictura_codec::decode_lens_correction_slots(&so.smart_filters[0].options),
            slots(40.0)
        );
    }

    #[test]
    fn smart_apply_replaces_the_entry() {
        let mut doc = smart_doc();
        assert!(apply_lens_correction(&mut doc, "0", &slots(20.0)));
        assert!(apply_lens_correction(&mut doc, "0", &slots(60.0)));
        let so = doc.layers[0].smart_object.as_ref().expect("smart object");
        assert_eq!(so.smart_filters.len(), 1, "replaced, not duplicated");
        assert_eq!(
            pictura_codec::decode_lens_correction_slots(&so.smart_filters[0].options),
            slots(60.0)
        );
    }

    #[test]
    fn smart_apply_refuses_ineligible_targets() {
        let mut missing = smart_doc();
        assert!(!apply_lens_correction(&mut missing, "9", &slots(20.0)));

        let mut raster = smart_doc();
        raster.layers[0].smart_object = None;
        assert!(!apply_lens_correction(&mut raster, "0", &slots(20.0)));

        let mut group = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        group.layers = pictura_core::LayerStack::from_layers(vec![Layer {
            name: "g".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            is_group: true,
            ..Default::default()
        }]);
        assert!(!apply_lens_correction(&mut group, "0", &slots(20.0)));

        // Slots that do not map are refused without attaching.
        let mut bad = smart_doc();
        assert!(!apply_lens_correction(&mut bad, "0", &[0.0; 15]));
        assert!(bad.layers[0]
            .smart_object
            .as_ref()
            .expect("smart object")
            .smart_filters
            .is_empty());
    }

    #[test]
    fn stored_slots_read_back() {
        let mut doc = smart_doc();
        assert!(lens_correction_slots(&doc, "0").is_none());
        assert!(apply_lens_correction(&mut doc, "0", &slots(35.0)));
        assert_eq!(lens_correction_slots(&doc, "0"), Some(slots(35.0)));
    }

    #[test]
    fn smart_source_buffer_reads_the_source() {
        let doc = smart_doc();
        let buffer = smart_source_buffer(&doc, "0").expect("renders");
        assert_eq!((buffer.width, buffer.height, buffer.channels), (4, 4, 4));
        assert_eq!(buffer.data[0], 0, "the source gradient, not the proxy");
        assert!(smart_source_buffer(&doc, "9").is_none());
    }
}
