//! Non-linear capture and state materialization. Non-linear history appends a
//! new state after the last one instead of truncating the states after the
//! cursor, so a branch capture is stored as a full anchor.

use super::planes::{doc_allocations, stamp_agreed, tile_planes};
use super::{Entry, History, Snapshot, Stored};
use crate::layer_target::LayerTarget;
use pictura_core::memory::Side;
use pictura_core::Document;
use pictura_select::Selection;

impl History {
    /// Append a non-linear capture as a full anchor after the last state,
    /// preserving the states after the cursor. The cursor state is stored into
    /// its segment anchor first, because `current` moves to the new state.
    ///
    /// ponytail: a non-linear branch capture keeps a whole document; the
    /// linear path stays delta-based (#1215's tiling).
    pub(super) fn append_non_linear(
        &mut self,
        doc: &mut Document,
        selection: &Option<Selection>,
        target: &LayerTarget,
        label: &str,
    ) {
        let cur = self
            .current
            .take()
            .expect("current state is always materialized");
        self.fill_anchor(cur);
        let target_before = self.current_target.clone();
        self.current_target = target.clone();
        tile_planes(doc);
        let mut current = doc.clone();
        let stored_snapshot = Snapshot {
            doc: doc.clone(),
            selection: selection.clone(),
            quick_mask: None,
        };
        self.census
            .add_all(Side::Kept, doc_allocations(&stored_snapshot.doc));
        let stored = Stored::Full(stored_snapshot);
        stamp_agreed(&mut current, doc);
        self.census.add_all(Side::Live, doc_allocations(&current));
        self.current = Some(Snapshot {
            doc: current,
            selection: selection.clone(),
            quick_mask: None,
        });
        let (history_marker, history_value) = self.history_marker();
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.states.push(Entry {
            id,
            label: label.to_string(),
            stored,
            target_before,
            target_after: target.clone(),
            history_marker,
            history_value,
        });
        self.cursor = self.states.len() - 1;
        self.enforce_depth();
        self.enforce_byte_cap();
    }

    /// Retained state `i` materialized without moving the cursor or touching
    /// the live document, or `None` when out of range.
    pub fn state_snapshot(&self, i: usize) -> Option<Snapshot> {
        (i < self.states.len()).then(|| self.materialize(i))
    }

    /// Whether a new capture after stepping back appends instead of
    /// discarding the later states.
    pub fn set_non_linear(&mut self, on: bool) {
        self.non_linear = on;
    }
}
