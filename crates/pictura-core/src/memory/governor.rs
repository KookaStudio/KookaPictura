//! The process-wide memory governor (#1211): a ceiling from Memory Usage % and
//! the cgroup, the room left under it, a pressure level, and RAII
//! reservations that fail with a typed error instead of letting the kernel's
//! OOM killer end the process.

use super::system::{self, HostPaths, HostReading};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MIB: u64 = 1024 * 1024;
/// The least headroom kept below a limit.
const MIN_HEADROOM: u64 = 256 * MIB;
/// How long a reading of the host is reused.
const REFRESH: Duration = Duration::from_millis(250);
/// Buffers from this size up are charged to the governor by [`try_plane`].
pub const CANVAS_SCALE_BYTES: usize = 16 * 1024 * 1024;
/// Memory Usage's default, in percent (`scratch-disks-and-memory.md`; owner
/// decision over the 70 % of `preferences.md`).
pub const DEFAULT_USAGE_PERCENT: u8 = 60;

/// Why memory was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    Insufficient { purpose: &'static str, bytes: u64 },
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insufficient { purpose, bytes } => {
                write!(f, "not enough memory for {purpose} ({bytes} bytes)")
            }
        }
    }
}

impl std::error::Error for MemoryError {}

/// How hard the host is pressed for memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressure {
    Normal,
    Elevated,
    Critical,
}

impl Pressure {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Elevated => "elevated",
            Self::Critical => "critical",
        }
    }
}

/// What the governor sees now, for reporting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryStatus {
    pub ceiling: Option<u64>,
    pub used: Option<u64>,
    pub reserved: u64,
    pub room: Option<u64>,
    pub pressure: Pressure,
    pub usage_percent: u8,
}

/// `limit` less the larger of 5 % of it or 256 MiB.
pub fn less_headroom(limit: u64) -> u64 {
    limit.saturating_sub((limit / 20).max(MIN_HEADROOM))
}

/// The ceiling of `reading` at `percent` Memory Usage: the smallest present
/// limit less headroom, or `None` without one.
pub fn ceiling_of(reading: &HostReading, percent: u8) -> Option<u64> {
    let usage = reading
        .total
        .map(|total| (total as u128 * u128::from(percent) / 100) as u64);
    [usage, reading.cgroup_max, reading.cgroup_high]
        .into_iter()
        .flatten()
        .min()
        .map(less_headroom)
}

/// The memory counted against the ceiling: the cgroup's when a cgroup limit is
/// present (what its OOM decision counts), else the process's resident set.
pub fn used_of(reading: &HostReading) -> Option<u64> {
    if reading.cgroup_max.is_some() || reading.cgroup_high.is_some() {
        reading.cgroup_current.or(reading.rss)
    } else {
        reading.rss
    }
}

/// The bytes a new reservation may take, or `None` for no limit.
pub fn room_of(reading: &HostReading, percent: u8, reserved: u64) -> Option<u64> {
    let under_ceiling = ceiling_of(reading, percent).map(|ceiling| {
        ceiling
            .saturating_sub(used_of(reading).unwrap_or(0))
            .saturating_sub(reserved)
    });
    let under_available = reading
        .available
        .map(|available| less_headroom(available).saturating_sub(reserved));
    [under_ceiling, under_available].into_iter().flatten().min()
}

/// The pressure level of `reading`: PSI where present, else the free share.
pub fn pressure_of(reading: &HostReading) -> Pressure {
    if let Some(avg10) = reading.psi_some_avg10 {
        return match avg10 {
            a if a >= 20.0 => Pressure::Critical,
            a if a >= 5.0 => Pressure::Elevated,
            _ => Pressure::Normal,
        };
    }
    match (reading.available, reading.total) {
        (Some(available), Some(total)) if total > 0 => match available * 100 / total {
            share if share < 10 => Pressure::Critical,
            share if share < 20 => Pressure::Elevated,
            _ => Pressure::Normal,
        },
        _ => Pressure::Normal,
    }
}

std::thread_local! {
    /// The reading a test pins for this thread.
    static PINNED: Cell<Option<HostReading>> = const { Cell::new(None) };
    /// The last refusal on this thread.
    static REFUSAL: Cell<Option<MemoryError>> = const { Cell::new(None) };
    /// Fail the next [`try_plane`] on this thread.
    static FAIL_NEXT_PLANE: Cell<bool> = const { Cell::new(false) };
}

/// The process-wide governor.
pub struct Governor {
    paths: HostPaths,
    usage_percent: AtomicU8,
    reserved: AtomicU64,
    cache: Mutex<Option<(Instant, HostReading)>>,
}

/// The process's governor, reading the real host.
pub fn governor() -> &'static Governor {
    static GOVERNOR: OnceLock<Governor> = OnceLock::new();
    GOVERNOR.get_or_init(|| Governor::new(HostPaths::default()))
}

impl Governor {
    pub fn new(paths: HostPaths) -> Self {
        Self {
            paths,
            usage_percent: AtomicU8::new(DEFAULT_USAGE_PERCENT),
            reserved: AtomicU64::new(0),
            cache: Mutex::new(None),
        }
    }

    /// The host now: the pinned reading of this thread, else the cached one,
    /// re-read when older than 250 ms.
    pub fn reading(&self) -> HostReading {
        if let Some(pinned) = PINNED.get() {
            return pinned;
        }
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match *cache {
            Some((at, reading)) if at.elapsed() < REFRESH => reading,
            _ => {
                let reading = system::read(&self.paths);
                *cache = Some((Instant::now(), reading));
                reading
            }
        }
    }

    pub fn usage_percent(&self) -> u8 {
        self.usage_percent.load(Ordering::Relaxed)
    }

    /// Set Memory Usage, clamped to 5–100 %.
    pub fn set_usage_percent(&self, percent: u8) {
        self.usage_percent
            .store(percent.clamp(5, 100), Ordering::Relaxed);
    }

    pub fn reserved(&self) -> u64 {
        self.reserved.load(Ordering::Relaxed)
    }

    pub fn ceiling(&self) -> Option<u64> {
        ceiling_of(&self.reading(), self.usage_percent())
    }

    pub fn room(&self) -> Option<u64> {
        room_of(&self.reading(), self.usage_percent(), self.reserved())
    }

    pub fn status(&self) -> MemoryStatus {
        let reading = self.reading();
        let percent = self.usage_percent();
        let reserved = self.reserved();
        MemoryStatus {
            ceiling: ceiling_of(&reading, percent),
            used: used_of(&reading),
            reserved,
            room: room_of(&reading, percent, reserved),
            pressure: pressure_of(&reading),
            usage_percent: percent,
        }
    }

    /// Hold `bytes` for `purpose` until the reservation drops, or refuse them
    /// (recording the refusal for this thread) when they exceed the room.
    pub fn reserve(
        &self,
        bytes: u64,
        purpose: &'static str,
    ) -> Result<Reservation<'_>, MemoryError> {
        if self.room().is_some_and(|room| bytes > room) {
            return Err(refuse(purpose, bytes));
        }
        self.reserved.fetch_add(bytes, Ordering::Relaxed);
        Ok(Reservation {
            governor: self,
            bytes,
        })
    }

    /// Whether `bytes` fit the room now; a refusal is recorded. For work that
    /// allocates right after the check.
    pub fn admit(&self, bytes: u64, purpose: &'static str) -> bool {
        self.reserve(bytes, purpose).is_ok()
    }
}

/// Bytes held against the governor's room until dropped.
#[must_use = "a reservation is released when dropped"]
pub struct Reservation<'a> {
    governor: &'a Governor,
    bytes: u64,
}

impl Reservation<'_> {
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        self.governor
            .reserved
            .fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

fn refuse(purpose: &'static str, bytes: u64) -> MemoryError {
    let error = MemoryError::Insufficient { purpose, bytes };
    REFUSAL.set(Some(error));
    error
}

/// The last refusal recorded on this thread.
pub fn last_refusal() -> Option<MemoryError> {
    REFUSAL.get()
}

/// Forget this thread's last refusal; a gated edit starts here.
pub fn clear_refusal() {
    REFUSAL.set(None);
}

/// Pin the reading this thread's governor calls see; `None` reads the host
/// again.
pub fn set_host_for_test(reading: Option<HostReading>) {
    PINNED.set(reading);
}

/// Make the next [`try_plane`] on this thread fail as an allocator failure.
pub fn fail_next_plane_for_test() {
    FAIL_NEXT_PLANE.set(true);
}

/// A `len`-byte buffer of `value` for `purpose`. A canvas-scale buffer
/// (16 MiB or more) is first admitted by the governor; the allocation is
/// fallible, so an allocator failure is [`MemoryError::Insufficient`] (recorded
/// like any refusal), never an abort.
pub fn try_plane(len: usize, value: u8, purpose: &'static str) -> Result<Vec<u8>, MemoryError> {
    let bytes = len as u64;
    if FAIL_NEXT_PLANE.replace(false) {
        return Err(refuse(purpose, bytes));
    }
    if len >= CANVAS_SCALE_BYTES && !governor().admit(bytes, purpose) {
        return Err(MemoryError::Insufficient { purpose, bytes });
    }
    let mut plane = Vec::new();
    plane
        .try_reserve_exact(len)
        .map_err(|_| refuse(purpose, bytes))?;
    plane.resize(len, value);
    Ok(plane)
}
