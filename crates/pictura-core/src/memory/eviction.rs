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
use super::HostReading;
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
    /// The epoch it was last touched at under an open hot hold, 0 for never.
    fn hot_stamp(&self) -> u64;
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
/// The oldest open hold's start, `u64::MAX` without one.
static OLDEST_OPEN: AtomicU64 = AtomicU64::new(u64::MAX);

struct Holds {
    /// Open holds by start epoch.
    open: BTreeMap<u64, u32>,
    /// Closed holds by `[start, end)` epoch interval.
    closed: BTreeMap<(u64, u64), u32>,
}

static HOLDS: Mutex<Holds> = Mutex::new(Holds {
    open: BTreeMap::new(),
    closed: BTreeMap::new(),
});

fn holds() -> std::sync::MutexGuard<'static, Holds> {
    HOLDS.lock().unwrap_or_else(|e| e.into_inner())
}

fn release<K: Ord>(map: &mut BTreeMap<K, u32>, key: &K) {
    if let Some(n) = map.get_mut(key) {
        *n -= 1;
        if *n == 0 {
            map.remove(key);
        }
    }
}

fn publish_oldest(holds: &Holds) {
    let oldest = holds.open.keys().next().copied().unwrap_or(u64::MAX);
    OLDEST_OPEN.store(oldest, Ordering::SeqCst);
}

/// Keeps tiles out of eviction until the pass has nothing colder left: the
/// stroke or edit in progress, the tiles the view built. While open it
/// catches every tile touched; once closed it keeps only those (#1290).
#[must_use = "tiles stay hot only while the hold lives"]
pub struct HotHold {
    start: u64,
    end: Option<u64>,
}

/// Start an open hot hold.
pub fn hot() -> HotHold {
    let mut holds = holds();
    let start = HOT_EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    *holds.open.entry(start).or_default() += 1;
    publish_oldest(&holds);
    HotHold { start, end: None }
}

impl HotHold {
    /// Stop catching tiles: the ones touched so far stay hot until the hold
    /// drops, later ones are not this hold's.
    pub fn close(&mut self) {
        if self.end.is_some() {
            return;
        }
        let mut holds = holds();
        release(&mut holds.open, &self.start);
        // Later touches record a newer epoch, outside [start, end).
        let end = HOT_EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
        *holds.closed.entry((self.start, end)).or_default() += 1;
        self.end = Some(end);
        publish_oldest(&holds);
    }
}

impl Drop for HotHold {
    fn drop(&mut self) {
        let mut holds = holds();
        match self.end {
            None => release(&mut holds.open, &self.start),
            Some(end) => release(&mut holds.closed, &(self.start, end)),
        }
        publish_oldest(&holds);
    }
}

/// The epoch a body touched now records, or `None` with no hold open.
pub(crate) fn hot_epoch() -> Option<u64> {
    (OLDEST_OPEN.load(Ordering::Relaxed) != u64::MAX).then(|| HOT_EPOCH.load(Ordering::Relaxed))
}

/// The live holds at the start of a pass.
struct HotSet {
    oldest_open: u64,
    /// Closed intervals, merged, sorted and disjoint.
    closed: Vec<(u64, u64)>,
}

impl HotSet {
    fn now() -> Self {
        let holds = holds();
        let mut closed: Vec<(u64, u64)> = Vec::new();
        for &(start, end) in holds.closed.keys() {
            match closed.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => closed.push((start, end)),
            }
        }
        Self {
            oldest_open: holds.open.keys().next().copied().unwrap_or(u64::MAX),
            closed,
        }
    }

    /// Whether a body stamped `stamp` (0: never under a hold) is hot.
    fn contains(&self, stamp: u64) -> bool {
        if stamp == 0 {
            return false;
        }
        if stamp >= self.oldest_open {
            return true;
        }
        let i = self.closed.partition_point(|&(start, _)| start <= stamp);
        i > 0 && stamp < self.closed[i - 1].1
    }
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
/// then history-held bodies, then the rest, and only then the hot ones of
/// each (#1290); pack, then spill, in each class. Pinned bodies stay.
pub fn run_pass(need: u64) -> PassReport {
    DISPLAY_DROP.fetch_add(1, Ordering::SeqCst);
    let mut report = PassReport::default();
    if need == 0 {
        return report;
    }
    let hot_set = HotSet::now();
    let mut all = bodies();
    // A tile released from a view hold (nonzero stamp) goes before a
    // never-held tile (0), regardless of the registry's shard order.
    all.sort_by_key(|weak| std::cmp::Reverse(weak.upgrade().map_or(0, |body| body.hot_stamp())));
    for (hot, history) in [(false, true), (false, false), (true, true), (true, false)] {
        let in_class = |body: &dyn Evictable| {
            body.history_held() == history && hot_set.contains(body.hot_stamp()) == hot
        };
        let spill = may_spill();
        for weak in &all {
            if report.freed >= need {
                break;
            }
            let Some(body) = weak.upgrade() else {
                continue;
            };
            // The clock bit is a cold tile's second chance; a hot tile is
            // reached only when nothing colder is left.
            if !in_class(&*body) || body.pinned() || (!hot && body.take_referenced()) {
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
                if in_class(&*body) && body.residency() == Residency::Packed {
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

// The allocating thread's estimate (#1290): the used bytes of the last host
// reading, the threshold above which it evicts, and the bytes allocated since.
static BASE_USED: AtomicU64 = AtomicU64::new(0);
static URGENT_AT: AtomicU64 = AtomicU64::new(u64::MAX);
static SINCE: AtomicU64 = AtomicU64::new(0);
/// `SINCE` at the last re-read over the threshold; `u64::MAX` for none since
/// the reading, so the first crossing re-reads at once.
static CHECKED: AtomicU64 = AtomicU64::new(u64::MAX);
/// Over the threshold, the allocating thread re-reads the host at most once
/// per this many bytes allocated.
const RECHECK_BYTES: u64 = 4 * 1024 * 1024;

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
    let Some((used, ceiling, _)) = record(&governor().reading_fresh()) else {
        return PassReport::default();
    };
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
    freed
}

/// The memory in use above which the allocating thread evicts: the ceiling
/// under a hard limit, whose headroom is all that stands before an OOM kill
/// (#1290); else 125 % of it.
fn urgent_threshold(reading: &HostReading, ceiling: u64) -> u64 {
    if reading.cgroup_max.is_some() {
        ceiling
    } else {
        ceiling + ceiling / 4
    }
}

/// Start the allocation estimate from `reading`; returns its used bytes,
/// ceiling and urgent threshold, or `None` without a ceiling.
fn record(reading: &HostReading) -> Option<(u64, u64, u64)> {
    let Some(ceiling) = ceiling_of(reading, governor().usage_percent()) else {
        URGENT_AT.store(u64::MAX, Ordering::Relaxed);
        return None;
    };
    let used = used_of(reading).unwrap_or(0);
    let at = urgent_threshold(reading, ceiling);
    BASE_USED.store(used, Ordering::Relaxed);
    SINCE.store(0, Ordering::Relaxed);
    CHECKED.store(u64::MAX, Ordering::Relaxed);
    URGENT_AT.store(at, Ordering::Relaxed);
    Some((used, ceiling, at))
}

/// A new dense body of `bytes` is about to be allocated. Once the last
/// reading plus what was allocated since crosses the urgent threshold, the
/// allocating thread re-reads the host and, when over it, evicts first: a
/// burst between background passes is caught before a hard limit kills.
pub(crate) fn on_allocate(bytes: u64) {
    if !is_enabled() || RECLAIMING.get() {
        return;
    }
    let since = SINCE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    let at = URGENT_AT.load(Ordering::Relaxed);
    if BASE_USED.load(Ordering::Relaxed).saturating_add(since) <= at {
        return;
    }
    let checked = CHECKED.load(Ordering::Relaxed);
    if checked != u64::MAX && since.saturating_sub(checked) < RECHECK_BYTES {
        return;
    }
    CHECKED.store(since, Ordering::Relaxed);
    let Some((used, ceiling, at)) = record(&governor().reading_fresh()) else {
        return;
    };
    if used <= at {
        return;
    }
    let freed = reclaim((used - ceiling / 10 * 9).max(bytes).max(RECHECK_BYTES));
    // The reading is stale by what was freed; re-read after the next step.
    BASE_USED.store(used.saturating_sub(freed), Ordering::Relaxed);
    CHECKED.store(0, Ordering::Relaxed);
}

/// Restart the allocation estimate from a (possibly pinned) reading (tests).
pub fn refresh_urgency() {
    record(&governor().reading_fresh());
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
