# Proposal

## Why

Issue #1157. On a 16507×16196 document, releasing a Move drag of a
canvas-sized layer blocks the GUI thread for about 4.6 s. `commit_move`
translates the layer cheaply, then `apply_edit_move` refreshes the union of the
before and after rectangles at full resolution: a region composite,
`doc.composite`, level 0 and the pyramid. For a layer that covers the canvas,
that union is the whole document. The cost scales with area.

#1148 made the switch to Move and the press cheap; the commit was left for this
change.

## What Changes

- **A large layer move composites on a worker.** When the document and the
  union of the dirty rectangles each reach 16 MP, the commit translates the
  layer and returns, leaving the composite to a worker that patches
  `doc.composite`, level 0 and the pyramid on the GUI thread when it lands.
  Smaller moves stay synchronous.
- **The drag preview stays until the composite lands.** A new `moveCommitted`
  signal reports that the composite is current; the canvas drops the Move
  preview on it instead of on mouse release.
- **Every reader settles first.** A pending composite is finished synchronously
  before an edit, a preview, a Move press, undo/redo/history jump, a save, an
  eyedropper sample, or a filter proxy. The picture accessor treats a pending
  composite as stale, so every other `picture()` consumer gets exact pixels
  too.
- **History waits for the composite.** The move's history state is captured
  when the composite lands (or when a reader settles it), so a history diff
  never records a stale composite. A superseded job, or one whose document was
  replaced, is dropped by generation.

## Not in this change

- Transform commits (`commit_transform`) keep their synchronous refresh; they
  are a different edit path and clear their preview immediately.
- A move that is not a plain layer move (effects, groups, shapes) still takes
  the full recomposite, now deferred under the same threshold.

## Capabilities

### New Capabilities

- `tools/move-commit-responsiveness`

### Modified Capabilities

None.

## Impact

`pictura-app`: the edit transaction (`edit.rs`), the Move/transform session
(`impl_transform/session.rs`), a new `impl_core/deferred_composite.rs`, the
display accessor (`picture.rs`), history, save, eyedropper and filter-proxy
entry points, the `moveCommitted` signal, and the Move tool release handler.
No engine crate changes.
