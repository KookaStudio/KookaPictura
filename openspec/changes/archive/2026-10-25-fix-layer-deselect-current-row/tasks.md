# Tasks

## 1. The regression tests

- [x] 1.1 Add `userDeselectClearsCurrentRow` to
  `crates/pictura-app/cpp/tests/tst_layer_target.cpp`: select a row, clear the
  selection through the selection model, and assert the target and the tree's
  current index are both empty.
- [x] 1.2 Add `clickingEmptySpaceClearsCurrentRow` to the same suite: click the
  empty space below the rows and assert the same empty projection.
- [x] 1.3 Confirm both tests fail against the unfixed panel and pass with it.

## 2. The fix

- [x] 2.1 Clear the tree's current index in `LayersPanel::writeTarget()` when
  the selected rows are empty, under the `applyingTarget_` guard.

## 3. Verification

- [x] 3.1 `QT_QPA_PLATFORM=offscreen build/crates/pictura-app/cpp/tests/tst_layer_target`
  passes.
- [x] 3.2 `tst_layers_panel`, `tst_layers_panel_fixes`, and
  `tst_nested_layer_target` still pass.
- [x] 3.3 `openspec validate --all --strict` passes.
- [x] 3.4 Archive the change with `openspec archive fix-layer-deselect-current-row -y`,
  so the pull request leaves no open change.
