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
use pictura_core::{BitDepth, Document, PsdRect, TILE};

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

/// The most a tiled mask over `bounds` holds (#1214): for each 256² tile the
/// box crosses, a dense tile and two bitmaps of one bit a pixel (the flood's
/// visited state and its match answers). Uniform tiles hold none of it, so a
/// flood over one colour costs far less.
pub(in crate::cxxqt_object) fn flood_bytes(bounds: PsdRect) -> u64 {
    let tile = TILE as i64;
    let span = |from: i32, to: i32| {
        let (from, to) = (i64::from(from).max(0), i64::from(to).max(0));
        if to <= from {
            return 0;
        }
        (to + tile - 1) / tile - from / tile
    };
    let tiles = span(bounds.left, bounds.right) * span(bounds.top, bounds.bottom);
    let per_tile = (TILE * TILE + 2 * TILE * TILE / 8) as u64;
    tiles.max(0) as u64 * per_tile
}

/// The bytes a write of `bounds` into the layer at `path` forks from history:
/// each 256² tile it crosses, of each colour and transparency plane, at the
/// document's sample size (#1213).
///
/// ponytail: a 16- or 32-bit native store stays flat, so a write that also
/// rewrites it copies the whole store; that copy is not charged here.
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
    let tile = TILE as i32;
    let span = |from: i32, to: i32, extent: i32| {
        let first = from / tile * tile;
        let last = ((to + tile - 1) / tile * tile).min(extent);
        (last - first).max(0) as u64
    };
    let rows = span(hit.top - rect.top, hit.bottom - rect.top, rect.height());
    let cols = span(hit.left - rect.left, hit.right - rect.left, rect.width());
    let planes = layer.channels.iter().filter(|c| c.id >= -1).count() as u64;
    planes * rows * cols * sample_bytes(doc.depth)
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

/// New Document (M1): the rendered composite and the display and its pyramid;
/// the base layer and the stored composite are uniform tiles (#1213).
pub(in crate::cxxqt_object) fn new_document_bytes(
    width: u32,
    height: u32,
    color_channels: u8,
    depth: BitDepth,
) -> u64 {
    // The base layer and the stored composite start as uniform tiles and hold
    // nothing (#1213); the rendered composite and the display remain.
    let per_pixel = sample_bytes(depth) * u64::from(color_channels) + 17;
    u64::from(width) * u64::from(height) * per_pixel
}

/// Add Layer (M2): an empty-rect layer stores no pixel planes, only its record
/// and history state (#1208).
pub(in crate::cxxqt_object) fn add_layer_bytes(_doc: &Document) -> u64 {
    256 * 1024
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
    fn a_write_forks_the_tiles_it_crosses() {
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
        // Rows 70..140 lie in tile row 0: 256 rows, clipped to the 100 columns.
        assert_eq!(fork_bytes(&doc, "0", rows(70, 140)), planes * 256 * 100);
        // The last tile row is cut at the layer's bottom: rows 256..300.
        assert_eq!(fork_bytes(&doc, "0", rows(290, 300)), planes * 44 * 100);
        // A box off the layer grows it to the union: 100×500, rows 256..500.
        assert_eq!(fork_bytes(&doc, "0", rows(400, 500)), planes * 244 * 100);
        assert_eq!(fork_bytes(&doc, "9", rows(0, 10)), 0, "no such layer");
    }

    // [tp_gate]
    #[test]
    fn fork_bytes_counts_tiles() {
        for (depth, bytes) in [
            (pictura_core::BitDepth::Eight, 1),
            (pictura_core::BitDepth::Sixteen, 2),
            (pictura_core::BitDepth::ThirtyTwo, 4),
        ] {
            let mut doc = Document::new(1000, 1000, pictura_core::ColorMode::Rgb, depth);
            doc.layers = pictura_core::LayerStack::from_layers(vec![
                crate::cxxqt_object::tests::pixel_layer("layer", 1000, 1000, (0, 0, 0)),
            ]);
            let planes = doc.layers[0].channels.iter().filter(|c| c.id >= -1).count() as u64;
            let inside = PsdRect {
                top: 300,
                left: 300,
                bottom: 310,
                right: 310,
            };
            let one_tile = (TILE * TILE) as u64;
            assert_eq!(fork_bytes(&doc, "0", inside), planes * one_tile * bytes);
        }
    }

    #[test]
    fn a_fill_that_grows_the_layer_forks_the_grown_tiles() {
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
        // grows it to 120×140, whose tiles the fill forks.
        let bounds = PsdRect {
            top: 80,
            left: 80,
            bottom: 140,
            right: 120,
        };
        let planes = doc.layers[0].channels.iter().filter(|c| c.id >= -1).count() as u64;
        // The grown layer is one 120×140 tile.
        assert_eq!(fork_bytes(&doc, "0", bounds), planes * 140 * 120);
    }
}
