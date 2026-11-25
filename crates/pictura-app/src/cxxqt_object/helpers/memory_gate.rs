//! The memory gate an edit passes before it allocates (#1207, #1211): it
//! estimates the bytes the edit will hold at its peak and asks the process's
//! memory governor (`pictura_core::memory`), which refuses them when they
//! exceed the room under its ceiling, instead of leaving the process to the
//! kernel's OOM killer.
//!
//! GUI thread only: the refusal record and test pins are thread-locals, as is
//! the governor's test host, so an edit gated on a worker thread would not
//! report its refusal to the bridge, which reads it on the GUI thread.

use pictura_core::memory::{self, HostReading};
use pictura_core::{BitDepth, Document, PsdRect, TILE};

/// Why a gated edit was refused. One value, three cases, so no two readers can
/// disagree about which refusal happened.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cxxqt_object) enum Refusal {
    /// The governor's room could not hold the edit.
    Memory,
    /// The edit needed tiles that could not be read back from the scratch disk.
    ScratchUnreadable,
    /// The spillable output did not fit the room plus the scratch disks' free
    /// space (#1290).
    ScratchFull,
}

/// What one admission decided. `spill` reports whether an admitted commit can
/// fit only by packing or spilling tiles (#1290), so a caller asks once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cxxqt_object) enum Admission {
    Admitted {
        /// The commit exceeds the room and must spill to fit room plus scratch.
        spill: bool,
    },
    Refused(Refusal),
}

impl Admission {
    pub(in crate::cxxqt_object) fn admitted(self) -> bool {
        matches!(self, Self::Admitted { .. })
    }
}

std::thread_local! {
    /// Why the last gated edit was refused, if it was. The one cell the
    /// `edit_refused_for_*` predicates read.
    static LAST_REFUSAL: std::cell::Cell<Option<Refusal>> =
        const { std::cell::Cell::new(None) };
    /// The scratch disks' free bytes as a test pins them; `None` reads them.
    /// Not refusal state: it is how the scratch room is read.
    static SCRATCH_AVAILABLE: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
    /// `unreadable_reads()` when the last edit committed. Not refusal state:
    /// it is the read counter the rollback diffs.
    static UNREADABLE_SEEN: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Forget the last refusal; every admission starts here.
fn reset() {
    LAST_REFUSAL.set(None);
    memory::clear_refusal();
}

/// The last refusal recorded on this thread, or `None`.
pub(in crate::cxxqt_object) fn last_refusal() -> Option<Refusal> {
    LAST_REFUSAL.get()
}

/// Record a refusal this gate decided beyond the governor's room.
fn record(reason: Refusal) {
    LAST_REFUSAL.set(Some(reason));
}

/// Forget the last refusal; every gated edit starts here.
pub(in crate::cxxqt_object) fn clear() {
    reset();
}

/// Record a refusal for unreadable scratch tiles.
pub(in crate::cxxqt_object) fn refuse_for_scratch() {
    record(Refusal::ScratchUnreadable);
}

/// Whether the last gated edit or save was refused for unreadable tiles.
pub(in crate::cxxqt_object) fn edit_refused_for_scratch() -> bool {
    last_refusal() == Some(Refusal::ScratchUnreadable)
}

/// Whether an edit that just committed read tiles lost from the scratch disk
/// and so must be rolled back (#1217); marks the reads seen.
pub(in crate::cxxqt_object) fn read_lost_tiles() -> bool {
    let now = memory::eviction::unreadable_reads();
    UNREADABLE_SEEN.replace(now) != now
}

/// Whether `bytes` fit the governor's room, resetting the last refusal first.
/// A path that runs no other gate calls this.
pub(in crate::cxxqt_object) fn admit_resident(bytes: u64) -> Admission {
    reset();
    if memory::governor().admit(bytes, "an edit") {
        return Admission::Admitted { spill: false };
    }
    record(Refusal::Memory);
    Admission::Refused(Refusal::Memory)
}

/// Whether a commit fits (#1290): its `working` bytes must fit the governor's
/// room, and its `spillable` bytes, which the evictor may pack or spill as they
/// are written, the room plus the scratch disks' free space. Resets the last
/// refusal first.
pub(in crate::cxxqt_object) fn admit_commit(cost: &CommitCost) -> Admission {
    reset();
    if !memory::governor().admit(cost.working, "an edit") {
        record(Refusal::Memory);
        return Admission::Refused(Refusal::Memory);
    }
    let Some(room) = memory::governor().room() else {
        return Admission::Admitted { spill: false };
    };
    if cost.spillable > room.saturating_add(scratch_available()) {
        refuse_scratch_full("an edit", cost.spillable);
        return Admission::Refused(Refusal::ScratchFull);
    }
    Admission::Admitted {
        spill: cost.working.saturating_add(cost.spillable) > room,
    }
}

/// [`admit_commit`] for a caller that holds the two byte counts apart. Kept for
/// the gate tests the retained suites drive.
#[cfg(test)]
pub(in crate::cxxqt_object) fn admit_spilling(working: u64, spillable: u64) -> bool {
    admit_commit(&CommitCost { working, spillable }).admitted()
}

/// Record that `bytes` of spillable output for `purpose` did not fit the room
/// plus the scratch disks' free space (#1290); it counts as a memory refusal.
pub(in crate::cxxqt_object) fn refuse_scratch_full(purpose: &'static str, bytes: u64) {
    record(Refusal::ScratchFull);
    memory::record_refusal(purpose, bytes);
}

/// Whether a commit can only fit by packing or spilling tiles: its working and
/// spillable bytes together exceed the room (#1290). Prefer
/// [`Admission::Admitted`]'s `spill`, which answers this from the admission.
/// Kept for the gate tests the retained suites drive.
#[cfg(test)]
pub(in crate::cxxqt_object) fn expects_spill(cost: &CommitCost) -> bool {
    memory::governor()
        .room()
        .is_some_and(|room| cost.working.saturating_add(cost.spillable) > room)
}

/// The free bytes on the usable scratch volumes.
pub(in crate::cxxqt_object) fn scratch_available() -> u64 {
    SCRATCH_AVAILABLE
        .get()
        .unwrap_or_else(|| pictura_core::scratch::sizes().available)
}

/// Whether the last gated edit was refused because the scratch disks could not
/// hold its spillable output.
pub(in crate::cxxqt_object) fn edit_refused_for_scratch_full() -> bool {
    last_refusal() == Some(Refusal::ScratchFull)
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
    let planes = layer.channels().iter().filter(|c| c.id >= -1).count() as u64;
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

/// The display: the tile cache's budget, which it never exceeds beyond the
/// presented view (#1216).
fn display() -> u64 {
    memory::governor().display_budget()
}

/// The stored composite a document keeps: only an authoritative one (#1216).
fn stored_composite(doc: &Document) -> u64 {
    if doc.composite_is_authoritative() {
        doc.composite.data.len() as u64
    } else {
        0
    }
}

/// New Document (M1): the base layer starts as uniform tiles and holds nothing
/// (#1213), and no composite is rendered or stored (#1216): the display tiles
/// are what remains. A document of another mode or depth keeps its composite.
pub(in crate::cxxqt_object) fn new_document_bytes(
    width: u32,
    height: u32,
    color_channels: u8,
    depth: BitDepth,
) -> u64 {
    let stored = if color_channels == 3 && depth == BitDepth::Eight {
        0
    } else {
        sample_bytes(depth) * u64::from(color_channels) + 4
    };
    u64::from(width) * u64::from(height) * stored + display()
}

/// Add Layer (M2): an empty-rect layer stores no pixel planes, only its record
/// and history state (#1208).
pub(in crate::cxxqt_object) fn add_layer_bytes(_doc: &Document) -> u64 {
    256 * 1024
}

/// A brush stroke's start (M6): the target layer's planes and the stroke's
/// canvas-sized buffers.
pub(in crate::cxxqt_object) fn brush_bytes(doc: &Document, path: &str) -> u64 {
    let layer = pictura_render::resolve_path(doc, path).map_or(0, |l| {
        l.channels().iter().map(|c| c.data.len() as u64).sum()
    });
    layer + 3 * canvas(doc)
}

/// Undo, Redo or a history jump (M7): a stored composite and the display
/// tiles the restore rebuilds.
pub(in crate::cxxqt_object) fn restore_bytes(doc: &Document) -> u64 {
    stored_composite(doc) + display()
}

/// The display refresh after a fill of `bounds` (M5): the region's composite
/// and display bytes, beside what the fill itself holds (10.9 bytes a pixel for
/// a whole-layer bucket at 4250², of which the fill's own planes are 6).
pub(in crate::cxxqt_object) fn refresh_bytes(bounds: PsdRect) -> u64 {
    6 * area(bounds)
}

/// What a commit holds (#1290): `working` stays resident; `spillable` is
/// written as tiles the evictor may pack or spill as they are written.
pub(in crate::cxxqt_object) struct CommitCost {
    pub(in crate::cxxqt_object) working: u64,
    pub(in crate::cxxqt_object) spillable: u64,
}

/// The most one worker thread's source window holds in the tiled geometry
/// commands: 1 Mpx of up to 32-bit samples in Image Size's resampler (#1293),
/// more than a turn's or a rotation's block window (#1294).
const WINDOW_BYTES: u64 = 4 << 20;

/// What a whole-document geometry command holds (#1293, #1294) given the
/// `(bytes, widest)` of the planes it writes and its new `canvas`: `working` is
/// one source window per worker thread, one output strip of the widest plane at
/// 32 bits, and the dense composite of the new canvas when the document's
/// composite is authoritative; `spillable` is every written plane, as tiles the
/// evictor may pack or spill.
pub(in crate::cxxqt_object) fn geometry_cost(
    doc: &Document,
    (planes, widest): (u64, u64),
    (width, height): (u32, u32),
) -> CommitCost {
    let threads = rayon::current_num_threads() as u64;
    let composite = if doc.composite_is_authoritative() {
        refresh_bytes(PsdRect {
            top: 0,
            left: 0,
            bottom: height as i32,
            right: width as i32,
        })
    } else {
        0
    };
    CommitCost {
        working: threads * WINDOW_BYTES + 4 * TILE as u64 * widest + composite,
        spillable: planes,
    }
}

/// What an Image Size to `width × height` holds (#1293): every resampled plane.
pub(in crate::cxxqt_object) fn image_size_cost(
    doc: &Document,
    width: u32,
    height: u32,
) -> CommitCost {
    let planes = pictura_render::resized_plane_bytes(doc, width, height);
    geometry_cost(doc, planes, (width, height))
}

/// What a Canvas Size to `width × height` at `anchor` holds (#1294): the
/// shifted alpha channels and document store, and the extended Background.
pub(in crate::cxxqt_object) fn canvas_size_cost(
    doc: &Document,
    width: u32,
    height: u32,
    anchor: pictura_ops::Anchor,
) -> CommitCost {
    let planes = pictura_render::canvas_plane_bytes(doc, width, height, anchor);
    geometry_cost(doc, planes, (width, height))
}

/// What a turn by `quarter_turns`, or a flip at 0, holds (#1294): every plane.
pub(in crate::cxxqt_object) fn orient_cost(doc: &Document, quarter_turns: u8) -> CommitCost {
    let planes = pictura_render::oriented_plane_bytes(doc, quarter_turns);
    let canvas = if quarter_turns % 2 == 1 {
        (doc.height, doc.width)
    } else {
        (doc.width, doc.height)
    };
    geometry_cost(doc, planes, canvas)
}

/// What an arbitrary Rotate by `angle_deg` about the canvas centre holds
/// (#1294): every rotated plane, and its new canvas.
pub(in crate::cxxqt_object) fn rotate_cost(
    doc: &Document,
    angle_deg: f64,
) -> (CommitCost, (u32, u32)) {
    let (bytes, widest, canvas) = pictura_render::rotated_plane_bytes(doc, angle_deg);
    (geometry_cost(doc, (bytes, widest), canvas), canvas)
}

/// The refresh of `bounds` that stays resident (#1300): [`refresh_bytes`]
/// where the refresh composites the region densely, as `refresh_display_rect`
/// does for an authoritative composite, a synchronous document, or a small
/// region; elsewhere it only marks the region's display tiles stale.
///
/// ponytail: a small region with no current tile under it is only marked
/// stale too, but it is still charged.
pub(in crate::cxxqt_object) fn refresh_working(doc: &Document, bounds: PsdRect) -> u64 {
    use crate::cxxqt_object::tiles::{REPAIR_PIXELS, SYNC_PIXELS};
    let dense = doc.composite_is_authoritative()
        || canvas(doc) <= SYNC_PIXELS
        || area(bounds) <= REPAIR_PIXELS;
    if dense {
        refresh_bytes(bounds)
    } else {
        0
    }
}

/// What a fill of `bounds` into the layer at `path` holds (#1300): its own
/// `resident` buffers (a flood's masks) and the resident refresh in RAM, and
/// the tiles it forks from history, written tile by tile into planes the
/// evictor may spill.
pub(in crate::cxxqt_object) fn fill_cost(
    doc: &Document,
    path: &str,
    bounds: PsdRect,
    resident: u64,
) -> CommitCost {
    CommitCost {
        working: resident + refresh_working(doc, bounds),
        spillable: fork_bytes(doc, path, bounds),
    }
}

/// Adding a named snapshot (M8): it shares every tile with the document, so
/// it costs the tile tables (#1215). Mid-stroke a stored composite is
/// rendered from the painted layers too.
pub(in crate::cxxqt_object) fn snapshot_bytes(doc: &Document, stroking: bool) -> u64 {
    let composite = if stroking && doc.composite_is_authoritative() {
        4 * canvas(doc)
    } else {
        0
    };
    crate::history::tile_table_bytes(doc) + composite
}

/// Restoring a named snapshot (M8): its tile tables, shared again, and the
/// display tiles the restore rebuilds.
pub(in crate::cxxqt_object) fn snapshot_restore_bytes(doc: &Document) -> u64 {
    crate::history::tile_table_bytes(doc) + display()
}

/// Whether the last gated edit was refused because it did not fit the
/// governor's room. A scratch-full refusal counts as one too, as before
/// (#1290); an unreadable-tile refusal does not.
///
/// The governor's own refusal is consulted only when the seam has no typed
/// reason, for an allocation that fails past the gate (`try_plane`'s failpoint,
/// `fallible-allocation`).
pub(in crate::cxxqt_object) fn edit_refused_for_memory() -> bool {
    match last_refusal() {
        Some(Refusal::Memory | Refusal::ScratchFull) => true,
        Some(Refusal::ScratchUnreadable) => false,
        None => memory::last_refusal().is_some(),
    }
}

/// The test pins: how a suite fixes the host and the scratch disks. Not part of
/// the seam's contract, so they sit together and away from the admission verbs.
pub(in crate::cxxqt_object) mod test_pins {
    use super::{memory, HostReading, SCRATCH_AVAILABLE};

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

    /// Pin this thread's scratch disks to `bytes` free; a negative value reads
    /// them again.
    pub(in crate::cxxqt_object) fn set_scratch_available_for_test(bytes: i64) {
        SCRATCH_AVAILABLE.set(u64::try_from(bytes).ok());
    }
}

pub(in crate::cxxqt_object) use test_pins::{
    set_available_memory_for_test, set_cgroup_for_test, set_scratch_available_for_test,
};

#[cfg(test)]
#[path = "memory_gate_tests.rs"]
mod tests;
