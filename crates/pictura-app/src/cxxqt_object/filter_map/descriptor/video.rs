//! Filter parameter descriptors: video.

use super::{DialogKind, FilterDescriptor};

// Generated from the former C++ table; kept compact so the data stays
// reviewable and under the file-size cap.
#[rustfmt::skip]
pub(super) const ROWS: &[FilterDescriptor] = &[
    FilterDescriptor {
        kind: "ntsc-colors",
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
    },
];
