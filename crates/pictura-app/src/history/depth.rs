//! History States (#636, `docs/10-workflow-io/scratch-disks-and-memory.md`):
//! the process-wide bound on every document's undo states. Lowering it trims
//! open histories at once, through [`History::apply_state_limit`].

use super::{BrushSource, History};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const DEFAULT_STATES: usize = 20;
const MAX_STATES: usize = 1000;

static LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_STATES);

/// The History States preference.
pub fn state_limit() -> usize {
    LIMIT.load(Ordering::Relaxed)
}

/// Set History States, clamped to 1–1000. Open histories keep their states
/// until [`History::apply_state_limit`] runs on them.
pub fn set_state_limit(states: usize) {
    LIMIT.store(states.clamp(1, MAX_STATES), Ordering::Relaxed);
}

impl History {
    fn depth_limit(&self) -> usize {
        #[cfg(test)]
        if let Some(limit) = self.state_limit {
            return limit;
        }
        state_limit()
    }

    /// Hold at most the limit's undo states besides the current one: drop the
    /// oldest while the current state is not the oldest, then redo states from
    /// the newest end. Named snapshots stay.
    pub(super) fn enforce_depth(&mut self) {
        let keep = self.depth_limit() + 1;
        while self.states.len() > keep && self.cursor > 0 {
            self.drop_oldest_state();
        }
        if self.states.len() > keep {
            if matches!(self.brush_source, BrushSource::State(i) if i >= keep) {
                self.pin_source();
            }
            self.truncate_states(keep);
        }
    }

    /// Trim to the current History States now; true when a state went.
    pub fn apply_state_limit(&mut self) -> bool {
        let before = self.states.len();
        self.enforce_depth();
        let trimmed = self.states.len() != before;
        if trimmed {
            self.revision = self.revision.wrapping_add(1);
        }
        trimmed
    }

    /// Bound this history by `limit` in place of the process's limit.
    #[cfg(test)]
    pub(crate) fn set_state_limit_for_test(&mut self, limit: usize) {
        self.state_limit = Some(limit);
    }
}
