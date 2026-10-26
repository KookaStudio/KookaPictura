# Proposal: The Move tool moves a multi-selection

## Why

Follow-up to `linked-layers-move` (#959). The Move group was built from
`layer_target.single()`, so it required exactly one selected layer. With several
layers selected — e.g. both members of a link set picked in the Layers panel —
`begin_move_preview` returned false and a drag moved nothing.

## What Changes

- The Move group is every selected movable layer plus every movable member of a
  selected layer's link set, in panel order, instead of one active layer.
- A group member must be a movable (pixel or shape) layer that is visible and
  not position-locked; hidden, locked, group, and adjustment layers are left
  out, so a selection with no movable layer still does nothing.
- The drag preview and the one `Move Layer` state cover the whole group.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/canvas-tools`: the Move tool translates every selected layer, not just
  one.
- `compositing/layer-management`: the move group is the selected layers plus
  their link sets.

## Impact

- Rust: `cxxqt_object/impl_transform/session.rs` and `mod.rs` (move group),
  `dispatch.rs` (nudge).
- Tests: `tst_linked_layers` gains `moveMultiSelectionMovesEveryLayer`.
- Docs: `docs/03-tools/move-and-transform.md`, `docs/dev/STATE.md`.
