//! Filter catalogue rows: video.

use crate::catalogue::{
    AlphaPolicy, ApronPolicy, DialogKind, FilterControl, FilterParam, FilterSpec, GpuPolicy,
};
use crate::filter::{FilterEngine, FilterKind};

#[rustfmt::skip]
pub(crate) const NTSC_COLORS: FilterSpec = FilterSpec {
    kind: FilterKind::NtscColors,
    id: "ntsc-colors",
    path: &["Filter", "Video", "NTSC Colors"],
    menu: true,
    label: "NTSC Colors",
    params: &[],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Never,
    apron: ApronPolicy::Fixed(128),
    gpu: GpuPolicy::Unsupported,
    gallery: None,
    build: crate::catalogue::build::video::build_ntsc_colors,
    engine: FilterEngine::Native,
};

/// Row parity is the layer's absolute row index, so a cropped section preview
/// would eliminate the wrong field; the preview resolves the whole layer.
#[rustfmt::skip]
pub(crate) const DE_INTERLACE: FilterSpec = FilterSpec {
    kind: FilterKind::DeInterlace,
    id: "de-interlace",
    path: &["Filter", "Video", "De-Interlace"],
    menu: true,
    label: "De-Interlace",
    params: &[
        FilterParam { control: FilterControl::Radio, label: "Eliminate:", minimum: 0.0, maximum: 1.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &["Odd Fields", "Even Fields"], initial: &[] },
        FilterParam { control: FilterControl::Radio, label: "Create New Fields by:", minimum: 0.0, maximum: 1.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &["Interpolation", "Duplication"], initial: &[] },
    ],
    whole_layer: true,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Always,
    apron: ApronPolicy::Fixed(8),
    gpu: GpuPolicy::Unsupported,
    gallery: None,
    build: crate::catalogue::build::video::build_de_interlace,
    engine: FilterEngine::Native,
};
