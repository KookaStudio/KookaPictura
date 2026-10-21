# Proposal

## Why

A hidden layer can still be moved and transformed. With `Show Transform Controls`
checked, selecting a hidden layer draws the transform box over the canvas, and a
Move drag or `Free Transform` edits its pixels even though the layer is not
visible. Paint and filter tools already refuse a hidden active layer through
`active_layer_visible`; the Move and transform paths only check the position
lock.

## What Changes

- Treat a hidden layer as untransformable: no controls box, no Free Transform,
  Skew, Distort, Perspective, or Warp session.
- Refuse the Move tool's whole-layer preview and commit on a hidden layer, and
  refuse Alt-drag duplication of it.
- Refuse selection-content moves and selection-lift transforms on a hidden
  layer, so a selection cannot bypass the visibility rule.
- Add Qt Test coverage for the controls box, the Move drag, and the transform
  commands.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/move-transform-controls`: hidden layers are not movable or
  transformable.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_transform/session.rs` — transform
  eligibility, selection lift, move preview and commit.
- `crates/pictura-app/src/cxxqt_object/impl_transform/mod.rs` — Alt-drag
  duplicate target.
- `crates/pictura-app/src/cxxqt_object/impl_selection.rs` — selection-content
  move and its live preview.
- `crates/pictura-app/cpp/tests/tst_transform_controls.cpp` — two Qt Test cases.
- No bridge signature, dependency, or data-format changes.
- No `design.md`: the visibility predicate already exists for paint/filter
  tools; this extends it to the Move and transform entry points.
