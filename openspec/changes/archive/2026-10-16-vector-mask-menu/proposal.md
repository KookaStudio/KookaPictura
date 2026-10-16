# Proposal

## Why

Issue #480: the Layers panel offers no commands on a vector-mask thumbnail,
and the trash deletes a doubly-masked layer outright where CS6 asks to delete
its vector mask (`docs/05-layers/layer-masks.md`).

## What Changes

- **Vector mask menu:** right-clicking the vector-mask thumbnail offers
  Disable/Enable Vector Mask, Delete Vector Mask and Rasterize Vector Mask,
  each acting on the clicked layer as one undo state.
- **Trash on a doubly-masked layer:** when the single selected layer has both
  masks, the trash asks "Delete Vector Mask?" (with "Don't show again", kept in
  the session) and deletes only that layer's vector mask. It reads the
  selected row, not the current row, which can sit outside the selection.
- **BREAKING**: none.

## Capabilities

- Modified `ui/layers-mask-commands`: two added requirements.
