# Tasks

## 1. Tests first

- [x] 1.1 Add the new Qt slots to
  `crates/pictura-app/cpp/tests/tst_transform_lock.cpp`:
  `layerClickAppliesThePendingTransform`, `layerClickDontApplyCancels`,
  `layerClickCancelKeepsTheTransform`, and
  `panelEditingControlsLockedWhilePending`; replace the old
  `layersPanelIsGreyedWhilePending`. Verify each slot exists and the suite
  fails before the wiring lands.
- [x] 1.2 Add a `clickRowForTest` seam to `LayersPanel` so a test moves the
  panel's active row the way a real click does.

## 2. Panel resolver

- [x] 2.1 Add `setPendingTransformResolver(std::function<bool()>)`,
  `setTransformPending(bool)`, and `resolvePendingTransform()` to `LayersPanel`
  (`layers_panel.h`, `layers_panel_target.cpp`); `resolvePendingTransform()` is
  public because the frame installs the resolver and the tool controller exposes
  it.
- [x] 2.2 Gate `writeTarget` and `writeCurrentAnchor` (`layers_panel_target.cpp`):
  resolve first only when the target or the anchor actually changes, and on a
  `Cancel` answer restore the tree selection with `applyTargetSelection`.
- [x] 2.3 Gate the tree mouse-press and double-click branches in
  `layers_panel_events.cpp` and `showContextMenu` / `performPanelMenuAction` in
  `layers_panel_menu.cpp`: resolve first; a `Cancel` answer consumes the gesture.
  A plain click that does not change the active layer asks nothing.
- [x] 2.4 Keep the editing controls disabled while a transform is pending by
  ANDing `!transformPending_` into the enabled states in `syncControls`
  (`layers_panel.cpp`) and `syncStripButtons` (`layers_panel_mask_menu.cpp`).
- [x] 2.5 Make `ToolController::resolvePendingTransform()` public
  (`tools.h`).

## 3. Frame wiring

- [x] 3.1 In `frame_transform_lock.cpp`, set the resolver to
  `[this]() { return tools_ && tools_->resolvePendingTransform(); }` in
  `installTransformLock`, and replace `layersPanel_->setEnabled(!locked)` in
  `syncTransformLock` with `layersPanel_->setTransformPending(locked)`.

## 4. Integration

- [x] 4.1 Run `cargo fmt --all --check` (pass),
  `cargo clippy --workspace --all-targets -- -D warnings` (pass),
  `cargo test --workspace --doc` (pass), and `cargo nextest run --workspace`
  (4276/4277 pass; the one failure is the pre-existing
  `pictura-render::oracle imagemagick_fixtures_reproduce` fixture-byte mismatch
  for the local ImageMagick version, unrelated to this C++-only change).
- [x] 4.2 Run `cmake -S . -B build -G Ninja && cmake --build build --parallel 12`,
  then the full `ctest --test-dir build` (196 passed) and
  `./build/pictura --headless --self-test` (471 passed).
- [x] 4.3 Run `openspec validate --all --strict` and
  `python3 scripts/check-test-plan.py`; confirm the change validates and every
  scenario row resolves to a test.
- [x] 4.4 Update `docs/dev/STATE.md` for the new panel behavior
  (`TASK-ALLOWS-DOCS`), archive the change, commit with `(#905)`, and open the
  pull request with the plan's manual rows (none) noted.
