//! Filter catalogue rows: engine-only kinds.

use crate::catalogue::{AlphaPolicy, ApronPolicy, DialogKind, FilterSpec, GpuPolicy};
use crate::filter::{FilterEngine, FilterKind};

#[rustfmt::skip]
pub(crate) const HDR_TONING: FilterSpec = FilterSpec {
    kind: FilterKind::HdrToning,
    id: "hdr-toning",
    path: &[],
    menu: false,
    label: "HDR Toning",
    params: &[],
    whole_layer: false,
    dialog: DialogKind::Shared,
    map_after_ok: false,
    preview_pane: true,
    stacked: false,
    preview_below: false,
    live_preview: true,
    alpha: AlphaPolicy::Never,
    apron: ApronPolicy::RadiusTimes3,
    gpu: GpuPolicy::Unsupported,
    gallery: None,
    build: crate::catalogue::build::engine_only::build_hdr_toning,
    engine: FilterEngine::Native,
};
