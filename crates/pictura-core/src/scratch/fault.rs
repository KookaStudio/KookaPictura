//! Failure injection for the scratch disk (#1217 step 5): a full disk, a read
//! error, a corrupted write, and a removed volume. Process-wide, armed until
//! cleared.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// A failure to inject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The configured volume at this rank has no space left.
    Full { volume: usize },
    /// Every read fails.
    ReadError,
    /// Every write lands with its first byte flipped.
    Corrupt,
    /// The configured volume at this rank is gone: its reads and writes fail.
    RemoveVolume { volume: usize },
}

static FULL: AtomicU8 = AtomicU8::new(0);
static REMOVED: AtomicU8 = AtomicU8::new(0);
static READ_ERROR: AtomicBool = AtomicBool::new(false);
static CORRUPT: AtomicBool = AtomicBool::new(false);

fn bit(volume: usize) -> u8 {
    1u8.checked_shl(volume as u32).unwrap_or(0)
}

/// Arm `fault`.
pub fn inject(fault: Fault) {
    match fault {
        Fault::Full { volume } => {
            FULL.fetch_or(bit(volume), Ordering::SeqCst);
        }
        Fault::RemoveVolume { volume } => {
            REMOVED.fetch_or(bit(volume), Ordering::SeqCst);
        }
        Fault::ReadError => READ_ERROR.store(true, Ordering::SeqCst),
        Fault::Corrupt => CORRUPT.store(true, Ordering::SeqCst),
    }
}

/// Disarm every fault.
pub fn clear_faults() {
    FULL.store(0, Ordering::SeqCst);
    REMOVED.store(0, Ordering::SeqCst);
    READ_ERROR.store(false, Ordering::SeqCst);
    CORRUPT.store(false, Ordering::SeqCst);
}

pub(super) fn full(rank: Option<usize>) -> bool {
    rank.is_some_and(|r| FULL.load(Ordering::SeqCst) & bit(r) != 0)
}

pub(super) fn removed(rank: Option<usize>) -> bool {
    rank.is_some_and(|r| REMOVED.load(Ordering::SeqCst) & bit(r) != 0)
}

pub(super) fn read_error() -> bool {
    READ_ERROR.load(Ordering::SeqCst)
}

pub(super) fn corrupt() -> bool {
    CORRUPT.load(Ordering::SeqCst)
}
