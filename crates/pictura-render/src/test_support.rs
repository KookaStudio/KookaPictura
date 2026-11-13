//! Process-wide test state that several unit-test modules share.

use std::sync::{Mutex, MutexGuard};

static GAMMA_LOCK: Mutex<()> = Mutex::new(());

/// Serializes tests that read or mutate the process-wide text gamma.
pub(crate) fn gamma_guard() -> MutexGuard<'static, ()> {
    GAMMA_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

static SCRATCH_LOCK: Mutex<()> = Mutex::new(());

/// Serializes tests that re-point the process-wide scratch volumes and run
/// eviction passes, so one test's volumes and spill counts are not another's.
pub(crate) fn scratch_guard() -> MutexGuard<'static, ()> {
    SCRATCH_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
