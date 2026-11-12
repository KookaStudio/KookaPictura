//! An edit that read tiles lost from the scratch disk changes nothing (#1217,
//! design §5): the document goes back to the history's current state, which
//! the edit has not been captured into yet, and the edit reports a typed
//! scratch refusal.
//!
//! The attribution is approximate: any read of an unreadable tile since the
//! last commit (a display rebuild included) counts, which errs toward refusing.

use super::helpers::memory_gate;
use super::presentation::Presentation;
use super::PictureViewRust;
use crate::history::Restored;

impl PictureViewRust {
    /// Restore the current history state when unreadable tiles were read
    /// since the last commit and that state holds one; the presentation of
    /// the restore, or `None` to let the edit commit.
    pub(super) fn roll_back_unreadable(&mut self) -> Option<Presentation> {
        if !memory_gate::take_unreadable_reads() {
            return None;
        }
        let prior = self.session.history.current_snapshot()?;
        if crate::history::unreadable_tiles(&prior.doc) == 0 {
            return None;
        }
        let prior = prior.clone();
        self.session.doc = Some(prior.doc);
        self.session.selection = prior.selection;
        super::metrics::cancel_edit();
        memory_gate::refuse_for_scratch();
        let dirty = self.session.derived_dirty();
        Some(self.restore_presentation(Restored::Everywhere, dirty))
    }
}

#[cfg(test)]
#[path = "scratch_rollback_tests.rs"]
pub(in crate::cxxqt_object) mod tests;
