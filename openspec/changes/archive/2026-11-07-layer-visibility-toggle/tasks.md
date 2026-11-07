# Tasks

Tests are written before their corresponding production edits. Execution is
deferred until all code is finished, following the compile-last rule.

## 1. Qt coverage

- [x] 1.1 Add `visibilityTogglePatchesRowsWithoutARebuild` to `tst_edit_refresh.cpp`: settle the debounce, toggle through `set_layers_visible` and `set_layer_visible` on a document with many layers, assert the row visibility follows and `refreshCountForTest()` stays put, then assert a following rename still rebuilds once. Covers `pl_visibility_eye`, `pl_visibility_external`, and `pl_visibility_following`.
- [x] 1.2 Add `soloVisibilityRebuildsTheLayersPanel` to `tst_edit_refresh.cpp`: call `apply_visibility` and assert the Layers panel rebuilds. Covers `pl_visibility_solo`.

## 2. Visibility signal

- [x] 2.1 Declare `layerVisibilityChanged(QStringList paths, bool visible)` on `PictureView` and emit it from `set_layer_visible` and `set_layers_visible` only when the edit changed something; leave `apply_visibility` on the `changed` path.

## 3. Layers panel

- [x] 3.1 Connect `layerVisibilityChanged` from `LayersPanel::setView` (through `connectVisibility`), patch the rows through `LayersModel::setVisible`, and expose a one-shot `consumeVisibilityPatch()` seam.
- [x] 3.2 Remove the redundant row patch from the eye handler in `layers_panel_events.cpp`; the synchronous signal now owns it.

## 4. Dock pass

- [x] 4.1 Skip the Layers panel in `refreshDock` when `consumeVisibilityPatch()` reports the pass is already covered, while still refreshing History and the remaining panels.

## 5. Verification and delivery

- [x] 5.1 After all source and test edits are complete, format and perform the production build using at most 80% of available CPUs; rebuild only when a compiler fix changes code.
- [x] 5.2 Run `tst_edit_refresh` plus the Layers-panel and panel-refresh suites named in the test plan, then the applicable repository gates; verify the new checks execute rather than skip.
- [x] 5.3 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every scenario row maps to a real test.
- [x] 5.4 Record the shipped toggle in `ROADMAP.md` under Engine and performance with #1197.
- [x] 5.5 Archive `layer-visibility-toggle` and include every manual test-plan row in the implementation PR body.
