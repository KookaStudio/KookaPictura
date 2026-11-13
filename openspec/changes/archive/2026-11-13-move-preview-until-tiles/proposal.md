# Keep the Move preview until the presented tiles land

## Why

The lazy layered-RGB Move commit marks the display tiles stale and returns
immediately. The canvas dropped the drag preview at once, so the old layer
position stayed on screen until the presented tiles rebuilt, which can take
seconds at fit zoom on a large document.

## What Changes

- **The preview stays presented.** A lazy Move keeps the drag preview (the
  backdrop patch and the moved layer) while the presented display tiles rebuild.
- **The presented tiles are requested at once.** The commit queues the missing
  tiles for the view instead of waiting for the canvas to ask for them, because
  the preview paint path does not request present-crop tiles.
- **`moveCommitted` waits for those tiles.** The signal fires when the presented
  tiles are current, so the canvas drops the preview onto the exact tiles.
- **A reader settles the wait.** An edit, a save, or another Move press clears
  the lazy pending state and drops the preview before it runs.

## Capabilities

- **MODIFIED** `tools/move-commit-responsiveness`

## Impact

- `pictura-app` Rust: `state.rs`, `edit.rs`, `tiles.rs`,
  `impl_core/deferred_composite/imp.rs`.
- `pictura-app` C++ tests: `tst_large_document.cpp`.
