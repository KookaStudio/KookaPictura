# Test plan

Every scenario is automated in the existing `tst_edit_refresh` suite; no new
suite is registered. The tests drive the real window so the debounced dock pass,
the panel signal connection, and the frame's one-pass skip are all exercised
together.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pl_visibility_eye` | Open a document with many layers, settle the 120 ms debounce, record `LayersPanel::refreshCountForTest()`, and toggle a row's eye through `set_layers_visible`. | `rowVisibleForTest()` follows the document and the refresh count does not move after another debounce window. | `tst_edit_refresh::visibilityTogglePatchesRowsWithoutARebuild` |
| `pl_visibility_external` | On the same settled document, toggle a top-level layer through `set_layer_visible`, the entry point used by the menu, Properties, and the control bridge. | The row shows the new visibility and the refresh count does not move. | `tst_edit_refresh::visibilityTogglePatchesRowsWithoutARebuild` |
| `pl_visibility_following` | After the visibility toggles, commit a non-visibility edit (rename a layer) and wait for the debounce. | The Layers panel rebuilds exactly once for the rename, proving the skip is scoped to the visibility pass. | `tst_edit_refresh::visibilityTogglePatchesRowsWithoutARebuild` |
| `pl_visibility_solo` | Call `apply_visibility` to show one layer and hide every other node, then wait for the debounce. | The Layers panel rebuilds its snapshot, because every row changed. | `tst_edit_refresh::soloVisibilityRebuildsTheLayersPanel` |

## Execution during apply

- Put both test functions in `crates/pictura-app/cpp/tests/tst_edit_refresh.cpp`;
  the suite is already registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- Finish all source/test edits before building; follow the repository's
  compile-last rule and 80% CPU limit.
- After the build, run `tst_edit_refresh` and the Layers-panel suites, then
  `python3 scripts/check-test-plan.py`.
