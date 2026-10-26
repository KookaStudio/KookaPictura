//! Filter catalogue rows: video.

use crate::catalogue::{AlphaPolicy, ApronPolicy, DialogKind, FilterSpec, GpuPolicy};
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
