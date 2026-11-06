//! The memory gate an edit passes before it allocates (#1207, #1211): it
//! estimates the bytes the edit will hold at its peak and asks the process's
//! memory governor (`pictura_core::memory`), which refuses them when they
//! exceed the room under its ceiling, instead of leaving the process to the
//! kernel's OOM killer.
//!
//! GUI thread only: the refusal record and the test host are the governor's
//! thread-locals, so an edit gated on a worker thread would not report its
//! refusal to the bridge, which reads it on the GUI thread.

use pictura_core::memory::{self, HostReading};
use pictura_core::{BitDepth, Document, PsdRect, BAND_ROWS};

/// Forget the last refusal; every gated edit starts here.
pub(in crate::cxxqt_object) fn clear() {
    memory::clear_refusal();
}

/// Whether `bytes` more fit the governor's room now. A refusal is remembered
/// for [`edit_refused_for_memory`].
pub(in crate::cxxqt_object) fn admit(bytes: u64) -> bool {
    memory::governor().admit(bytes, "an edit")
}

/// [`clear`], then [`admit`]: the gate of a path that runs no other gate.
pub(in crate::cxxqt_object) fn admit_fresh(bytes: u64) -> bool {
    clear();
    admit(bytes)
}

/// The pixels of `bounds`.
pub(in crate::cxxqt_object) fn area(bounds: PsdRect) -> u64 {
    bounds.width().max(0) as u64 * bounds.height().max(0) as u64
}

/// The bytes a write of `bounds` into the layer at `path` forks from history:
/// each 64-row band it crosses, of each colour and transparency plane, at the
/// layer's width.
///
/// ponytail: 8-bit planes only; a 16- or 32-bit native store written beside
/// them is not charged, so the gate under-counts a deep document's fill.
pub(in crate::cxxqt_object) fn fork_bytes(doc: &Document, path: &str, bounds: PsdRect) -> u64 {
    let Some(layer) = pictura_render::resolve_path(doc, path) else {
        return 0;
    };
    // The fill grows the layer to its box first (#1208), so the bands it forks
    // are the grown rect's, not the old rect's.
    let rect = if layer.rect.is_empty() {
        bounds
    } else {
        layer.rect.union(bounds)
    };
    let Some(hit) = bounds.intersect(rect) else {
        return 0;
    };
    let band = BAND_ROWS as i32;
    let first = (hit.top - rect.top) / band * band;
    let last = ((hit.bottom - rect.top + band - 1) / band * band).min(rect.height());
    let planes = layer.channels.iter().filter(|c| c.id >= -1).count() as u64;
    planes * (last - first).max(0) as u64 * rect.width().max(0) as u64
}

// The estimates of the paths the memory benchmark drives (#1211). Each is
// fitted to its scenario's peak resident size added at 4250² (the process
// benchmark, `scripts/profile-tools.py --scenarios memory --peak`), which
// counts the shell's display buffers beside the engine's heap, rounded up;
// `estimates_cover_the_measured_peaks` keeps each at or above its scenario's
// heap peak. They approximate this engine's allocations, not Photoshop's.

fn sample_bytes(depth: BitDepth) -> u64 {
    match depth {
        BitDepth::One | BitDepth::Eight => 1,
        BitDepth::Sixteen => 2,
        BitDepth::ThirtyTwo => 4,
    }
}

fn canvas(doc: &Document) -> u64 {
    u64::from(doc.width) * u64::from(doc.height)
}

/// New Document (M1): the base layer, the composite, and the display and its
/// pyramid (21.4 bytes a pixel for RGB at 4250²).
pub(in crate::cxxqt_object) fn new_document_bytes(
    width: u32,
    height: u32,
    color_channels: u8,
    depth: BitDepth,
) -> u64 {
    let per_pixel = sample_bytes(depth) * (2 * u64::from(color_channels) + 1) + 17;
    u64::from(width) * u64::from(height) * per_pixel
}

/// Add Layer (M2): an empty-rect layer stores no pixel planes, only its record
/// and history state (#1208).
pub(in crate::cxxqt_object) fn add_layer_bytes(_doc: &Document) -> u64 {
    64 * 1024
}

/// A brush stroke's start (M6): the target layer's planes and the stroke's
/// canvas-sized buffers.
pub(in crate::cxxqt_object) fn brush_bytes(doc: &Document, path: &str) -> u64 {
    let layer = pictura_render::resolve_path(doc, path)
        .map_or(0, |l| l.channels.iter().map(|c| c.data.len() as u64).sum());
    layer + 3 * canvas(doc)
}

/// Undo, Redo or a history jump (M7): the restored composite and the display
/// (13.1 bytes a pixel at 4250²).
pub(in crate::cxxqt_object) fn restore_bytes(doc: &Document) -> u64 {
    doc.composite.data.len() as u64 + 12 * canvas(doc)
}

/// The display refresh after a fill of `bounds` (M5): the region's composite
/// and display bytes, beside what the fill itself holds (10.9 bytes a pixel for
/// a whole-layer bucket at 4250², of which the fill's own planes are 6).
pub(in crate::cxxqt_object) fn refresh_bytes(bounds: PsdRect) -> u64 {
    6 * area(bounds)
}

/// Adding a named snapshot (M8).
pub(in crate::cxxqt_object) fn snapshot_bytes(doc: &Document) -> u64 {
    crate::history::document_bytes(doc) + 4 * canvas(doc)
}

/// Restoring a named snapshot (M8): its detached copy and a full composite.
pub(in crate::cxxqt_object) fn snapshot_restore_bytes(doc: &Document) -> u64 {
    crate::history::document_bytes(doc) + 10 * canvas(doc)
}

/// Save (M11): the save job's copy and the encoder's buffers.
pub(in crate::cxxqt_object) fn save_bytes(doc: &Document) -> u64 {
    crate::history::document_bytes(doc) + 8 * canvas(doc)
}

/// Whether the last gated edit was refused because it did not fit the
/// governor's room.
pub(in crate::cxxqt_object) fn edit_refused_for_memory() -> bool {
    memory::last_refusal().is_some()
}

/// Pin the host this thread's governor sees to `bytes` of available memory
/// and nothing else; a negative value reads the host again.
pub(in crate::cxxqt_object) fn set_available_memory_for_test(bytes: i64) {
    memory::set_host_for_test(u64::try_from(bytes).ok().map(|available| HostReading {
        available: Some(available),
        ..HostReading::default()
    }));
}

/// Pin the host to a cgroup limited to `max` bytes with `current` in use; a
/// negative `max` reads the host again.
pub(in crate::cxxqt_object) fn set_cgroup_for_test(max: i64, current: i64) {
    memory::set_host_for_test(u64::try_from(max).ok().map(|max| HostReading {
        cgroup_max: Some(max),
        cgroup_current: Some(current.max(0) as u64),
        ..HostReading::default()
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: i64 = 1024 * 1024;

    #[test]
    fn a_refusal_is_remembered_until_the_next_fill() {
        // 1 GiB available leaves 768 MiB of room.
        set_available_memory_for_test(1024 * MIB);
        clear();
        assert!(admit(700 * MIB as u64));
        assert!(!edit_refused_for_memory());
        assert!(!admit(800 * MIB as u64));
        assert!(edit_refused_for_memory());
        clear();
        assert!(!edit_refused_for_memory());
        set_available_memory_for_test(-1);
    }

    #[test]
    fn a_write_forks_the_bands_it_crosses() {
        let mut doc = Document::new(
            100,
            300,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
                "layer",
                100,
                300,
                (0, 0, 0),
            )]);
        let planes = doc.layers[0].channels.iter().filter(|c| c.id >= -1).count() as u64;
        let rows = |top, bottom| PsdRect {
            top,
            left: 10,
            bottom,
            right: 20,
        };
        // Rows 70..140 cross bands 1 and 2: rows 64..192.
        assert_eq!(fork_bytes(&doc, "0", rows(70, 140)), planes * 128 * 100);
        // The last band is cut at the layer's bottom.
        assert_eq!(fork_bytes(&doc, "0", rows(290, 300)), planes * 44 * 100);
        // A box off the layer grows it to the union: 100×500, rows 384..500.
        assert_eq!(fork_bytes(&doc, "0", rows(400, 500)), planes * 116 * 100);
        assert_eq!(fork_bytes(&doc, "9", rows(0, 10)), 0, "no such layer");
    }

    #[test]
    fn a_fill_that_grows_the_layer_forks_the_grown_bands() {
        let mut doc = Document::new(
            100,
            300,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers =
            pictura_core::LayerStack::from_layers(vec![crate::cxxqt_object::tests::pixel_layer(
                "layer",
                20,
                20,
                (0, 0, 0),
            )]);
        // The layer is 20×20 at the origin; a fill box at (80, 80)–(120, 140)
        // grows it to 120×140, whose bands the fill forks.
        let bounds = PsdRect {
            top: 80,
            left: 80,
            bottom: 140,
            right: 120,
        };
        let planes = doc.layers[0].channels.iter().filter(|c| c.id >= -1).count() as u64;
        // Rows 80..140 cross bands 64..128 and 128..140, at the grown width.
        assert_eq!(fork_bytes(&doc, "0", bounds), planes * 76 * 120);
    }
}
