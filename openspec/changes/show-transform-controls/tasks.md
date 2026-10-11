# Tasks: show-transform-controls

## 1. Session orchestration

- [x] 1.1 Bridge readout `transform_session_rect(view) -> QString` ("left top
  right bottom") in `impl_transform/bridge.rs`.
- [x] 1.2 `ToolController::{transformControlsSession, syncTransformControls}`
  and the `transformControls_` flag; hook the sync into `applyToolPolicy`,
  `bindCanvas` (same-canvas branch), `setMoveShowTransformControls`, and the
  frame's Layers-panel selection handler.
- [x] 1.3 Controls gestures: `handlePressed` falls through to the Move handler
  for inside/outside presses, `handleReleased` commits a handle gesture and
  re-syncs; `beginTransformImpl` lets `Ctrl+T` take the box over.

## 2. Options bar

- [x] 2.1 `OptionsBar`'s `transformChanged` handler keeps the Move page while a
  controls session is active; drop the `ponytail:` note in
  `options_bar_move.cpp`.

## 3. Verification

- [x] 3.1 `crates/pictura-app/cpp/tests/tst_transform_controls.cpp`: checking
  the box shows the session overlay with no reference point; unchecking hides
  it; a handle drag scales and records one `"Free Transform"` state with the
  box returning; an inside drag records one `"Move Layer"` state with the box
  returning; a selection, a Background layer, and a tool switch suppress it;
  `Ctrl+T` switches to the modal session.
- [x] 3.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo nextest run --workspace`, `ctest --test-dir build -R
  '^tst_'`, `bash scripts/verify-fast.sh`, `openspec validate --all --strict`.
