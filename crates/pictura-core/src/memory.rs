//! Memory accounting (#1211, `docs/10-workflow-io/scratch-disks-and-memory.md`,
//! Memory accounting model): the host reader, the process-wide governor, and
//! the tile census history's budget is checked against (#1215).

mod census;
mod governor;
pub mod system;

pub use census::{Side, TileCensus};
pub use governor::{
    ceiling_of, clear_refusal, fail_next_plane_for_test, governor, last_refusal, less_headroom,
    pressure_of, room_of, set_host_for_test, try_filled_plane, try_plane, used_of, Governor,
    MemoryError, MemoryStatus, Pressure, Reservation, CANVAS_SCALE_BYTES, DEFAULT_USAGE_PERCENT,
    HISTORY_SHARE,
};
pub use system::{available_memory, total_memory, HostPaths, HostReading};

#[cfg(test)]
mod tests;
