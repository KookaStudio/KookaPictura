# Proposal: A moved link set recomputes its move preview

## Why

Follow-up to `linked-move-preview-fix` (#959). A rendered link set's preview
image is composited at the layers' positions and clipped to the canvas, so it
changes whenever the layers move. The move-preview cache keyed only on the group
and the content revision — neither changes on a move — so after dragging a set
the next drag reused the image composited at the old positions and drew it at
the new origin. The layers previewed (and appeared to land) away from the
pointer; the larger the previous move, the larger the error.

## What Changes

- The move-preview cache also keys on the move group's union rect, so a moved
  set recomputes its preview and an unmoved set still reuses it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/canvas-tools`: an ADDED "Move preview freshness" requirement.

## Impact

- Rust: `cxxqt_object/impl_transform/mod.rs` (`move_group_union`),
  `impl_transform/session.rs` (`move_cache_valid`, `compute_move_preview`),
  `state.rs` (`move_prepared_union`).
- Tests: `tst_linked_layers::linkedMovePreviewOriginMatchesTheCrop` extended.
