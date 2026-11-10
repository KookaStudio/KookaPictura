//! Unique tile bytes held beyond the live document (#1215).
//!
//! A holder of tiles (history) reports every allocation it takes or lets go
//! of, as `(address, bytes)`, on one of two sides: `Kept`, what it retains for
//! itself, and `Live`, what it holds in common with the live document. An
//! allocation reported several times counts once, and one held on both sides
//! does not count, so [`TileCensus::beyond_live`] is what letting go of
//! everything kept would free. Updates are incremental: no walk over all tiles.

use std::collections::HashMap;

/// Which side of a census an allocation is reported on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Retained by the holder alone.
    Kept,
    /// Shared with the live document.
    Live,
}

#[derive(Default)]
struct Count {
    bytes: u64,
    kept: u32,
    live: u32,
}

/// Refcounts of allocations by address, and the bytes kept and not live.
///
/// Every reported allocation is alive while it is counted (its holder keeps
/// it), so an address cannot be reused under a stale entry.
#[derive(Default)]
pub struct TileCensus {
    counts: HashMap<usize, Count>,
    beyond: u64,
}

impl TileCensus {
    /// Count one more reference to `(address, bytes)` on `side`.
    pub fn add(&mut self, side: Side, (address, bytes): (usize, usize)) {
        if bytes == 0 {
            return;
        }
        let count = self.counts.entry(address).or_default();
        count.bytes = bytes as u64;
        let only_kept = |c: &Count| c.kept > 0 && c.live == 0;
        let was = only_kept(count);
        match side {
            Side::Kept => count.kept += 1,
            Side::Live => count.live += 1,
        }
        let (now, bytes) = (only_kept(count), count.bytes);
        self.settle(was, now, bytes);
    }

    /// Count one reference fewer to `(address, bytes)` on `side`. Releasing
    /// what was never counted is a holder's bookkeeping bug: debug builds
    /// assert, release builds ignore it.
    pub fn remove(&mut self, side: Side, (address, bytes): (usize, usize)) {
        if bytes == 0 {
            return;
        }
        let Some(count) = self.counts.get_mut(&address) else {
            debug_assert!(false, "released an allocation the census never counted");
            return;
        };
        let only_kept = |c: &Count| c.kept > 0 && c.live == 0;
        let was = only_kept(count);
        let held = match side {
            Side::Kept => &mut count.kept,
            Side::Live => &mut count.live,
        };
        debug_assert!(*held > 0, "released more references than were counted");
        *held = held.saturating_sub(1);
        let now = only_kept(count);
        let (bytes, gone) = (count.bytes, count.kept == 0 && count.live == 0);
        if gone {
            self.counts.remove(&address);
        }
        self.settle(was, now, bytes);
    }

    /// Count every allocation of `all` on `side`.
    pub fn add_all(&mut self, side: Side, all: impl IntoIterator<Item = (usize, usize)>) {
        for a in all {
            self.add(side, a);
        }
    }

    /// Release every allocation of `all` on `side`.
    pub fn remove_all(&mut self, side: Side, all: impl IntoIterator<Item = (usize, usize)>) {
        for a in all {
            self.remove(side, a);
        }
    }

    /// Bytes of the allocations counted `Kept` and not `Live`, each once.
    pub fn beyond_live(&self) -> u64 {
        self.beyond
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.counts.clear();
        self.beyond = 0;
    }

    fn settle(&mut self, was: bool, now: bool, bytes: u64) {
        match (was, now) {
            (false, true) => self.beyond += bytes,
            (true, false) => self.beyond -= bytes,
            _ => {}
        }
    }
}
