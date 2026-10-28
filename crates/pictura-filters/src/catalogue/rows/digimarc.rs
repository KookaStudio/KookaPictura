//! Filter catalogue rows: Digimarc (the in-tree approximate watermark).

use crate::catalogue::{
    AlphaPolicy, ApronPolicy, DialogKind, FilterControl, FilterParam, FilterSpec, GpuPolicy,
};
use crate::filter::{FilterEngine, FilterKind};

#[rustfmt::skip]
pub(crate) const EMBED_WATERMARK: FilterSpec = FilterSpec {
    kind: FilterKind::EmbedWatermark,
    id: "embed-watermark",
    path: &["Filter", "Digimarc", "Embed Watermark"],
    menu: true,
    label: "Embed Watermark",
    params: &[
        FilterParam { control: FilterControl::Choice, label: "Watermark Type:", minimum: 0.0, maximum: 1.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &["Image ID", "Transaction ID"], initial: &[], },
        FilterParam { control: FilterControl::Number, label: "Creator ID:", minimum: 0.0, maximum: 8388607.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::Number, label: "Image ID:", minimum: 0.0, maximum: 16777215.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::Radio, label: "Watermark Durability:", minimum: 0.0, maximum: 2.0, value: 1.0, decimals: 0, suffix: "", pixel_distance: false, choices: &["Low", "Medium", "High"], initial: &[], },
    ],
    whole_layer: true,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Never,
    apron: ApronPolicy::Fixed(16),
    gpu: GpuPolicy::Unsupported,
    gallery: None,
    build: crate::catalogue::build::digimarc::build_embed_watermark,
    engine: FilterEngine::Native,
};
