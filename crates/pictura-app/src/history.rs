use crate::layer_target::LayerTarget;
use pictura_core::{Document, PsdRect};
use pictura_select::Selection;

mod planes;
mod styles;

use planes::{
    adopt_metadata, apply_tiles, diff_into, hollow, plane_kinds, planes_agree, private_copy,
    shared_copy, stamp_agreed, swap_tiles, tile_rect, PlaneDelta, PlaneKind,
};
pub(crate) use planes::{copy_plane, detached_copy};
pub use styles::{
    create_character_style, create_paragraph_style, delete_character_style, delete_paragraph_style,
    edit_character_style, edit_paragraph_style, rename_character_style,
};

#[derive(Clone)]
pub struct Snapshot {
    pub doc: Document,
    pub selection: Option<Selection>,
}

/// Where restoring a state changed the picture of the live document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restored {
    /// The composite changed only inside this rectangle; an empty one means
    /// nowhere.
    Region(PsdRect),
    /// The change has no bound to report: redraw the whole document.
    Everywhere,
}

/// A state's delta against the previous state within one geometry-stable
/// segment: the new metadata plus the changed tiles, each holding one side of
/// the change (see [`planes::TileDelta`]). `meta` is the new document with
/// every tracked plane cleared.
///
/// ponytail: tiles are uncompressed; RLE them if a full-layer filter's delta
/// (one copy of the layer and composite) ever needs to shrink further.
struct Delta {
    meta: Document,
    tiles: Vec<PlaneDelta>,
    selection: Option<Selection>,
}

/// A state is an anchor (the oldest state, or a geometry change that breaks
/// the delta chain) or a delta against the previous state. The anchor of the
/// segment holding the cursor is hollow: its pixels are `current` with the
/// segment's deltas reverted, so history never holds that state twice. Every
/// other anchor is full.
enum Stored {
    Full(Snapshot),
    /// Metadata and selection only; every tracked plane cleared.
    Hollow(Snapshot),
    Delta(Delta),
}

struct Entry {
    /// Never reused while the state is retained, so a saved identity cannot
    /// accidentally match a different state after a drop or truncation.
    id: u64,
    label: String,
    stored: Stored,
    /// The live target before the operation that created this state.
    target_before: LayerTarget,
    /// The live target after the operation that created this state.
    target_after: LayerTarget,
}

/// How a restore picks the layer target: an undo reverses one operation, a
/// redo re-applies one, and a jump lands on a state the user picked.
#[derive(Clone, Copy)]
enum TargetMode {
    Undo,
    Redo,
    Jump,
}

struct Named {
    label: String,
    snapshot: Snapshot,
    target: LayerTarget,
}

/// The History Brush's source: a state, a named snapshot, or a state the stack
/// has since dropped. The default is the oldest state, CS6's opening snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrushSource {
    #[default]
    Oldest,
    State(usize),
    Snapshot(usize),
    /// Dropped by the depth limit or a new capture after undo; kept in
    /// `History::pinned` so the brush never reads a discarded state.
    Pinned,
}

/// Bounded undo/redo over labeled `(Document, Selection)` states plus up to
/// [`MAX_SNAPSHOTS`] named restore points.
///
/// History keeps one materialized state, `current`, the state at the cursor,
/// in tracked planes the live document never shares: a write to a live tracked
/// plane then happens in place rather than copying a whole plane, and a capture
/// finds what changed by comparing the two. Untracked stores (native samples,
/// smart-object payloads, preserved byte blocks, and the selection) stay shared
/// with the live document until one side writes them. Every other state is
/// stored as the 64×64 tiles that changed against its predecessor, so retention
/// scales with the edited area. A geometry or structure change starts a new
/// segment at an anchor.
#[derive(Default)]
pub struct History {
    states: Vec<Entry>,
    next_id: u64,
    cursor: usize,
    snapshots: Vec<Named>,
    current: Option<Snapshot>,
    brush_source: BrushSource,
    pinned: Option<Snapshot>,
    current_target: LayerTarget,
}

const MAX_DEPTH: usize = 20;
const MAX_SNAPSHOTS: usize = 10;

/// The picture change a walk through the states accumulates.
struct Damage {
    kinds: Vec<PlaneKind>,
    height: usize,
    rect: Option<PsdRect>,
    everywhere: bool,
}

impl Damage {
    fn new(doc: &Document) -> Self {
        Self {
            kinds: plane_kinds(doc),
            height: doc.height as usize,
            rect: None,
            everywhere: false,
        }
    }

    /// The document rows a composite tile covers. The composite stacks its
    /// channels in one plane, so a tile row is a document row modulo the
    /// height, and a tile straddling two channels covers the whole height.
    fn rows(&self, tile: &planes::TileDelta) -> (i32, i32) {
        let h = self.height.max(1);
        let (first, last) = (tile.y / h, (tile.y + tile.h - 1) / h);
        if first == last {
            ((tile.y % h) as i32, ((tile.y + tile.h - 1) % h + 1) as i32)
        } else {
            (0, h as i32)
        }
    }

    fn add(&mut self, deltas: &[PlaneDelta]) {
        for delta in deltas {
            match self.kinds.get(delta.index) {
                Some(PlaneKind::Composite) => {
                    for tile in &delta.tiles {
                        let (top, bottom) = self.rows(tile);
                        let r = PsdRect {
                            top,
                            bottom,
                            ..tile_rect(tile)
                        };
                        self.rect = Some(match self.rect {
                            None => r,
                            Some(u) => PsdRect {
                                top: u.top.min(r.top),
                                left: u.left.min(r.left),
                                bottom: u.bottom.max(r.bottom),
                                right: u.right.max(r.right),
                            },
                        });
                    }
                }
                Some(PlaneKind::LayerChannel) => {}
                _ => self.everywhere = true,
            }
        }
    }
}

/// Whether two states would display differently with the same composite.
fn display_differs(a: &Document, b: &Document) -> bool {
    (a.width, a.height, a.mode, a.depth) != (b.width, b.height, b.mode, b.depth)
        || a.document_icc != b.document_icc
}

impl History {
    /// Capture `snapshot` as the next state. The snapshot is consumed; see
    /// [`History::capture_live`] for capturing a live document by reference.
    #[cfg(test)]
    pub fn capture(&mut self, snapshot: Snapshot, label: &str) {
        let Snapshot { mut doc, selection } = snapshot;
        self.capture_live(&mut doc, &selection, &LayerTarget::default(), label);
    }

    /// Capture the live document as the next state: diff it against the
    /// private current state and store the changed tiles, or start a new
    /// segment when the tracked-plane geometry changed. `doc` is left sharing
    /// no plane with the history, so its next write is in place.
    pub fn capture_live(
        &mut self,
        doc: &mut Document,
        selection: &Option<Selection>,
        target: &LayerTarget,
        label: &str,
    ) {
        self.capture_live_with(doc, selection, target, label, None);
    }

    /// As [`Self::capture_live`] with a history hint: only tiles intersecting
    /// the hint are compared. An empty hint falls back to the full scan.
    pub fn capture_live_with(
        &mut self,
        doc: &mut Document,
        selection: &Option<Selection>,
        target: &LayerTarget,
        label: &str,
        hint: Option<&[PsdRect]>,
    ) {
        if matches!(self.brush_source, BrushSource::State(i) if i > self.cursor) {
            self.pin_source();
        }
        self.states.truncate(self.cursor + 1);
        let target_before = self.current_target.clone();
        self.current_target = target.clone();
        let fresh_segment = |doc: &mut Document| {
            (
                Snapshot {
                    doc: private_copy(doc),
                    selection: selection.clone(),
                },
                Stored::Hollow(Snapshot {
                    doc: hollow(doc),
                    selection: selection.clone(),
                }),
            )
        };
        let stored = match self.current.take() {
            None => {
                let (current, stored) = fresh_segment(doc);
                self.current = Some(current);
                stored
            }
            Some(mut cur) => match diff_into(&mut cur.doc, doc, hint) {
                Some(tiles) => {
                    adopt_metadata(&mut cur.doc, doc);
                    cur.selection = selection.clone();
                    stamp_agreed(&mut cur.doc, doc);
                    self.current = Some(cur);
                    Stored::Delta(Delta {
                        meta: hollow(doc),
                        tiles,
                        selection: selection.clone(),
                    })
                }
                None => {
                    let current = Snapshot {
                        doc: shared_copy(doc, &cur.doc),
                        selection: selection.clone(),
                    };
                    let stored = Stored::Hollow(Snapshot {
                        doc: hollow(doc),
                        selection: selection.clone(),
                    });
                    self.fill_anchor(cur);
                    self.current = Some(current);
                    stored
                }
            },
        };
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.states.push(Entry {
            id,
            label: label.to_string(),
            stored,
            target_before,
            target_after: target.clone(),
        });
        self.cursor = self.states.len() - 1;
        while self.states.len() > MAX_DEPTH + 1 {
            match self.brush_source {
                BrushSource::Oldest | BrushSource::State(0) => self.pin_source(),
                BrushSource::State(i) => self.brush_source = BrushSource::State(i - 1),
                _ => {}
            }
            self.drop_oldest();
        }
    }

    pub fn undo(&mut self) -> Option<Snapshot> {
        if self.cursor == 0 {
            return None;
        }
        self.step_to(self.cursor - 1, None);
        self.current_target = self.states[self.cursor].target_after.clone();
        self.current.clone()
    }

    pub fn redo(&mut self) -> Option<Snapshot> {
        if self.cursor + 1 >= self.states.len() {
            return None;
        }
        self.step_to(self.cursor + 1, None);
        self.current_target = self.states[self.cursor].target_after.clone();
        self.current.clone()
    }

    /// Undo into the live document in place, reporting where its picture
    /// changed; `None` at the oldest state.
    pub fn undo_live(
        &mut self,
        doc: &mut Document,
        selection: &mut Option<Selection>,
        target: &mut LayerTarget,
    ) -> Option<Restored> {
        (self.cursor > 0)
            .then(|| self.restore_live(self.cursor - 1, TargetMode::Undo, doc, selection, target))
    }

    /// Redo into the live document in place; `None` at the newest state.
    pub fn redo_live(
        &mut self,
        doc: &mut Document,
        selection: &mut Option<Selection>,
        target: &mut LayerTarget,
    ) -> Option<Restored> {
        (self.cursor + 1 < self.states.len())
            .then(|| self.restore_live(self.cursor + 1, TargetMode::Redo, doc, selection, target))
    }

    /// Jump the live document to state `i` in place; `None` when out of range.
    pub fn jump_live(
        &mut self,
        i: usize,
        doc: &mut Document,
        selection: &mut Option<Selection>,
        target: &mut LayerTarget,
    ) -> Option<Restored> {
        (i < self.states.len())
            .then(|| self.restore_live(i, TargetMode::Jump, doc, selection, target))
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Drop every state but the current one and every named restore point, so
    /// the stack restarts at a single anchor. The History Brush's source resets
    /// to the oldest (the kept state); no snapshot survives to point at.
    pub fn purge(&mut self) {
        self.purge_states();
        self.snapshots.clear();
        self.pinned = None;
        if let BrushSource::Snapshot(_) = self.brush_source {
            self.brush_source = BrushSource::Oldest;
        }
    }

    /// Drop every undo state but the current one, keeping the named restore
    /// points. The stack restarts at a single anchor.
    pub fn purge_states(&mut self) {
        if let Some(current) = self.current.as_ref() {
            let label = self.label(self.cursor).to_string();
            let id = self.states.get(self.cursor).map_or(0, |entry| entry.id);
            self.states = vec![Entry {
                id,
                label,
                stored: Stored::Hollow(Snapshot {
                    doc: hollow(&current.doc),
                    selection: current.selection.clone(),
                }),
                target_before: self.current_target.clone(),
                target_after: self.current_target.clone(),
            }];
            self.cursor = 0;
        } else {
            self.states.clear();
            self.cursor = 0;
        }
        self.pinned = None;
        if matches!(
            self.brush_source,
            BrushSource::Oldest | BrushSource::State(_)
        ) {
            self.brush_source = BrushSource::Oldest;
        }
    }

    /// Drop the named restore points only. The undo history is untouched.
    pub fn purge_snapshots(&mut self) {
        self.snapshots.clear();
        if let BrushSource::Snapshot(_) = self.brush_source {
            self.brush_source = BrushSource::Oldest;
        }
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.states.len()
    }

    /// Number of undoable steps (`0` at the oldest state).
    pub fn depth(&self) -> usize {
        self.cursor
    }

    /// Number of labeled states, including the current one.
    pub fn count(&self) -> usize {
        self.states.len()
    }

    /// Position of the current state, in `0..count()`.
    pub fn index(&self) -> usize {
        self.cursor
    }

    /// Identity of the state at the cursor, or `None` without states.
    pub fn current_id(&self) -> Option<u64> {
        self.states.get(self.cursor).map(|entry| entry.id)
    }

    /// Label of state `i`, or `""` when out of range.
    pub fn label(&self, i: usize) -> &str {
        self.states.get(i).map_or("", |e| e.label.as_str())
    }

    /// Restore state `i` and move the cursor to it, or `None` when out of range.
    pub fn jump(&mut self, i: usize) -> Option<Snapshot> {
        if i >= self.states.len() {
            return None;
        }
        self.step_to(i, None);
        self.current_target = self.states[self.cursor].target_after.clone();
        self.current.clone()
    }

    pub fn add_snapshot(&mut self, label: &str, snapshot: Snapshot, target: LayerTarget) {
        self.snapshots.push(Named {
            label: label.to_string(),
            snapshot,
            target,
        });
        while self.snapshots.len() > MAX_SNAPSHOTS {
            match self.brush_source {
                BrushSource::Snapshot(0) => self.pin_source(),
                BrushSource::Snapshot(i) => self.brush_source = BrushSource::Snapshot(i - 1),
                _ => {}
            }
            self.snapshots.remove(0);
        }
    }

    #[cfg(test)]
    pub(crate) fn current_snapshot(&self) -> Option<&Snapshot> {
        self.current.as_ref()
    }

    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Label of named snapshot `i`, or `""` when out of range.
    pub fn snapshot_label(&self, i: usize) -> &str {
        self.snapshots.get(i).map_or("", |s| s.label.as_str())
    }

    /// The state stored in named snapshot `i`, or `None` when out of range.
    pub fn snapshot(&self, i: usize) -> Option<Snapshot> {
        self.snapshots.get(i).map(|s| s.snapshot.clone())
    }

    /// The layer target captured with named snapshot `i`.
    pub fn snapshot_target(&self, i: usize) -> Option<LayerTarget> {
        self.snapshots.get(i).map(|s| s.target.clone())
    }

    /// Keep the history's idea of the live target current when the user moves
    /// the layer selection without recording a state.
    pub fn set_current_target(&mut self, target: &LayerTarget) {
        self.current_target = target.clone();
    }

    /// Move to `target` and bring the live document there: in place when it
    /// agreed with `current` and no anchor was crossed, otherwise as a fresh
    /// private copy.
    fn restore_live(
        &mut self,
        target: usize,
        mode: TargetMode,
        doc: &mut Document,
        selection: &mut Option<Selection>,
        layer_target: &mut LayerTarget,
    ) -> Restored {
        let leaving = self.cursor;
        let agreed = self
            .current
            .as_ref()
            .is_some_and(|cur| planes_agree(&cur.doc, doc));
        let before = self.current.as_ref().map(|cur| hollow(&cur.doc));
        let mut damage = self.step_to(target, agreed.then_some(&mut *doc));
        let cur = self
            .current
            .as_mut()
            .expect("current state is always materialized");
        if agreed && !damage.everywhere {
            adopt_metadata(doc, &cur.doc);
            stamp_agreed(&mut cur.doc, doc);
        } else {
            damage.everywhere = true;
            let installed = shared_copy(&mut cur.doc, doc);
            *doc = installed;
        }
        *selection = cur.selection.clone();
        // The target the restore carries: an undo reverses the operation and
        // lands on the target from before it, a redo re-applies it and lands
        // on the target it produced, and a jump lands on the target captured
        // with the state the user picked. The live target only follows when
        // the user left it alone.
        let (untouched, restored) = match mode {
            TargetMode::Undo => (
                self.states
                    .get(leaving)
                    .map(|entry| entry.target_after.clone()),
                self.states
                    .get(leaving)
                    .map(|entry| entry.target_before.clone()),
            ),
            TargetMode::Redo => (
                self.states
                    .get(target)
                    .map(|entry| entry.target_before.clone()),
                self.states
                    .get(target)
                    .map(|entry| entry.target_after.clone()),
            ),
            TargetMode::Jump => (
                self.states
                    .get(leaving)
                    .map(|entry| entry.target_after.clone()),
                self.states
                    .get(target)
                    .map(|entry| entry.target_after.clone()),
            ),
        };
        if Some(&*layer_target) == untouched.as_ref() {
            *layer_target = restored.unwrap_or_default();
        } else {
            layer_target.retain_resolving(doc);
        }
        self.current_target = layer_target.clone();
        if before.is_none_or(|b| display_differs(&b, &cur.doc)) {
            damage.everywhere = true;
        }
        if damage.everywhere {
            Restored::Everywhere
        } else {
            Restored::Region(damage.rect.unwrap_or(PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            }))
        }
    }

    /// The anchor of the segment holding state `i`.
    fn anchor_of(&self, i: usize) -> usize {
        (0..=i)
            .rev()
            .find(|&k| !matches!(self.states[k].stored, Stored::Delta(_)))
            .expect("state 0 is always an anchor")
    }

    /// The first state after `i`'s segment: the next anchor, or the end.
    fn segment_end(&self, i: usize) -> usize {
        (i + 1..self.states.len())
            .find(|&k| !matches!(self.states[k].stored, Stored::Delta(_)))
            .unwrap_or(self.states.len())
    }

    /// State `k`'s metadata document and selection.
    fn meta_of(&self, k: usize) -> (&Document, &Option<Selection>) {
        match &self.states[k].stored {
            Stored::Full(s) | Stored::Hollow(s) => (&s.doc, &s.selection),
            Stored::Delta(d) => (&d.meta, &d.selection),
        }
    }

    /// The states whose deltas a walk from `from` to `to` crosses, in order.
    fn crossed(from: usize, to: usize) -> Vec<usize> {
        if to < from {
            (to + 1..=from).rev().collect()
        } else {
            (from + 1..=to).collect()
        }
    }

    /// Rebuild state `to` in `snap`, a copy of state `from` of the same
    /// segment, leaving the deltas as they are. Every delta it crosses holds
    /// the side it needs: walking down from `current` meets before-bytes,
    /// walking up from `current` or a full anchor meets after-bytes.
    fn walk(&self, snap: &mut Snapshot, from: usize, to: usize) {
        for k in Self::crossed(from, to) {
            if let Stored::Delta(delta) = &self.states[k].stored {
                apply_tiles(&mut snap.doc, &delta.tiles);
            }
        }
        if from != to {
            let (meta, selection) = self.meta_of(to);
            adopt_metadata(&mut snap.doc, meta);
            snap.selection = selection.clone();
        }
    }

    /// Move the materialized state `snap` from `from` to `to` of the same
    /// segment, swapping each crossed delta's bytes with it so the delta keeps
    /// the side `snap` left, and writing the same tiles into `live` when given.
    fn walk_swapping(
        &mut self,
        snap: &mut Snapshot,
        from: usize,
        to: usize,
        mut live: Option<&mut Document>,
        damage: &mut Damage,
    ) {
        for k in Self::crossed(from, to) {
            if let Stored::Delta(delta) = &mut self.states[k].stored {
                swap_tiles(&mut snap.doc, live.as_deref_mut(), &mut delta.tiles);
                damage.add(&delta.tiles);
            }
        }
        if from != to {
            let (meta, selection) = self.meta_of(to);
            adopt_metadata(&mut snap.doc, meta);
            snap.selection = selection.clone();
        }
    }

    /// Store `cur`, the state at the cursor, reverted to its segment's anchor
    /// as that anchor's full state. The cursor is leaving the segment.
    fn fill_anchor(&mut self, mut cur: Snapshot) {
        let anchor = self.anchor_of(self.cursor);
        let mut ignored = Damage::new(&cur.doc);
        self.walk_swapping(&mut cur, self.cursor, anchor, None, &mut ignored);
        self.states[anchor].stored = Stored::Full(cur);
    }

    /// Move `current` to `target`, writing the same tiles into `live` while no
    /// anchor is crossed. The damage is `everywhere` once one is.
    fn step_to(&mut self, target: usize, mut live: Option<&mut Document>) -> Damage {
        let mut damage = Damage::new(
            &self
                .current
                .as_ref()
                .expect("current state is always materialized")
                .doc,
        );
        while self.cursor != target {
            let anchor = self.anchor_of(self.cursor);
            let end = self.segment_end(self.cursor);
            let mut cur = self.current.take().expect("current state");
            let stop = target.clamp(anchor, end - 1);
            self.walk_swapping(
                &mut cur,
                self.cursor,
                stop,
                live.as_deref_mut(),
                &mut damage,
            );
            self.cursor = stop;
            if stop == target {
                self.current = Some(cur);
                break;
            }
            damage.everywhere = true;
            live = None;
            if target < anchor {
                // Back across this segment's anchor: it keeps the state it is,
                // and the previous segment's full anchor turns hollow.
                self.states[anchor].stored = Stored::Full(cur);
                let prev = self.anchor_of(anchor - 1);
                let Stored::Full(base) = std::mem::replace(
                    &mut self.states[prev].stored,
                    Stored::Delta(Delta {
                        meta: Document::default(),
                        tiles: Vec::new(),
                        selection: None,
                    }),
                ) else {
                    unreachable!("an anchor outside the cursor's segment is full");
                };
                self.states[prev].stored = Stored::Hollow(Snapshot {
                    doc: hollow(&base.doc),
                    selection: base.selection.clone(),
                });
                let mut moved = base;
                let mut ignored = Damage::new(&moved.doc);
                self.walk_swapping(&mut moved, prev, anchor - 1, None, &mut ignored);
                self.current = Some(moved);
                self.cursor = anchor - 1;
            } else {
                // Forward into the next segment: this anchor becomes full and
                // the next one hands over its state and turns hollow.
                self.cursor = stop;
                self.fill_anchor(cur);
                let Stored::Full(next) = std::mem::replace(
                    &mut self.states[end].stored,
                    Stored::Delta(Delta {
                        meta: Document::default(),
                        tiles: Vec::new(),
                        selection: None,
                    }),
                ) else {
                    unreachable!("an anchor outside the cursor's segment is full");
                };
                self.states[end].stored = Stored::Hollow(Snapshot {
                    doc: hollow(&next.doc),
                    selection: next.selection.clone(),
                });
                self.current = Some(next);
                self.cursor = end;
            }
        }
        damage
    }

    /// Rebuild state `target` without moving the cursor.
    fn materialize(&self, target: usize) -> Snapshot {
        let anchor = self.anchor_of(target);
        match &self.states[anchor].stored {
            Stored::Full(base) => {
                let mut snap = base.clone();
                self.walk(&mut snap, anchor, target);
                snap
            }
            _ => {
                let mut snap = self.current.clone().expect("current state");
                self.walk(&mut snap, self.cursor, target);
                snap
            }
        }
    }

    /// Make state 1 an anchor, then drop state 0.
    fn drop_oldest(&mut self) {
        let promoted = match (&self.states[0].stored, &self.states[1].stored) {
            (_, Stored::Full(_) | Stored::Hollow(_)) => None,
            (Stored::Hollow(_), Stored::Delta(d)) => Some(Stored::Hollow(Snapshot {
                doc: d.meta.clone(),
                selection: d.selection.clone(),
            })),
            _ => Some(Stored::Full(self.materialize(1))),
        };
        if let Some(stored) = promoted {
            self.states[1].stored = stored;
        }
        self.states.remove(0);
        self.cursor -= 1;
    }

    #[cfg(test)]
    pub(crate) fn retained_tile_bytes(&self) -> usize {
        self.states
            .iter()
            .filter_map(|e| match &e.stored {
                Stored::Delta(d) => Some(d),
                _ => None,
            })
            .flat_map(|d| d.tiles.iter())
            .flat_map(|p| p.tiles.iter())
            .map(|t| t.bytes.len())
            .sum()
    }

    /// Bytes of the retained documents' tracked planes, counting each shared
    /// allocation once.
    #[cfg(test)]
    pub(crate) fn retained_plane_bytes(&self) -> usize {
        fn collect(doc: &Document, out: &mut Vec<pictura_core::Plane<u8>>) {
            planes::each_plane(doc, &mut |p, _, _, _| {
                if !p.is_empty() {
                    out.push(p.clone());
                }
            });
        }
        let mut all = Vec::new();
        for entry in &self.states {
            collect(
                match &entry.stored {
                    Stored::Full(s) | Stored::Hollow(s) => &s.doc,
                    Stored::Delta(d) => &d.meta,
                },
                &mut all,
            );
        }
        if let Some(current) = &self.current {
            collect(&current.doc, &mut all);
        }
        if let Some(pinned) = &self.pinned {
            collect(&pinned.doc, &mut all);
        }
        for named in &self.snapshots {
            collect(&named.snapshot.doc, &mut all);
        }
        let mut unique: Vec<&pictura_core::Plane<u8>> = Vec::new();
        let mut bytes = 0;
        for plane in &all {
            if unique.iter().any(|q| q.shares(plane)) {
                continue;
            }
            unique.push(plane);
            bytes += plane.len();
        }
        bytes
    }

    /// States retained with their whole pixels: full anchors, plus `current`.
    #[cfg(test)]
    fn full_state_count(&self) -> usize {
        self.states
            .iter()
            .filter(|e| matches!(e.stored, Stored::Full(_)))
            .count()
            + usize::from(self.current.is_some())
    }

    #[cfg(test)]
    fn anchor_count(&self) -> usize {
        self.states
            .iter()
            .filter(|e| !matches!(e.stored, Stored::Delta(_)))
            .count()
    }
}

impl History {
    pub fn brush_source(&self) -> BrushSource {
        self.brush_source
    }

    /// Point the History Brush at state or snapshot `source`; false (and no
    /// change) when it does not exist.
    pub fn set_brush_source(&mut self, source: BrushSource) -> bool {
        let exists = match source {
            BrushSource::Oldest => true,
            BrushSource::State(i) => i < self.states.len(),
            BrushSource::Snapshot(i) => i < self.snapshots.len(),
            BrushSource::Pinned => false,
        };
        if exists {
            self.brush_source = source;
            self.pinned = None;
        }
        exists
    }

    /// The snapshot the History Brush paints from.
    pub fn brush_source_doc(&self) -> Option<Snapshot> {
        match self.brush_source {
            BrushSource::Oldest => Some(self.materialize(0)),
            BrushSource::State(i) => (i < self.states.len()).then(|| self.materialize(i)),
            BrushSource::Snapshot(i) => self.snapshots.get(i).map(|s| s.snapshot.clone()),
            BrushSource::Pinned => self.pinned.clone(),
        }
    }

    fn pin_source(&mut self) {
        if matches!(self.brush_source, BrushSource::Pinned) {
            return;
        }
        let snapshot = match self.brush_source {
            BrushSource::Oldest => Some(self.materialize(0)),
            BrushSource::State(i) => (i < self.states.len()).then(|| self.materialize(i)),
            BrushSource::Snapshot(i) => self.snapshots.get(i).map(|s| s.snapshot.clone()),
            BrushSource::Pinned => None,
        };
        self.pinned = snapshot;
        self.brush_source = BrushSource::Pinned;
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_retention;
