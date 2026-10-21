# Tasks

## 1. Layer target core

- [x] 1.1 Author `single_only_when_one`, `stale_paths_are_dropped`, and `stale_anchor_coerces_to_a_survivor`, `open_target_prefers_the_topmost_pixel_layer`, `open_target_falls_back_to_the_top_layer`, and `open_target_finds_a_pixel_inside_the_top_group` in `crates/pictura-app/src/layer_target.rs`, plus the empty/dedup cases; verify every name appears in `test-plan.md`
- [x] 1.2 Implement `LayerTarget` (`selected`, `active`, `single`, `anchor`, `select_only`, `select`, `clear`, `retain_resolving`) using `pictura_render::resolve_path`; verify the module imports no Qt type and every planned unit-test name exists
- [x] 1.3 Register the module, replace `active_layer` in `PictureViewRust`, and update every `PictureViewRust { .. }` literal in the tests (`cxxqt_object/tests.rs`, `tests_profiles.rs`, `impl_layers/matting.rs`, `filter_tools/lens_blur_depths.rs`, `filter_tools/canvas_proxy.rs`); verify `rg "active_layer" crates/pictura-app/src/cxxqt_object/state.rs` returns only the shim accessors

## 2. History carries the target

- [x] 2.1 Author `undo_restores_layer_target`, `undo_keeps_a_later_selection`, `undo_restores_structural_target`, `redo_restores_post_operation_target`, `jump_restores_target_across_multiple_states`, and `named_snapshot_restores_layer_target` in `history/tests.rs`; verify every name appears in `test-plan.md`
- [x] 2.2 Add `target_before` / `target_after` to each history `Entry`, thread the live target through `capture` / `capture_live` / `undo_live` / `redo_live` / `jump_live` / `restore_live` / `purge_states` / `history_restore_snapshot` / `history_add_snapshot`, and implement the conditional restore plus the named-snapshot rule; verify every restore and purge path assigns the target by inspection
- [x] 2.3 Keep `Snapshot` document-only and leave the existing fixture literals unchanged; verify the production constructors in `history.rs`, `impl_history.rs`, and `impl_core.rs` capture the target on the history entry
- [x] 2.4 Update `tests_profiles.rs` and `filter_tools/jobs/tests.rs` `capture_live` calls with the target argument; verify `rg "capture_live\\(" crates/pictura-app/src` shows no two-argument call remains

## 3. Bridge surface and initialization

- [x] 3.1 Author `panelSelectionUpdatesTarget`, `multiSelectionKeepsAnchor`, `multiSelectionHasNoEditTarget`, `deselectClearsTarget`, `selectionDoesNotDirtyOrRecord`, `undoFirstEditAfterOpen`, `undoRestoresStructuralTarget`, `undoKeepsLaterSelection`, `redoRestoresPostOperationTarget`, `namedSnapshotRestoresTarget`, `documentSwitchRestoresTarget`, and `newDocumentSelectsFirstLayer` in `tst_layer_target.cpp`, and register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify the file and CMake list contain every name
- [x] 3.2 Add the side bridge `cxxqt_object/layer_target_bridge.rs` with `set_layer_target`, `selected_layer_paths`, `layer_selection_anchor`, `select_layer_path`, and `clear_layer_target`; target changes ride the existing `changed` signal; declare it in `cxxqt_object.rs` and register the file in `crates/pictura-app/build.rs`; keep `set_active_layer` / `active_layer_path` as shims; verify `wc -l crates/pictura-app/src/cxxqt_object.rs` is 1214 or less
- [x] 3.3 Compute the opening target before `history_for` in `open_jobs` (carry it on `OpenedDocument`), assign it after `reset_edit_state`, and do the same for new documents, imports, image operations, and mode conversion; verify `rg "history_for"` shows the target argument and `rg "history_for\\("` no longer captures before the target exists
- [x] 3.4 Update the callers that used `set_active_layer("")` to disable tool edits (`tst_properties_panel.cpp`, `tst_filter_menu.cpp`, `selftest_active_layer.cpp`) to `clear_layer_target()` or to the new invariant; verify `rg "set_active_layer\\(QString\\(\\)\\)|set_active_layer\\(\\\"\\\"" crates/pictura-app` returns only intentional cases

## 4. The Layers panel becomes a view

- [x] 4.1 Author `deselectSurvivesRefresh`, `filterHidingTargetKeepsIt`, `multiSelectionIsShownInFull`, `filteredTargetCommands`, and `repeatedRefreshKeepsTarget`, and update `backgroundOnlyStaysActive` and `activeLayerNotFirstRow` without adding net lines; verify the names match `test-plan.md` and `wc -l tst_layers_panel.cpp` stays at 1400 or less
- [x] 4.2 Split the panel into write-on-user-change and apply-on-refresh; guard `refresh`, `applyFilter`, `enterIsolation`, `releaseIsolation`, `setView`, and `applyTargetSelection`; call `applyTargetSelection()` after every filter change; delete the `selected`/`selectedPath` capture, the `selectPaths(selected, selectedPath)` re-select, and the first-row fallback; verify no `set_active_layer` remains in `syncControls` and no `selectPaths(selected` remains in `refresh`
- [x] 4.3 Migrate `propertyPaths`, the blend handler, the header `active` gate, the blend/opacity/fill eligibility, the mask target, and the selection-based panel actions in `layers_panel_actions.cpp`, `layers_panel_menu.cpp`, `layers_panel_mask_menu.cpp`, `panel_group_menu.cpp`, and `layers_panel_events.cpp` to the Rust target; keep row-targeted actions on their clicked path; verify `rg "selectedPaths\\(\\)|currentPath\\(\\)"` in those files shows only display/test uses

## 5. Structural target transitions

- [x] 5.1 Author `createdNodesBecomeTarget`, `pasteBecomesTargetAndUndo`, `duplicateBecomesTarget`, `deleteSelectsSurvivor`, `groupAndUngroupSelectResult`, `mergeSelectsResult`, `mergeVisibleAndFlatten`, `reorderKeepsLayerSelected`, `arrangeAndReverseKeepTarget`, `backgroundFromLayerKeepsTarget`, `crossDocumentCopySelectsCopy`, and `refusedOperationLeavesTarget`; verify the names match `test-plan.md`
- [x] 5.2 Set targets inline before `record()` for add, duplicate, group, ungroup (including the survivor remap), delete, move, arrange, reverse, background from layer, merge, flatten, and the control-server bridge functions; verify each command writes the target before `record` and only when the operation changed the document
- [x] 5.3 Set targets for paste, place, type creation, shape creation, New Smart Object via Copy, cross-document copy, Alt-drag copy, and mode conversion; verify each bridge function has a target write before `record` and a matching plan row

## 6. Consumers read the Rust target

- [x] 6.1 Author `moveAutoSelectHiddenLayer`, `singleSelectionIsEditTarget`, `zeroOrMultiSelectionRefusesToolEdit`, and `multiSelectionKeepsStructuralCommands`; verify the names match `test-plan.md`
- [x] 6.2 Classify the Rust `active_layer` readers (including `duplicate_move_target` and test-only references) and the 33 non-test C++ `active_layer_path()` readers into edit target, anchor, or setter; migrate each to `single()`, `anchor()`, or `layer_selection_anchor()`; verify the review list contains every `rg "active_layer"` and `rg "active_layer_path\\(\\)"` hit
- [x] 6.3 Wire `ToolController::layerSelectionRequested` to `view->select_layer_path(path)` instead of `layersPanel_->selectPaths`, replace the `layersPanel_->currentPath()` / `selectedPaths()` reads in `frame_menus*.cpp` with the Rust target, and replace the post-operation / Select All / Similar / Linked / Deselect `selectPaths` calls with target writes, using `selectPaths` only to settle the panel projection; verify `rg "layersPanel_->(currentPath|selectedPaths)" crates/pictura-app/cpp/frame_menus*.cpp` returns nothing
- [x] 6.4 Keep the `layersPanel_` ownership guards where the panel may not exist during construction; verify every target-writing command still works headless with no panel

## 7. Integration and close-out

- [x] 7.1 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every scenario row is covered, every named test exists, and the change validates
- [ ] 7.2 Build verification is deferred by request: `cargo fmt --all` ran, but clippy, nextest, the C++ build, and the self-test were not run in this session
- [x] 7.3 Run `openspec archive layer-target-selection` and re-run `openspec validate --all --strict`; verify no non-archived change remains
