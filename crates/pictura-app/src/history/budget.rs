//! The byte cap on history (#1207, #1211, #1215). Twenty states of a large
//! document can hold more than the host has, so after each capture the oldest
//! states are dropped while the tile bytes history holds beyond the live
//! document exceed the governor's history budget. Those bytes come from the
//! tile census, each tile counted once however many states share it. One undo
//! is always kept, and named snapshots are never dropped for the budget.

use super::{planes, History};
use pictura_core::Document;
use std::collections::HashSet;

/// The bytes of `doc`'s planes, each allocation counted once.
pub(crate) fn document_bytes(doc: &Document) -> u64 {
    let mut seen = HashSet::new();
    let mut bytes = 0u64;
    planes::each_plane(doc, &mut |p, _, _, _| {
        for (ptr, len) in p.allocations() {
            if len > 0 && seen.insert(ptr) {
                bytes += len as u64;
            }
        }
    });
    bytes
}

/// What a copy of `doc`'s tile tables holds: a tile handle, its generation
/// and its settle flag for each tile of each tracked plane, sharing the tiles.
pub(crate) fn tile_table_bytes(doc: &Document) -> u64 {
    const PER_TILE: u64 = 64;
    let mut tiles = 0u64;
    planes::each_plane(doc, &mut |p, _, _, _| tiles += p.tile_count().max(1) as u64);
    tiles * PER_TILE
}

impl History {
    /// Drop the oldest states while the bytes history holds beyond the live
    /// document exceed the budget, keeping the current state and one undo.
    pub(super) fn enforce_byte_cap(&mut self) {
        #[cfg(test)]
        self.check_census();
        let Some(cap) = self.byte_cap() else {
            return;
        };
        while self.states.len() > 2 && self.held_bytes() > cap {
            self.drop_oldest_state();
        }
        #[cfg(test)]
        self.check_census();
    }

    fn byte_cap(&self) -> Option<u64> {
        #[cfg(test)]
        if self.byte_cap.is_some() {
            return self.byte_cap.map(|cap| cap as u64);
        }
        pictura_core::memory::governor().history_budget()
    }

    /// Set the byte cap in place of the host's.
    #[cfg(test)]
    pub(crate) fn set_byte_cap(&mut self, cap: usize) {
        self.byte_cap = Some(cap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_target::LayerTarget;
    use pictura_core::{BitDepth, ColorMode};

    /// Samples that differ across the plane, so its tiles stay dense.
    fn pattern(seed: u8) -> Vec<u8> {
        (0..64 * 256 * 3)
            .map(|i: usize| (i as u8).wrapping_mul(7).wrapping_add(seed))
            .collect()
    }

    fn doc(seed: u8) -> Document {
        let mut doc = Document::new(64, 256, ColorMode::Rgb, BitDepth::Eight);
        doc.composite.data = pattern(seed).into();
        doc
    }

    #[test]
    fn the_oldest_states_go_when_history_outgrows_its_cap() {
        let plane = 64 * 256 * 3;
        let mut history = History::default();
        history.set_byte_cap(plane * 5 / 2);
        let target = LayerTarget::default();
        let mut live = doc(0);
        history.capture_live(&mut live, &None, &target, "Open");
        for seed in 1..=8u8 {
            live.composite.data = pattern(seed).into();
            history.capture_live(&mut live, &None, &target, "Edit");
            assert!(
                history.count() <= 2 || history.held_bytes() as usize <= plane * 5 / 2,
                "history holds {} bytes over its cap after edit {seed}",
                history.held_bytes() as usize
            );
        }
        assert!(history.count() < 9, "no state was dropped");
        assert!(history.count() >= 2, "the last undo was dropped");
        let mut selection = None;
        let mut restored_target = LayerTarget::default();
        history
            .undo_live(&mut live, &mut selection, &mut restored_target)
            .expect("one undo is kept");
        assert_eq!(live.composite.data, pattern(7));
    }

    #[test]
    fn the_history_cap_is_half_the_ceiling() {
        use pictura_core::memory::{self, HostReading};
        let plane = 64 * 256 * 3;
        // A cgroup limit of five composites plus the 256 MiB headroom: a
        // ceiling of five composites, so a cap of two and a half.
        memory::set_host_for_test(Some(HostReading {
            cgroup_max: Some(256 * 1024 * 1024 + 5 * plane as u64),
            cgroup_current: Some(0),
            ..HostReading::default()
        }));
        let mut history = History::default();
        let target = LayerTarget::default();
        let mut live = doc(0);
        history.capture_live(&mut live, &None, &target, "Open");
        for seed in 1..=8u8 {
            live.composite.data = pattern(seed).into();
            history.capture_live(&mut live, &None, &target, "Edit");
            assert!(history.count() <= 2 || history.held_bytes() as usize <= plane * 5 / 2);
        }
        memory::set_host_for_test(None);
        assert!(history.count() < 9, "no state was dropped");
        let mut selection = None;
        let mut restored_target = LayerTarget::default();
        history
            .undo_live(&mut live, &mut selection, &mut restored_target)
            .expect("one undo is kept");
        assert_eq!(live.composite.data, pattern(7));
    }

    #[test]
    fn a_roomy_cap_keeps_every_state() {
        let mut history = History::default();
        history.set_byte_cap(usize::MAX);
        let target = LayerTarget::default();
        let mut live = doc(0);
        history.capture_live(&mut live, &None, &target, "Open");
        for seed in 1..=5u8 {
            live.composite.data = pattern(seed).into();
            history.capture_live(&mut live, &None, &target, "Edit");
        }
        assert_eq!(history.count(), 6);
    }
}
