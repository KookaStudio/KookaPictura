//! Add, delete, reorder, and clear the smart filters on a smart-object layer.
//!
//! Thin path-resolving wrappers over the `pictura-codec` descriptor edits, so
//! each document op rewrites the preserved `SoLd`/`SoLE` `filterFX` list and the
//! typed `SmartObject.smart_filters` view together. Adding uses the codec's
//! generic attach, which replaces a same-`filterID` entry rather than stacking a
//! duplicate. ponytail: a same-id stack is not modelled; revisit if CS6 stacking
//! of one filter type is needed.

use pictura_core::{Document, SmartFilter};

use super::layer_ops::{resolve_path, resolve_path_mut};

/// Whether `path` resolves to a layer carrying a smart object that can take a
/// smart filter: not a group, no adjustment data, and a typed object.
pub fn can_add_smart_filter(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| {
        !layer.is_group && layer.adjustment.is_none() && layer.smart_object.is_some()
    })
}

/// Append (or replace, by `filter_id`) a smart filter on the layer at `path`.
///
/// Returns `false` without mutating for an ineligible target or a codec refusal
/// (a non-embedded object or a legacy placed layer).
pub fn add_smart_filter(
    doc: &mut Document,
    path: &str,
    filter_id: i32,
    name: &str,
    options: &[u8],
) -> bool {
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if layer.is_group || layer.adjustment.is_some() || layer.smart_object.is_none() {
        return false;
    }
    pictura_codec::attach_smart_filter(
        layer,
        SmartFilter {
            filter_id,
            name: name.to_string(),
            enabled: true,
            options: options.to_vec(),
        },
    )
    .is_ok()
}

/// Delete the smart filter at `index` on the layer at `path`.
pub fn delete_smart_filter(doc: &mut Document, path: &str, index: usize) -> bool {
    resolve_path_mut(doc, path).is_some_and(|layer| {
        layer.smart_object.is_some() && pictura_codec::delete_smart_filter(layer, index).is_ok()
    })
}

/// Move the smart filter at `from` to `to` on the layer at `path`.
pub fn reorder_smart_filters(doc: &mut Document, path: &str, from: usize, to: usize) -> bool {
    resolve_path_mut(doc, path).is_some_and(|layer| {
        layer.smart_object.is_some()
            && pictura_codec::reorder_smart_filters(layer, from, to).is_ok()
    })
}

/// Remove every smart filter from the layer at `path`.
pub fn clear_smart_filters(doc: &mut Document, path: &str) -> bool {
    resolve_path_mut(doc, path).is_some_and(|layer| {
        layer.smart_object.is_some() && pictura_codec::clear_smart_filters(layer).is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, Channel, ColorMode, Layer, PsdRect};

    fn raster_layer() -> Layer {
        let mut layer = Layer {
            name: "Layer".to_string(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            ..Default::default()
        };
        for id in 0..3i16 {
            layer.channels.push(Channel {
                id,
                data: vec![60u8; 16].into(),
            });
        }
        layer
    }

    fn smart_doc() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![raster_layer()]);
        assert!(crate::convert_to_smart_object(&mut doc, "0"));
        doc
    }

    fn filter_ids(doc: &Document) -> Vec<i32> {
        resolve_path(doc, "0")
            .unwrap()
            .smart_object
            .as_ref()
            .unwrap()
            .smart_filters
            .iter()
            .map(|f| f.filter_id)
            .collect()
    }

    #[test]
    fn add_delete_reorder_and_clear_track_the_typed_list() {
        let mut doc = smart_doc();
        assert!(add_smart_filter(
            &mut doc,
            "0",
            2683,
            "Camera Raw Filter",
            &[]
        ));
        assert!(add_smart_filter(&mut doc, "0", 4242, "Second", &[]));
        assert_eq!(filter_ids(&doc), vec![2683, 4242]);

        assert!(reorder_smart_filters(&mut doc, "0", 0, 1));
        assert_eq!(filter_ids(&doc), vec![4242, 2683]);

        assert!(delete_smart_filter(&mut doc, "0", 0));
        assert!(clear_smart_filters(&mut doc, "0"));
        assert!(filter_ids(&doc).is_empty());
    }

    #[test]
    fn ops_refuse_a_non_smart_layer() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![raster_layer()]);
        assert!(!add_smart_filter(&mut doc, "0", 2683, "x", &[]));
        assert!(!delete_smart_filter(&mut doc, "0", 0));
        assert!(!reorder_smart_filters(&mut doc, "0", 0, 1));
        assert!(!clear_smart_filters(&mut doc, "0"));
        assert!(!can_add_smart_filter(&doc, "0"));
    }
}
