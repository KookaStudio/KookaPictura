# Tasks

## 1. Tests first

- [x] 1.1 Add `merge_down_applies_masks_into_alpha` to `crates/pictura-render/src/document_ops/layer_ops/merge_tests.rs` covering a masked upper layer, the union alpha, and the compositor oracle; verify with `cargo test -p pictura-render merge_down_applies_masks_into_alpha`
- [x] 1.2 Add `crates/pictura-app/cpp/tests/tst_merge_commands.cpp` with `layerMenuContextSensitive` and `layersPanelContextSensitiveLabel`, covering one/two/zero selections, the label swap, `Ctrl+E` routing through `layer.merge.layers`, unconditional absence of `layer.merge.down`, refusal history, one-history-state undo, and the flyout label plus disabled state; register `tst_merge_commands` in `crates/pictura-app/cpp/tests/CMakeLists.txt` and verify it builds and is listed by `ctest -N`
- [x] 1.3 Update `crates/pictura-app/cpp/tests/tst_command_tree.cpp` to drop the retired `LayerMergeDown` coverage and the stale “Merge Down” comment, leaving that suite focused on arrange/stamp/delete/lock; verify it still compiles

## 2. One command in the Layer menu

- [x] 2.1 Remove the `LayerMergeDown` handler and enable provider from `frame_menus_layer_ops.cpp`, and remove `merge_down_run` plus its bridge declaration from `layer_arrange.rs`; verify `rg -n "merge_down_run" crates/` returns no hits
- [x] 2.2 Remove `LayerMergeDown` from `commands.h` and its row from `command_tree.cpp`; verify `rg -n "LayerMergeDown|layer\\.merge\\.down" crates/ --glob '!crates/pictura-app/cpp/tests/**'` returns no hits and `tst_merge_commands` asserts the retired id is absent
- [x] 2.3 Add a null-guarded `setLabelProvider` for `layer.merge.layers` in `frame_menus.cpp` returning `Merge Down` only for exactly one selected path and `Merge Layers` otherwise; keep it to a few lines and verify the single-, multi-, and zero-selection label assertions pass
- [x] 2.4 Update the layer-ops comment in `frame.h`, the top comment in `frame_menus_layer_ops.cpp`, the `layer_arrange.rs` module comment, and any stale `tst_command_tree.cpp` comment; verify `rg -n "Merge Down" crates/pictura-app/cpp/frame_menus_layer_ops.cpp crates/pictura-app/cpp/frame.h crates/pictura-app/src/cxxqt_object/layer_arrange.rs` reads correctly

## 3. Layers panel flyout label and enablement

- [x] 3.1 Add `LayersPanel::canMergeSelection()` as a thin wrapper around `view_ && view_->can_merge_layers(selectedPaths())`
- [x] 3.2 Give wired panel-menu actions their action id via `QAction::setData`, then update the `mergeDown` action's text and enabled state in `panel_group_menu.cpp` at build time and on `headerMenu_->aboutToShow`; verify `tst_merge_commands::layersPanelContextSensitiveLabel` passes with one, two, and invalid selections

## 4. Docs

- [x] 4.1 Update `docs/05-layers/merge-and-flatten.md` (command table at 25-26, adjacent-merge wording at 44-45, UI surface at 92-93, `LayerMergeActions` at 213, one-layer edge case at 247-248), `docs/02-ui-ux/panels/layers-panel.md:48`, and `docs/02-ui-ux/menus.md:192`; docs remain uncommitted per the no-proactive-commit rule, and `bash scripts/guard.sh` passes

## 5. Verification and archive

- [x] 5.1 Run `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`, and `bash scripts/verify-full.sh`; the only verify-full failure was the expected non-archived change check
- [x] 5.2 Archive the change with `openspec archive merge-context-sensitive-command` and re-run `openspec validate --all --strict`
