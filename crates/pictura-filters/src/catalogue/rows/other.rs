//! Filter catalogue rows: other.

use crate::catalogue::{
    AlphaPolicy, ApronPolicy, DialogKind, FilterControl, FilterParam, FilterSpec, GpuPolicy,
};
use crate::filter::{FilterEngine, FilterKind};

#[rustfmt::skip]
pub(crate) const CUSTOM: FilterSpec = FilterSpec {
    kind: FilterKind::Custom,
    id: "custom",
    path: &["Filter", "Other", "Custom"],
    menu: true,
    label: "Custom",
    params: &[
        FilterParam { control: FilterControl::Matrix, label: "", minimum: -999.0, maximum: 999.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[ 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, ], },
        FilterParam { control: FilterControl::Number, label: "Scale:", minimum: 1.0, maximum: 9999.0, value: 1.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::Number, label: "Offset:", minimum: -9999.0, maximum: 9999.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
    ],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Never,
    apron: ApronPolicy::Fixed(128),
    gpu: GpuPolicy::CpuPreferred,
    gallery: None,
    build: crate::catalogue::build::stylize::build_custom,
    engine: FilterEngine::Native,
};

#[rustfmt::skip]
pub(crate) const HIGH_PASS: FilterSpec = FilterSpec {
    kind: FilterKind::HighPass,
    id: "high-pass",
    path: &["Filter", "Other", "High Pass"],
    menu: true,
    label: "High Pass",
    params: &[FilterParam { control: FilterControl::Slider, label: "Radius:", minimum: 0.1, maximum: 250.0, value: 4.0, decimals: 1, suffix: " pixels", pixel_distance: true, choices: &[], initial: &[], }],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Never,
    apron: ApronPolicy::RadiusCeil,
    gpu: GpuPolicy::CpuPreferred,
    gallery: None,
    build: crate::catalogue::build::stylize::build_high_pass,
    engine: FilterEngine::Native,
};

#[rustfmt::skip]
pub(crate) const MAXIMUM: FilterSpec = FilterSpec {
    kind: FilterKind::Maximum,
    id: "maximum",
    path: &["Filter", "Other", "Maximum"],
    menu: true,
    label: "Maximum",
    params: &[FilterParam { control: FilterControl::Slider, label: "Radius:", minimum: 1.0, maximum: 100.0, value: 2.0, decimals: 0, suffix: " pixels", pixel_distance: true, choices: &[], initial: &[], }],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Always,
    apron: ApronPolicy::IntRadius,
    gpu: GpuPolicy::CpuPreferred,
    gallery: None,
    build: crate::catalogue::build::stylize::build_maximum,
    engine: FilterEngine::Native,
};

#[rustfmt::skip]
pub(crate) const MINIMUM: FilterSpec = FilterSpec {
    kind: FilterKind::Minimum,
    id: "minimum",
    path: &["Filter", "Other", "Minimum"],
    menu: true,
    label: "Minimum",
    params: &[FilterParam { control: FilterControl::Slider, label: "Radius:", minimum: 1.0, maximum: 100.0, value: 2.0, decimals: 0, suffix: " pixels", pixel_distance: true, choices: &[], initial: &[], }],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Always,
    apron: ApronPolicy::IntRadius,
    gpu: GpuPolicy::CpuPreferred,
    gallery: None,
    build: crate::catalogue::build::stylize::build_minimum,
    engine: FilterEngine::Native,
};

#[rustfmt::skip]
pub(crate) const OFFSET: FilterSpec = FilterSpec {
    kind: FilterKind::Offset,
    id: "offset",
    path: &["Filter", "Other", "Offset"],
    menu: true,
    label: "Offset",
    params: &[
        FilterParam { control: FilterControl::Slider, label: "Horizontal:", minimum: -2000.0, maximum: 2000.0, value: 0.0, decimals: 0, suffix: " pixels right", pixel_distance: true, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::Slider, label: "Vertical:", minimum: -2000.0, maximum: 2000.0, value: 0.0, decimals: 0, suffix: " pixels down", pixel_distance: true, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::CheckBox, label: "Wrap Around", minimum: 0.0, maximum: 1.0, value: 1.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::CheckBox, label: "Repeat Edge Pixels", minimum: 0.0, maximum: 1.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
        FilterParam { control: FilterControl::Color, label: "Background:", minimum: 0.0, maximum: 255.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[0.0, 0.0, 0.0], },
        FilterParam { control: FilterControl::CheckBox, label: "Set to Transparent", minimum: 0.0, maximum: 1.0, value: 0.0, decimals: 0, suffix: "", pixel_distance: false, choices: &[], initial: &[], },
    ],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Always,
    apron: ApronPolicy::Offset,
    gpu: GpuPolicy::Unsupported,
    gallery: None,
    build: crate::catalogue::build::stylize::build_offset,
    engine: FilterEngine::Native,
};
