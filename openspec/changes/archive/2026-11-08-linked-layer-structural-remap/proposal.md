# Proposal: Keep links across layer inserts/deletes and align the controls box

## Why

Two follow-up problems remain after linked layers transform as one (#1250):

- Adding a new layer or deleting another layer clears every link set, so two
  linked layers lose their chain badge even when the operation did not touch
  them. Link sets are path-keyed, and the edit transaction clears them on every
  structural outcome instead of remapping the paths an insertion or deletion
  shifted.
- The Move tool's idle controls box is drawn around the active layer's content
  only, while the linked transform session's box is the union of the set. A
  press on the visible handle is then inside the union quad, so the press is
  read as a Move and the whole set translates instead of scaling.

## What Changes

- Layer insertions (new layer/group, duplicate) and deletions remap the
  path-keyed link sets instead of clearing them. A deleted member leaves its
  set; a set left with one member is dropped. Other structural edits (reorder,
  group, ungroup, merge, rasterize) still clear the links because their path
  changes are not remapped.
- The Move tool's controls box spans the link set's content union, and its cache
  key includes the set membership so linking/unlinking rebuilds it. A handle
  drag then scales/rotates the whole set instead of falling through to a Move.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: link sets survive index-shifting inserts and
  deletes.
- `tools/move-transform-controls`: the controls box and its gestures follow the
  link set.

## Impact

- Rust: `state.rs`, `edit.rs`, `impl_layers_select.rs`, `impl_layers.rs`,
  `impl_layers/target_transitions.rs`, `impl_transform/bridge*`.
- C++: `tool_transform.cpp` (controls-box cache key).
- Tests: Rust remap units, `tst_linked_layers`.
