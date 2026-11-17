# Proposal

Tracking issue: #881.

## Why

Liquify edits the mesh inside the dialog, and the only recovery is Restore All
or Reconstruct, both of which throw away the whole warp. Ctrl+Z and Ctrl+Shift+Z
do nothing, so one bad stroke costs the session. CS6 undoes Liquify paint
strokes inside the dialog.

## What Changes

- The Rust Liquify session keeps a bounded undo/redo history of mesh snapshots
  (distortion and freeze mask), one entry per brush stroke and one per
  whole-mesh action.
- `Ctrl+Z` undoes the most recent action and `Ctrl+Shift+Z` redoes it; the
  preview refreshes to the restored mesh.
- A new action after undo discards the redo history; an empty undo or redo is a
  no-op.
- OK still commits one document history state; Cancel still discards the
  session and its history.
- Qt Test coverage for strokes, freeze-mask actions, redo invalidation, the
  empty case, and the history bound.

## Capabilities

### Added Capabilities

- `imaging/liquify-history`: per-stroke undo and redo inside the Liquify
  dialog, bounded and independent of the document history.

## Impact

- `crates/pictura-app/src/cxxqt_object/filter_tools/liquify.rs`: bridge
  functions.
- `crates/pictura-app/src/cxxqt_object/filter_tools/liquify/imp.rs`: session
  history.
- `crates/pictura-app/cpp/liquify_dialog.h`, `liquify_dialog.cpp`: Undo/Redo
  shortcuts.
- `crates/pictura-app/cpp/tests/tst_liquify_dialog.cpp`: tests.
