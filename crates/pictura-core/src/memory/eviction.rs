//! Tile eviction under the memory ceiling (#1217): a clock (second-chance)
//! sweep over every dense tile body that packs (LZ4 in RAM) and spills (the
//! scratch disk) in priority order, a background thread that runs it above
//! the ceiling, hot holds that keep the tiles in use resident, and the
//! counters the readouts show.
//!
//! Order (R2): the display tile cache is asked to drop, then tiles only
//! history holds, then cold live tiles; packing before spilling within each.
//! `Backed` tiles (#1218) would come second; they do not exist yet.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, Weak};
use std::time::{Duration, Instant};

use super::governor::{ceiling_of, governor, used_of};
use crate::scratch::ScratchError;

/// Where a tile's samples are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Residency {
    Resident,
    /// Compressed in RAM.
    Packed,
    /// In a scratch extent.
    Spilled,
    /// Lost from scratch: reads as the default.
    Unreadable,
}

/// What evicting one body did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Packing {
    Skipped,
    Freed { bytes: u64, written: u64 },
    Failed(ScratchError),
}

/// A dense tile body as the evictor sees it, for every sample type.
pub(crate) trait Evictable: Send + Sync {
    fn pinned(&self) -> bool;
    /// Clear the clock bit, returning whether it was set.
    fn take_referenced(&self) -> bool;
    /// Touched under a hot hold at or after epoch `oldest`.
    fn hot_since(&self, oldest: u64) -> bool;
    fn history_held(&self) -> bool;
    fn residency(&self) -> Residency;
    /// Pack a resident body (or drop it to its clean extent); an
    /// incompressible one is spilled raw when `may_spill`.
    fn pack(&self, may_spill: bool) -> Packing;
    /// Spill a packed body to scratch.
    fn spill(&self) -> Packing;
}

const SHARDS: usize = 16;

struct Shard {
    bodies: Vec<Weak<dyn Evictable>>,
    /// Live entries at the last prune: the shard prunes when it doubles.
    live: usize,
}

static REGISTRY: [Mutex<Shard>; SHARDS] = [const {
    Mutex::new(Shard {
        bodies: Vec::new(),
        live: 0,
    })
}; SHARDS];

fn shard_of_thread() -> usize {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::thread_local! {
        static SHARD: usize = (NEXT.fetch_add(1, Ordering::Relaxed) as usize) % SHARDS;
    }
    SHARD.with(|s| *s)
}

fn lock(shard: &Mutex<Shard>) -> std::sync::MutexGuard<'_, Shard> {
    shard.lock().unwrap_or_else(|e| e.into_inner())
}

/// Track a new body. Dead entries are pruned when a shard has doubled since
/// its last prune, so the registry stays proportional to the live tiles: a
/// dead entry still holds its body's header (not its samples) until then.
pub(crate) fn register(body: Weak<dyn Evictable>) {
    let mut shard = lock(&REGISTRY[shard_of_thread()]);
    shard.bodies.push(body);
    if shard.bodies.len() > 2 * shard.live.max(32) {
        shard.bodies.retain(|w| w.strong_count() > 0);
        shard.live = shard.bodies.len();
    }
}

/// Every live body, pruning the dead.
fn bodies() -> Vec<Weak<dyn Evictable>> {
    let mut all = Vec::new();
    for shard in &REGISTRY {
        let mut shard = lock(shard);
        shard.bodies.retain(|w| w.strong_count() > 0);
        shard.live = shard.bodies.len();
        all.extend(shard.bodies.iter().cloned());
    }
    all
}

// ---- Counters ----

static PACKED: AtomicI64 = AtomicI64::new(0);
static PACKED_BYTES: AtomicI64 = AtomicI64::new(0);
static SPILLED: AtomicI64 = AtomicI64::new(0);
static UNREADABLE: AtomicI64 = AtomicI64::new(0);
static UNREADABLE_READS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn count_packed(n: i64, bytes: i64) {
    PACKED.fetch_add(n, Ordering::Relaxed);
    PACKED_BYTES.fetch_add(bytes, Ordering::Relaxed);
}

pub(crate) fn count_spilled(n: i64) {
    SPILLED.fetch_add(n, Ordering::Relaxed);
}

pub(crate) fn count_unreadable() {
    UNREADABLE.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn count_unreadable_dropped() {
    UNREADABLE.fetch_sub(1, Ordering::Relaxed);
}

pub(crate) fn count_unreadable_read() {
    UNREADABLE_READS.fetch_add(1, Ordering::Relaxed);
}

/// Live unreadable bodies, process-wide.
pub fn unreadable_count() -> u64 {
    UNREADABLE.load(Ordering::Relaxed).max(0) as u64
}

/// Reads of unreadable tiles since the process started: an edit that sees
/// this move read data it does not have (design §5).
pub fn unreadable_reads() -> u64 {
    UNREADABLE_READS.load(Ordering::Relaxed)
}

/// The packed, spilled and unreadable tiles now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileCounts {
    pub packed: u64,
    pub packed_bytes: u64,
    pub spilled: u64,
    pub unreadable: u64,
}

pub fn counts() -> TileCounts {
    let get = |a: &AtomicI64| a.load(Ordering::Relaxed).max(0) as u64;
    TileCounts {
        packed: get(&PACKED),
        packed_bytes: get(&PACKED_BYTES),
        spilled: get(&SPILLED),
        unreadable: get(&UNREADABLE),
    }
}

pub(crate) fn residency_of(body: &dyn Evictable) -> Residency {
    body.residency()
}

// ---- Hot holds ----

static HOT_EPOCH: AtomicU64 = AtomicU64::new(0);
/// The oldest live hold's epoch, `u64::MAX` without one.
static OLDEST_HOLD: AtomicU64 = AtomicU64::new(u64::MAX);
static HOLDS: Mutex<BTreeMap<u64, u32>> = Mutex::new(BTreeMap::new());

/// Keeps the tiles touched from now on, until it and every later hold drop,
/// out of eviction: the stroke or edit in progress, the visible view.
#[must_use = "tiles stay hot only while the hold lives"]
pub struct HotHold {
    epoch: u64,
}

/// Start a hot hold.
pub fn hot() -> HotHold {
    let mut holds = HOLDS.lock().unwrap_or_else(|e| e.into_inner());
    let epoch = HOT_EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    *holds.entry(epoch).or_default() += 1;
    let oldest = *holds.keys().next().expect("just inserted");
    OLDEST_HOLD.store(oldest, Ordering::SeqCst);
    HotHold { epoch }
}

impl Drop for HotHold {
    fn drop(&mut self) {
        let mut holds = HOLDS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = holds.get_mut(&self.epoch) {
            *n -= 1;
            if *n == 0 {
                holds.remove(&self.epoch);
            }
        }
        let oldest = holds.keys().next().copied().unwrap_or(u64::MAX);
        OLDEST_HOLD.store(oldest, Ordering::SeqCst);
    }
}

/// The epoch a body touched now records, or `None` with no hold alive.
pub(crate) fn hot_epoch() -> Option<u64> {
    (OLDEST_HOLD.load(Ordering::Relaxed) != u64::MAX).then(|| HOT_EPOCH.load(Ordering::Relaxed))
}

// ---- Display drop requests ----

static DISPLAY_DROP: AtomicU64 = AtomicU64::new(0);

/// Advanced by every eviction pass: a display cache that sees it move drops
/// its unpinned tiles, the first thing R2 frees.
pub fn display_drop_epoch() -> u64 {
    DISPLAY_DROP.load(Ordering::SeqCst)
}

// ---- Passes ----

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassReport {
    /// Bytes no longer held in RAM.
    pub freed: u64,
    pub packed: u64,
    pub spilled: u64,
    /// Bytes written to scratch.
    pub written: u64,
    /// The scratch disks were full.
    pub scratch_full: bool,
}

impl PassReport {
    fn add(&mut self, packing: Packing, spilled: bool) {
        match packing {
            Packing::Freed { bytes, written } => {
                self.freed += bytes;
                self.written += written;
                if spilled || written > 0 {
                    self.spilled += 1;
                } else {
                    self.packed += 1;
                }
            }
            Packing::Failed(ScratchError::Full) => self.scratch_full = true,
            Packing::Failed(_) | Packing::Skipped => {}
        }
    }
}

struct SpillPause {
    until: Option<Instant>,
    backoff: Duration,
}

static SPILL_PAUSE: Mutex<SpillPause> = Mutex::new(SpillPause {
    until: None,
    backoff: Duration::from_millis(100),
});

fn may_spill() -> bool {
    let pause = SPILL_PAUSE.lock().unwrap_or_else(|e| e.into_inner());
    pause.until.is_none_or(|t| Instant::now() >= t)
}

/// Record the outcome of spilling: a full scratch pauses it with backoff
/// (100 ms doubling to 5 s, Parity criterion 5); a success resets it.
fn spill_outcome(full: bool, wrote: bool) {
    let mut pause = SPILL_PAUSE.lock().unwrap_or_else(|e| e.into_inner());
    if full {
        pause.until = Some(Instant::now() + pause.backoff);
        pause.backoff = (pause.backoff * 2).min(Duration::from_secs(5));
    } else if wrote {
        pause.until = None;
        pause.backoff = Duration::from_millis(100);
    }
}

/// One clock sweep that frees up to `need` bytes: the display drop request,
/// then history-held bodies, then the rest; pack, then spill, in each.
pub fn run_pass(need: u64) -> PassReport {
    DISPLAY_DROP.fetch_add(1, Ordering::SeqCst);
    let mut report = PassReport::default();
    if need == 0 {
        return report;
    }
    let oldest = OLDEST_HOLD.load(Ordering::SeqCst);
    let all = bodies();
    for history in [true, false] {
        let spill = may_spill();
        for weak in &all {
            if report.freed >= need {
                break;
            }
            let Some(body) = weak.upgrade() else {
                continue;
            };
            if body.history_held() != history
                || body.pinned()
                || body.hot_since(oldest)
                || body.take_referenced()
            {
                continue;
            }
            report.add(body.pack(spill && !report.scratch_full), false);
        }
        if spill && !report.scratch_full {
            for weak in &all {
                if report.freed >= need || report.scratch_full {
                    break;
                }
                let Some(body) = weak.upgrade() else {
                    continue;
                };
                if body.history_held() == history && body.residency() == Residency::Packed {
                    report.add(body.spill(), true);
                }
            }
        }
        if report.freed >= need {
            break;
        }
    }
    spill_outcome(report.scratch_full, report.written > 0);
    report
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static URGENT: AtomicBool = AtomicBool::new(false);

std::thread_local! {
    static RECLAIMING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether eviction runs: reclaim and the urgent path act only then.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Turn eviction on or off without the thread (tests).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on {
        URGENT.store(false, Ordering::Relaxed);
    }
}

/// Turn eviction on and start its thread, once per process.
pub fn start() {
    static THREAD: std::sync::Once = std::sync::Once::new();
    set_enabled(true);
    THREAD.call_once(|| {
        let _ = std::thread::Builder::new()
            .name("pictura-evict".into())
            .spawn(|| loop {
                if is_enabled() {
                    step();
                }
                std::thread::sleep(Duration::from_millis(100));
            });
    });
}

/// Read the host and evict when over the ceiling: from above 100 % down to
/// 90 %; above 125 % allocating threads evict too.
pub fn step() -> PassReport {
    let g = governor();
    let reading = g.reading_fresh();
    let Some(ceiling) = ceiling_of(&reading, g.usage_percent()) else {
        URGENT.store(false, Ordering::Relaxed);
        return PassReport::default();
    };
    let used = used_of(&reading).unwrap_or(0);
    URGENT.store(used > ceiling + ceiling / 4, Ordering::Relaxed);
    if used <= ceiling {
        return PassReport::default();
    }
    let report = run_pass(used - ceiling / 10 * 9);
    trim();
    report
}

/// Evict up to `bytes` on this thread, timed as scratch I/O; returns the
/// bytes freed. Nothing while eviction is off or this thread is already
/// reclaiming.
pub fn reclaim(bytes: u64) -> u64 {
    if !is_enabled() || bytes == 0 || RECLAIMING.get() {
        return 0;
    }
    RECLAIMING.set(true);
    let _io = crate::scratch::io_timer();
    let mut freed = 0;
    for _ in 0..3 {
        let report = run_pass(bytes - freed);
        freed += report.freed;
        if report.freed == 0 || freed >= bytes {
            break;
        }
    }
    trim();
    RECLAIMING.set(false);
    if freed >= bytes {
        URGENT.store(false, Ordering::Relaxed);
    }
    freed
}

/// A new dense body of `bytes` is about to be allocated: above 125 % of the
/// ceiling the allocating thread evicts first.
pub(crate) fn on_allocate(bytes: u64) {
    if URGENT.load(Ordering::Relaxed) {
        reclaim(bytes.max(4 * 1024 * 1024));
    }
}

/// Re-evaluate the urgent flag from a (possibly pinned) reading (tests).
pub fn refresh_urgency() {
    let g = governor();
    let reading = g.reading_fresh();
    let urgent = ceiling_of(&reading, g.usage_percent())
        .is_some_and(|c| used_of(&reading).unwrap_or(0) > c + c / 4);
    URGENT.store(urgent && is_enabled(), Ordering::Relaxed);
}

/// Hand freed heap back to the kernel: glibc keeps freed 64 KiB tiles in its
/// arenas, so without this RSS would not fall.
fn trim() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: malloc_trim only releases free heap memory.
    unsafe {
        libc::malloc_trim(0);
    }
}

#[cfg(test)]
mod tests;
