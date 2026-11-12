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
/// Memory Usage's default, in percent (`scratch-disks-and-memory.md` and
/// `preferences.md`; owner decision over a community guide's 70 %).
pub const DEFAULT_USAGE_PERCENT: u8 = 60;

/// History may hold one part in this many of the ceiling (#1207, #1215).
pub const HISTORY_SHARE: u64 = 2;

/// The display tile cache's cap (#1216): recomputable memory, dropped first.
pub const DISPLAY_CAP: u64 = 192 * MIB;
/// The display tile cache may hold one part in this many of the ceiling.
pub const DISPLAY_SHARE: u64 = 4;

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
/// present (what its OOM decision counts, less the page cache it reclaims
/// first), else the process's resident set.
pub fn used_of(reading: &HostReading) -> Option<u64> {
    if reading.cgroup_max.is_some() || reading.cgroup_high.is_some() {
        reading
            .cgroup_current
            .map(|current| current.saturating_sub(reading.cgroup_file.unwrap_or(0)))
            .or(reading.rss)
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

    /// The host now, without the cache (the evictor's view, #1217); a pinned
    /// reading still wins.
    pub fn reading_fresh(&self) -> HostReading {
        if let Some(pinned) = PINNED.get() {
            return pinned;
        }
        let reading = system::read(&self.paths);
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), reading));
        reading
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
        self.reserved.load(Ordering::Acquire)
    }

    pub fn ceiling(&self) -> Option<u64> {
        ceiling_of(&self.reading(), self.usage_percent())
    }

    /// The bytes history may hold beyond the live document: its share of the
    /// ceiling. `None` when the host reports no ceiling.
    pub fn history_budget(&self) -> Option<u64> {
        self.ceiling().map(|ceiling| ceiling / HISTORY_SHARE)
    }

    /// The bytes the display tile cache may hold: the smaller of
    /// [`DISPLAY_CAP`] and its share of the ceiling, or the cap when the host
    /// reports no ceiling.
    pub fn display_budget(&self) -> u64 {
        self.ceiling().map_or(DISPLAY_CAP, |ceiling| {
            (ceiling / DISPLAY_SHARE).min(DISPLAY_CAP)
        })
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
        let reading = self.reading();
        if self.try_reserve(&reading, bytes, 0) {
            return Ok(self.reservation(bytes));
        }
        // Over the room: evict reclaimable tiles first, and refuse only what
        // eviction cannot make room for (#1217). Freed bytes count as room
        // until the next reading shows them gone.
        let short = room_of(&reading, self.usage_percent(), self.reserved())
            .map_or(bytes, |room| bytes.saturating_sub(room));
        let freed = super::eviction::reclaim(short);
        if freed > 0 && self.try_reserve(&self.reading_fresh(), bytes, freed) {
            return Ok(self.reservation(bytes));
        }
        Err(refuse(purpose, bytes))
    }

    /// Add `bytes` to the reservations if they fit `reading`'s room plus
    /// `extra`; checked and added in one step, so two threads (a Save worker
    /// and a GUI gate) cannot both pass against the same room.
    fn try_reserve(&self, reading: &HostReading, bytes: u64, extra: u64) -> bool {
        let percent = self.usage_percent();
        self.reserved
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reserved| {
                let room = room_of(reading, percent, reserved);
                room.is_none_or(|room| bytes <= room.saturating_add(extra))
                    .then(|| reserved.saturating_add(bytes))
            })
            .is_ok()
    }

    fn reservation(&self, bytes: u64) -> Reservation<'_> {
        Reservation {
            governor: self,
            bytes,
        }
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
            .fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

fn refuse(purpose: &'static str, bytes: u64) -> MemoryError {
    let error = MemoryError::Insufficient { purpose, bytes };
    REFUSAL.set(Some(error));
    error
}

/// Record a refusal of `bytes` for `purpose` on this thread, for a gate that
/// decides beyond the governor's room (the scratch disks, #1290).
pub fn record_refusal(purpose: &'static str, bytes: u64) -> MemoryError {
    refuse(purpose, bytes)
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

/// Make the next [`try_plane`] or [`try_filled_plane`] on this thread fail as
/// an allocator failure.
pub fn fail_next_plane_for_test() {
    FAIL_NEXT_PLANE.set(true);
}

/// A tiled `width × height` plane of `value` for `purpose`. Its tiles are
/// uniform and hold no allocation (#1213), so only the test failpoint refuses
/// it; the refusal is recorded like any other.
pub fn try_filled_plane(
    width: usize,
    height: usize,
    value: u8,
    purpose: &'static str,
) -> Result<crate::Plane<u8>, MemoryError> {
    if FAIL_NEXT_PLANE.replace(false) {
        return Err(refuse(purpose, (width * height) as u64));
    }
    Ok(crate::Plane::filled(width, height, 1, value))
}

/// A `len`-byte buffer of `value` for `purpose`. A canvas-scale buffer
/// (16 MiB or more) is reserved with the governor while it is built; the allocation is
/// fallible, so an allocator failure is [`MemoryError::Insufficient`] (recorded
/// like any refusal), never an abort.
pub fn try_plane(len: usize, value: u8, purpose: &'static str) -> Result<Vec<u8>, MemoryError> {
    try_buffer(len, value, purpose)
}

/// [`try_plane`] for samples of any width: `len` samples of `value`.
pub fn try_buffer<T: Clone>(
    len: usize,
    value: T,
    purpose: &'static str,
) -> Result<Vec<T>, MemoryError> {
    let bytes = (len * std::mem::size_of::<T>()) as u64;
    if FAIL_NEXT_PLANE.replace(false) {
        return Err(refuse(purpose, bytes));
    }
    // Held across the allocation and its fill, so a concurrent reservation
    // sees these bytes until the reading counts them as resident.
    let _held = if bytes as usize >= CANVAS_SCALE_BYTES {
        Some(governor().reserve(bytes, purpose)?)
    } else {
        None
    };
    let mut plane = Vec::new();
    plane
        .try_reserve_exact(len)
        .map_err(|_| refuse(purpose, bytes))?;
    plane.resize(len, value);
    Ok(plane)
}
