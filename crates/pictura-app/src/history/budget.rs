//! The byte cap on history (#1207, #1211). Twenty states of a large document
//! can hold more than the host has, so after each capture the oldest states are
//! dropped while what history holds beyond the live document exceeds half the
//! memory governor's ceiling. One undo is always kept.

use super::{planes, History, Stored};
use pictura_core::Document;
use std::collections::HashSet;

/// History may hold one part in `SHARE` of the governor's ceiling.
const SHARE: u64 = 2;

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

impl History {
    /// Drop the oldest states while the bytes history holds beyond `live`
    /// exceed the cap, keeping the current state and one undo.
    pub(super) fn enforce_byte_cap(&mut self, live: &Document) {
        let Some(cap) = self.byte_cap() else {
            return;
        };
        while self.states.len() > 2 && self.held_bytes(live) > cap {
            self.drop_oldest_state();
        }
    }

    fn byte_cap(&self) -> Option<usize> {
        #[cfg(test)]
        if self.byte_cap.is_some() {
            return self.byte_cap;
        }
        pictura_core::memory::governor()
            .ceiling()
            .map(|ceiling| (ceiling / SHARE) as usize)
    }

    /// Set the byte cap in place of the host's.
    #[cfg(test)]
    pub(crate) fn set_byte_cap(&mut self, cap: usize) {
        self.byte_cap = Some(cap);
    }

    /// The bytes history keeps alive that `live` does not share: retained
    /// planes' allocations (each counted once) and stored delta tiles.
    fn held_bytes(&self, live: &Document) -> usize {
        let mut shared = HashSet::new();
        planes::each_plane(live, &mut |p, _, _, _| {
            shared.extend(p.allocations().into_iter().map(|(ptr, _)| ptr));
        });
        let mut bytes = 0;
        let mut count = |doc: &Document| {
            planes::each_plane(doc, &mut |p, _, _, _| {
                for (ptr, len) in p.allocations() {
                    if len > 0 && shared.insert(ptr) {
                        bytes += len;
                    }
                }
            });
        };
        for entry in &self.states {
            match &entry.stored {
                Stored::Full(s) | Stored::Hollow(s) => count(&s.doc),
                Stored::Delta(d) => count(&d.meta),
            }
        }
        if let Some(current) = &self.current {
            count(&current.doc);
        }
        if let Some(pinned) = &self.pinned {
            count(&pinned.doc);
        }
        for named in &self.snapshots {
            count(&named.snapshot.doc);
        }
        let tiles: usize = self
            .states
            .iter()
            .filter_map(|e| match &e.stored {
                Stored::Delta(d) => Some(d),
                _ => None,
            })
            .flat_map(|d| d.tiles.iter())
            .flat_map(|p| p.tiles.iter())
            .map(|t| t.bytes.len())
            .sum();
        bytes + tiles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_target::LayerTarget;
    use pictura_core::{BitDepth, ColorMode};

    fn doc(seed: u8) -> Document {
        let mut doc = Document::new(64, 256, ColorMode::Rgb, BitDepth::Eight);
        doc.composite.data = vec![seed; 64 * 256 * 3].into();
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
            live.composite.data.as_mut_slice().fill(seed);
            history.capture_live(&mut live, &None, &target, "Edit");
            assert!(
                history.count() <= 2 || history.held_bytes(&live) <= plane * 5 / 2,
                "history holds {} bytes over its cap after edit {seed}",
                history.held_bytes(&live)
            );
        }
        assert!(history.count() < 9, "no state was dropped");
        assert!(history.count() >= 2, "the last undo was dropped");
        let mut selection = None;
        let mut restored_target = LayerTarget::default();
        history
            .undo_live(&mut live, &mut selection, &mut restored_target)
            .expect("one undo is kept");
        assert!(live.composite.data.iter().all(|&v| v == 7));
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
            live.composite.data.as_mut_slice().fill(seed);
            history.capture_live(&mut live, &None, &target, "Edit");
            assert!(history.count() <= 2 || history.held_bytes(&live) <= plane * 5 / 2);
        }
        memory::set_host_for_test(None);
        assert!(history.count() < 9, "no state was dropped");
        let mut selection = None;
        let mut restored_target = LayerTarget::default();
        history
            .undo_live(&mut live, &mut selection, &mut restored_target)
            .expect("one undo is kept");
        assert!(live.composite.data.iter().all(|&v| v == 7));
    }

    #[test]
    fn a_roomy_cap_keeps_every_state() {
        let mut history = History::default();
        history.set_byte_cap(usize::MAX);
        let target = LayerTarget::default();
        let mut live = doc(0);
        history.capture_live(&mut live, &None, &target, "Open");
        for seed in 1..=5u8 {
            live.composite.data.as_mut_slice().fill(seed);
            history.capture_live(&mut live, &None, &target, "Edit");
        }
        assert_eq!(history.count(), 6);
    }
}
