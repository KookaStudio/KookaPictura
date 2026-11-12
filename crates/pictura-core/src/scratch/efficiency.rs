//! Efficiency (Parity criterion 9): monotonic compute and scratch-I/O time.
//!
//! I/O is time a thread spends blocked on scratch reads, writes and
//! synchronous reclaim; the background evictor blocks nobody and is not
//! counted. Compute is the wall time of [`operation`]s less the I/O that
//! happened while they ran.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static IO_NS: AtomicU64 = AtomicU64::new(0);
static COMPUTE_NS: AtomicU64 = AtomicU64::new(0);

/// Cumulative compute and scratch-I/O nanoseconds since the process started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Efficiency {
    pub compute_ns: u64,
    pub io_ns: u64,
}

impl Efficiency {
    /// `compute / (compute + io) × 100` over the interval since `earlier`;
    /// 100 when nothing was measured.
    pub fn percent_since(&self, earlier: &Efficiency) -> f64 {
        let compute = self.compute_ns.saturating_sub(earlier.compute_ns) as f64;
        let io = self.io_ns.saturating_sub(earlier.io_ns) as f64;
        if compute + io == 0.0 {
            100.0
        } else {
            compute / (compute + io) * 100.0
        }
    }

    /// [`Self::percent_since`] the process start.
    pub fn percent(&self) -> f64 {
        self.percent_since(&Efficiency::default())
    }
}

/// The counters now.
pub fn efficiency() -> Efficiency {
    Efficiency {
        compute_ns: COMPUTE_NS.load(Ordering::Relaxed),
        io_ns: IO_NS.load(Ordering::Relaxed),
    }
}

/// Times scratch I/O until dropped.
#[must_use = "the I/O is timed until the timer drops"]
pub struct IoTimer(Instant);

pub fn io_timer() -> IoTimer {
    IoTimer(Instant::now())
}

impl Drop for IoTimer {
    fn drop(&mut self) {
        IO_NS.fetch_add(self.0.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
}

/// Times an operation until dropped; its compute is its wall time less the
/// scratch I/O recorded meanwhile.
#[must_use = "the operation is timed until the guard drops"]
pub struct Operation {
    start: Instant,
    io_at_start: u64,
}

pub fn operation() -> Operation {
    Operation {
        start: Instant::now(),
        io_at_start: IO_NS.load(Ordering::Relaxed),
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        let wall = self.start.elapsed().as_nanos() as u64;
        let io = IO_NS
            .load(Ordering::Relaxed)
            .saturating_sub(self.io_at_start);
        COMPUTE_NS.fetch_add(wall.saturating_sub(io), Ordering::Relaxed);
    }
}
