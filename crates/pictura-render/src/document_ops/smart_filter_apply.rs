//! Apply a Filter-menu filter, or a Filter Gallery stack, to a Smart Object as
//! a Smart Filter: the generic Pictura record is appended to the object's
//! chain and the compositor renders it live over the unfiltered source, so the
//! proxy pixels are never touched.

use pictura_codec::{FilterStep, FILTER_GALLERY_NAME, PICTURA_FILTER_ID};
use pictura_core::{Document, FilterMask, SmartFilter};

use super::layer_ops::resolve_path_mut;

/// The generic Smart Filter record for `steps`, named after the first step's
/// catalogue label (without its ellipsis), or "Filter Gallery" for a gallery
/// stack. `None` when a step is not smart-capable or its slots do not build.
pub fn smart_filter_record(steps: &[FilterStep], gallery: bool) -> Option<SmartFilter> {
    let name = if gallery {
        FILTER_GALLERY_NAME.to_string()
    } else {
        let spec = pictura_filters::catalogue::descriptor(&steps.first()?.kind)?;
        spec.label.trim_end_matches('…').trim_end().to_string()
    };
    let filter = SmartFilter {
        filter_id: PICTURA_FILTER_ID,
        name,
        enabled: true,
        options: pictura_codec::encode_pictura_filter(steps, gallery),
    };
    crate::smart_filter::decode_smart_filter(&filter).map(|_| filter)
}

/// Append `filter` to the embedded Smart Object at `path`.
///
/// When the object has no filter mask and `selection` is given, the selection
/// becomes the mask and the mask is enabled; an existing mask is kept. Returns
/// `false` without mutating for a target that fails
/// [`crate::can_replace_smart_object_contents`] or a codec refusal (a legacy
/// placed layer).
pub fn apply_smart_filter(
    doc: &mut Document,
    path: &str,
    filter: SmartFilter,
    selection: Option<FilterMask>,
) -> bool {
    if !crate::can_replace_smart_object_contents(doc, path) {
        return false;
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if pictura_codec::append_smart_filter(layer, filter).is_err() {
        return false;
    }
    let Some(so) = layer.smart_object.as_mut() else {
        return false;
    };
    if so.filter_mask.is_none() {
        if let Some(mask) = selection {
            so.filter_mask = Some(mask);
            let _ = pictura_codec::set_filter_mask_enabled(layer, true);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_ops::layer_ops::resolve_path;
    use pictura_core::{BitDepth, Channel, ColorMode, Layer, LayerBlock, PsdRect, SmartObjectKind};

    fn gradient_layer() -> Layer {
        let mut layer = Layer {
            name: "Layer".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            ..Default::default()
        };
        for id in 0..3i16 {
            let data: Vec<u8> = (0..64)
                .map(|i| ((i % 8) * 30 + (i / 8) * (id as usize + 1) * 3) as u8)
                .collect();
            layer.channels.push(Channel {
                id,
                data: data.into(),
            });
        }
        layer
    }

    fn smart_doc() -> Document {
        let mut doc = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![gradient_layer()]);
        assert!(crate::convert_to_smart_object(&mut doc, "0"));
        doc
    }

    fn blur(radius: f64) -> SmartFilter {
        smart_filter_record(
            &[FilterStep {
                kind: "gaussian-blur".into(),
                slots: vec![radius],
            }],
            false,
        )
        .expect("record")
    }

    fn left_half() -> FilterMask {
        let data: Vec<u8> = (0..64).map(|i| if i % 8 < 4 { 255 } else { 0 }).collect();
        FilterMask {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 8,
                right: 8,
            },
            data: data.into(),
        }
    }

    fn channels(doc: &Document) -> Vec<Vec<u8>> {
        doc.layers[0]
            .channels
            .iter()
            .map(|c| c.data.to_vec())
            .collect()
    }

    fn so(doc: &Document) -> &pictura_core::SmartObject {
        resolve_path(doc, "0")
            .and_then(|l| l.smart_object.as_ref())
            .expect("smart object")
    }

    #[test]
    fn apply_appends_and_keeps_proxy() {
        let mut doc = smart_doc();
        let before = channels(&doc);
        let plain = crate::composite_rgba(&doc);
        assert_eq!(blur(2.0).name, "Gaussian Blur");
        assert!(apply_smart_filter(&mut doc, "0", blur(2.0), None));
        assert!(apply_smart_filter(&mut doc, "0", blur(2.0), None));
        assert_eq!(so(&doc).smart_filters.len(), 2, "appended, not replaced");
        assert_eq!(channels(&doc), before, "the proxy is untouched");
        assert_ne!(
            crate::composite_rgba(&doc).data,
            plain.data,
            "the blur shows"
        );
    }

    #[test]
    fn selection_becomes_mask() {
        let mut doc = smart_doc();
        assert!(apply_smart_filter(
            &mut doc,
            "0",
            blur(2.0),
            Some(left_half())
        ));
        let so = so(&doc);
        assert!(so.filter_mask_enabled);
        assert_eq!(
            so.filter_mask.as_ref().unwrap().data.to_vec(),
            left_half().data.to_vec()
        );
    }

    #[test]
    fn existing_mask_kept() {
        let mut doc = smart_doc();
        assert!(apply_smart_filter(
            &mut doc,
            "0",
            blur(1.0),
            Some(left_half())
        ));
        let mut other = left_half();
        other.data = vec![7u8; 64].into();
        assert!(apply_smart_filter(&mut doc, "0", blur(2.0), Some(other)));
        assert_eq!(
            so(&doc).filter_mask.as_ref().unwrap().data.to_vec(),
            left_half().data.to_vec()
        );
    }

    #[test]
    fn no_selection_no_mask() {
        let mut doc = smart_doc();
        assert!(apply_smart_filter(&mut doc, "0", blur(2.0), None));
        assert!(so(&doc).filter_mask.is_none());
    }

    #[test]
    fn refuses_ineligible() {
        let mut missing = smart_doc();
        assert!(!apply_smart_filter(&mut missing, "9", blur(2.0), None));

        let mut raster = Document::new(8, 8, ColorMode::Rgb, BitDepth::Eight);
        raster.layers = pictura_core::LayerStack::from_layers(vec![gradient_layer()]);
        assert!(!apply_smart_filter(&mut raster, "0", blur(2.0), None));

        let mut linked = smart_doc();
        linked.layers[0].smart_object.as_mut().unwrap().kind = SmartObjectKind::External;
        assert!(!apply_smart_filter(&mut linked, "0", blur(2.0), None));
        assert!(so(&linked).smart_filters.is_empty());

        let mut legacy = smart_doc();
        legacy.layers[0]
            .extra_blocks
            .retain(|b| &b.key != b"SoLd" && &b.key != b"SoLE");
        legacy.layers[0].extra_blocks.push(LayerBlock {
            key: *b"PlLd",
            data: vec![0; 8],
        });
        assert!(!apply_smart_filter(&mut legacy, "0", blur(2.0), None));
        assert!(so(&legacy).smart_filters.is_empty());

        assert!(
            smart_filter_record(
                &[FilterStep {
                    kind: "liquify".into(),
                    slots: Vec::new(),
                }],
                false,
            )
            .is_none(),
            "an excluded kind makes no record"
        );
    }

    #[test]
    fn save_reopen_keeps_filters_and_mask() {
        let mut doc = smart_doc();
        assert!(apply_smart_filter(
            &mut doc,
            "0",
            blur(2.0),
            Some(left_half())
        ));
        let emboss = smart_filter_record(
            &[FilterStep {
                kind: "emboss".into(),
                slots: pictura_filters::catalogue::descriptor("emboss")
                    .unwrap()
                    .defaults(),
            }],
            false,
        )
        .expect("record");
        assert!(apply_smart_filter(&mut doc, "0", emboss, None));
        let before = crate::composite_rgba(&doc);

        let back = pictura_codec::read_psd(&pictura_codec::write_psd(&doc).expect("writes"))
            .expect("reads");
        let (a, b) = (so(&doc), so(&back));
        assert_eq!(b.smart_filters.len(), 2);
        for (x, y) in a.smart_filters.iter().zip(&b.smart_filters) {
            assert_eq!(
                (x.filter_id, &x.name, &x.options),
                (y.filter_id, &y.name, &y.options)
            );
        }
        assert!(b.filter_mask_enabled);
        assert_eq!(
            b.filter_mask.as_ref().map(|m| m.data.to_vec()),
            a.filter_mask.as_ref().map(|m| m.data.to_vec())
        );
        assert_eq!(crate::composite_rgba(&back).data, before.data);
    }
}
