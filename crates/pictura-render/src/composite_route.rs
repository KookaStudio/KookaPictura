//! Which content path composites a layer. The CPU compositor dispatches on
//! [`content_route`] and the GPU eligibility check matches the same enum, both
//! without a wildcard arm, so a new CPU content path cannot ship without a GPU
//! decision.
//!
//! Only the content is classified. Effects render around any content, clipping
//! and knockout are decided by the caller, and Blend If, Dissolve, and text
//! gamma apply inside the blend; each stays an attribute check.

use pictura_adjust::Adjustment;
use pictura_core::{BlendMode, Layer};

use crate::smart_filter::decode_smart_filter;

pub(crate) enum ContentRoute {
    /// A group that recurses onto the running canvas (exact only at opacity
    /// 255 with no mask).
    PassThroughGroup,
    /// A group composited into its own canvas, then blended as one layer.
    IsolatedGroup,
    /// A fill-content or adjustment layer whose block decodes.
    Fill(Adjustment),
    /// An adjustment block that does not decode: a CPU no-op.
    UndecodedAdjustment,
    /// A smart object whose enabled filters all decode: the CPU re-renders
    /// the embedded source through them and ignores the pixel proxy.
    SmartFiltered,
    /// A smart object without a pixel proxy, rendered from its source.
    SmartSource,
    /// A type layer without a pixel proxy, rendered from its text model.
    TypeSource,
    /// A `vscg` vector fill that does not decode, on a layer with no
    /// adjustment block. The CPU draws it as plain pixels.
    UndecodedVectorFill,
    Pixels,
}

/// The smart-filtered route: at least one individually enabled filter, and
/// every enabled filter one this engine models. The group's master switch does
/// not matter, because the route still re-renders the source.
pub(crate) fn smart_filters_apply(layer: &Layer) -> bool {
    let Some(so) = layer.smart_object.as_ref() else {
        return false;
    };
    let mut enabled = 0usize;
    for filter in so.smart_filters.iter().filter(|f| f.enabled) {
        if decode_smart_filter(filter).is_none() {
            return false;
        }
        enabled += 1;
    }
    enabled > 0
}

pub(crate) fn content_route(layer: &Layer) -> ContentRoute {
    if layer.is_group {
        return if matches!(layer.blend, BlendMode::PassThrough)
            && layer.opacity == 255
            && layer.mask.is_none()
            && layer.vector_mask.is_none()
        {
            ContentRoute::PassThroughGroup
        } else {
            ContentRoute::IsolatedGroup
        };
    }
    if let Some(adjustment) = crate::fill::decode_layer_fill(layer) {
        return ContentRoute::Fill(adjustment);
    }
    if layer.adjustment.is_some() {
        return ContentRoute::UndecodedAdjustment;
    }
    if smart_filters_apply(layer) {
        return ContentRoute::SmartFiltered;
    }
    let proxy = crate::channel_plane(layer, 0).is_some();
    if layer.smart_object.is_some() && !proxy {
        return ContentRoute::SmartSource;
    }
    if layer.type_tool.is_some() && !proxy {
        return ContentRoute::TypeSource;
    }
    if layer.extra_block(b"vscg").is_some() {
        return ContentRoute::UndecodedVectorFill;
    }
    ContentRoute::Pixels
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use pictura_codec::CAMERA_RAW_FILTER_ID;
    use pictura_core::{
        AdjustmentData, Channel, LayerBlock, PsdRect, SmartFilter, SmartObject, TypeTool,
    };

    /// The CPU content path before the route existed, verbatim: the oracle for
    /// the classification. `UndecodedVectorFill` reaches `composite_pixels`.
    fn legacy_path(layer: &Layer) -> &'static str {
        if layer.is_group {
            if matches!(layer.blend, BlendMode::PassThrough)
                && layer.opacity == 255
                && layer.mask.is_none()
                && layer.vector_mask.is_none()
            {
                "pass-through"
            } else {
                "isolated"
            }
        } else if crate::fill::decode_layer_fill(layer).is_some() {
            "fill"
        } else if layer.adjustment.is_some() {
            "no-op"
        } else if smart_filters_apply_legacy(layer) {
            "smart-filtered"
        } else if layer.smart_object.is_some() && crate::channel(layer, 0).is_none() {
            "smart-source"
        } else if layer.type_tool.is_some() && crate::channel(layer, 0).is_none() {
            "type-source"
        } else {
            "pixels"
        }
    }

    /// The loop `composite_smart_filtered_source` ran before the predicate.
    fn smart_filters_apply_legacy(layer: &Layer) -> bool {
        let Some(so) = layer.smart_object.as_ref() else {
            return false;
        };
        let mut enabled = 0usize;
        for filter in &so.smart_filters {
            if !filter.enabled {
                continue;
            }
            if decode_smart_filter(filter).is_none() {
                return false;
            }
            enabled += 1;
        }
        enabled > 0
    }

    fn cpu_path(route: &ContentRoute) -> &'static str {
        match route {
            ContentRoute::PassThroughGroup => "pass-through",
            ContentRoute::IsolatedGroup => "isolated",
            ContentRoute::Fill(_) => "fill",
            ContentRoute::UndecodedAdjustment => "no-op",
            ContentRoute::SmartFiltered => "smart-filtered",
            ContentRoute::SmartSource => "smart-source",
            ContentRoute::TypeSource => "type-source",
            ContentRoute::UndecodedVectorFill | ContentRoute::Pixels => "pixels",
        }
    }

    pub(crate) fn rect() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        }
    }

    pub(crate) fn proxy() -> Vec<Channel> {
        [0i16, 1, 2, -1]
            .map(|id| Channel {
                id,
                data: vec![90u8; 4].into(),
            })
            .to_vec()
    }

    pub(crate) fn filter(filter_id: i32, enabled: bool) -> SmartFilter {
        SmartFilter {
            filter_id,
            name: "f".into(),
            enabled,
            options: Vec::new().into(),
        }
    }

    pub(crate) fn smart(channels: Vec<Channel>, filters: Vec<SmartFilter>, master: bool) -> Layer {
        Layer {
            rect: rect(),
            channels,
            smart_object: Some({
                let mut so = SmartObject::default();
                so.smart_filters = filters;
                so.smart_filters_enabled = master;
                so
            }),
            ..Default::default()
        }
    }

    pub(crate) fn type_layer(channels: Vec<Channel>) -> Layer {
        Layer {
            rect: rect(),
            channels,
            type_tool: Some(TypeTool {
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                text: "Hi".into(),
                bounds: [0, 0, 2, 2],
                text_desc: Vec::new().into(),
                warp_desc: Vec::new().into(),
                fonts: vec!["Arial".into()],
                style: None,
                vertical: false,
                box_bounds: None,
                path: None,
            }),
            ..Default::default()
        }
    }

    pub(crate) fn adjustment(key: [u8; 4], data: Vec<u8>) -> Layer {
        Layer {
            rect: rect(),
            adjustment: Some(AdjustmentData {
                key,
                data: data.into(),
            }),
            ..Default::default()
        }
    }

    pub(crate) fn corpus() -> Vec<Layer> {
        let group = |blend, opacity| Layer {
            is_group: true,
            blend,
            opacity,
            ..Default::default()
        };
        vec![
            group(BlendMode::PassThrough, 255),
            group(BlendMode::PassThrough, 128),
            group(BlendMode::Normal, 255),
            adjustment(*b"nvrt", Vec::new()),
            adjustment(*b"zzzz", vec![1, 2, 3]),
            Layer {
                rect: rect(),
                extra_blocks: vec![LayerBlock {
                    key: *b"vscg",
                    data: vec![0xFF; 3].into(),
                }],
                ..Default::default()
            },
            smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], true),
            smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], false),
            smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, false)], true),
            smart(proxy(), vec![filter(1, true)], true),
            smart(Vec::new(), Vec::new(), true),
            smart(proxy(), Vec::new(), true),
            type_layer(Vec::new()),
            type_layer(proxy()),
            Layer {
                rect: rect(),
                channels: proxy(),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn content_route_matches_the_legacy_chain() {
        let mut seen = std::collections::BTreeSet::new();
        for layer in corpus() {
            let route = content_route(&layer);
            assert_eq!(cpu_path(&route), legacy_path(&layer), "{layer:?}");
            seen.insert(cpu_path(&route));
        }
        assert_eq!(seen.len(), 8, "the corpus reaches every CPU path");
    }

    #[test]
    fn the_filter_master_switch_does_not_change_the_route() {
        let off = smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], false);
        assert!(matches!(content_route(&off), ContentRoute::SmartFiltered));
    }
}
