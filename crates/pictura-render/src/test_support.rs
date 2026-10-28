//! Process-wide test state that several unit-test modules share.

use std::sync::{Mutex, MutexGuard};

static GAMMA_LOCK: Mutex<()> = Mutex::new(());

/// Serializes tests that read or mutate the process-wide text gamma.
pub(crate) fn gamma_guard() -> MutexGuard<'static, ()> {
    GAMMA_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
