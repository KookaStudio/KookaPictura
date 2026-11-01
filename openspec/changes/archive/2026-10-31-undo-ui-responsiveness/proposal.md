# Proposal

## Why

Issue #1067. Undo, redo and history jumps freeze the brush ring while they run,
at a cost that grows with the document. The restore itself is already bounded:
a brush undo writes its tiles in place and reports a region. The blocking work
comes after it. Every restore runs the whole panel pass inline, and the Layers
thumbnails then copy each diverged plane into one contiguous allocation.

## What Changes

- **Panels follow the debounce.** A restore that ended no live session reports
  a new `historyRestored` signal instead of `sessionCancelled`. Tools still drop
  local gesture state, and the panels refresh on the 120 ms debounce a paint
  commit already uses. A restore that did end a live session is unchanged.
- **Thumbnails stop materializing planes.** A new `Plane::sample` reads one
  sample from its band. Layer and mask thumbnails and `mask_sample` use it, so
  their cost follows the thumbnail, not the plane.
- **Mask undo is bounded.** A layer-mask delta reports its tiles in document
  coordinates. A restore across a moved mask, or a document-channel change, still
  reports the whole document.

## Capabilities

### New Capabilities

- `document/history-restore-responsiveness`: what a history restore may do on
  the GUI thread.

### Modified Capabilities

None.

## Impact

`crates/pictura-core/src/plane.rs`, `crates/pictura-app/src/history*`,
`crates/pictura-app/src/cxxqt_object/{impl_history,helpers_composite}.rs`,
`crates/pictura-app/cpp/frame_canvas.cpp`. Structural undo stays a whole-document
restore on the GUI thread and is tracked separately.
