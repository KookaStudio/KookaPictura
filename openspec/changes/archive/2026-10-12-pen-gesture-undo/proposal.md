# Proposal: pen-gesture-undo

## Why

Issue #307: while a Pen-group tool holds a live gesture (placing an anchor and
dragging its handles, dragging an existing handle with Convert Point, or
drawing a Freeform Pen trail), Ctrl+Z does not step back that gesture. The live
edit is un-recorded by design — it commits on release — so an undo pressed
mid-gesture restores the *previous* committed edit and the release then records
a state for the discarded gesture: the last committed anchor is lost and the
new one is a no-op. On the first anchor of a path there is nothing to undo, so
Ctrl+Z does nothing at all and the release still commits.

Committed pen edits already record one document history state each, and
Ctrl+Z steps back through them; the gap is only the in-progress gesture.

## What Changes

- A live Pen-group gesture SHALL be one modal tool-session step: while it is in
  progress, Edit > Undo (Ctrl+Z) SHALL cancel the gesture (restore the path to
  its pre-press state) instead of touching the document history; the next Undo
  SHALL then step back the last committed edit.
- Releasing the pointer after a cancelled gesture SHALL record no history state.
- Committed edits (anchor place, insert, delete, close, convert click, freeform
  release) keep recording exactly one document history state each.

## Capabilities

### New Capabilities

- `tools/pen-edit-session`: the Pen-group tools' live gesture as a cancellable
  modal session step, layered on the existing modal tool-session undo contract.

## Impact

- `crates/pictura-app/src/cxxqt_object/paths.rs` (gesture backup bridge),
  `crates/pictura-app/src/cxxqt_object/state.rs` (view state field),
  `crates/pictura-app/cpp/tool_pen.cpp` (session undo on the handler).
- Tests: `crates/pictura-app/cpp/tests/tst_pen_tools.cpp`.
- No new dependency.

## Provenance

Independent fix; the pen group's per-edit document history is already recorded
(no CS6 oracle exists for path geometry or history labels). Ceiling
(`ponytail:`): the gesture is reverted from an in-view path backup, not a
replayed inverse edit.
