# Proposal

## Why

Issue #1290, phase 5 review. Cancel on a Free Transform commit's progress
window resumed the transform session (#1278): the handles and the scaled preview
stayed on the canvas. A user who cancels expects the layer back in its
original shape, as Esc gives. Phase 5 sends many more commits through the job,
so Cancel is now common.

## What Changes

- Cancel from the progress window ends the Free Transform as Esc does: the
  handles and the transform image go away, the layer shows its original pixels
  (a lifted selection's pixels are put back), and the document and history are
  unchanged. This applies to an Enter/Commit of a Free Transform and to a Move
  tool controls-box gesture. The Edit ▸ Transform one-shot commands already
  did this.
- The engine contract is unchanged: a cancelled job lands nothing and leaves
  the session live; the shell then ends it with `cancel_transform`.

## Capabilities

- **REMOVED + ADDED** `tools/move-commit-responsiveness`: "A large transform commit
  leaves the GUI thread" is replaced by "A large transform commit runs as a
  cancellable job", which ends the transform on cancel.

## Impact

- `pictura-app` C++: `ToolController::commitFreeTransform` and
  `commitTransformControls` call `cancel_transform` after a cancelled job.
