# Tasks

> Implementation status: the code for sections 1–6 is written. Their inline
> "verify …" clauses were not executed in this pass: the build and test run is
> deferred to 2.2 and 7.2–7.5, which stay open for it.

## 1. Test plan and test scaffold

- [x] 1.1 Write `test-plan.md` in this change folder mapping every scenario in the two spec deltas to an automated test or a justified manual check, and verify `scripts/check-test-plan.py` reports only the not-yet-written tests
- [x] 1.2 Move `duplicate_paths_returns_new_paths` out of
  `crates/pictura-render/src/document_ops/layer_ops/tests.rs` (1387 lines
  against the 1400-line test cap) into `tests_stack.rs`, adapting its local
  layer/group helpers, and write
  `duplicate_paths_reports_every_copy_at_its_final_path`,
  `duplicate_paths_shifts_a_nested_copy_with_its_ancestor` and
  `duplicate_paths_copies_a_source_a_later_copy_would_hide` there, covering
  the sibling, nested-ancestor and shifted-source cases, and verify they fail
  against the current engine
- [x] 1.3 Write `duplicate_target_names_every_copy`, `merge_target_selects_the_result`,
  `group_target_selects_the_group` and `ungroup_target_selects_the_children` in
  `crates/pictura-app/src/cxxqt_object/document_session/tests.rs` (it already
  builds the `PictureViewRust` fixture; `cxxqt_object/tests.rs` is near its
  cap), and verify they fail before the rules exist
- [x] 1.4 Write `document_scoped_state_is_cleared_with_the_document` in the same
  file, replacing the document and asserting the solo and the expansion are
  gone, and verify it fails before the state moves
- [x] 1.5 Write `refreshDoesNotRewriteTheTarget`, `currentRowFollowsTheTarget`,
  `selectSimilarKeepsActiveSelected`, `selectLinkedSelectsMembers` and
  `documentSwitchCurrentRow` in
  `crates/pictura-app/cpp/tests/tst_layer_target.cpp`; `currentRowFollowsTheTarget`
  also asserts the Properties panel follows the new current row, and verify the
  tests fail before the panel projects the target
- [x] 1.6 Extend `duplicateMultiSelection` in
  `crates/pictura-app/cpp/tests/tst_layer_commands.cpp` to assert
  `selected_layer_paths` names both copies and that the active path is one of
  them, and verify it fails before the fix
- [x] 1.7 Write `soloBelongsToItsDocument` and
  `expansionDoesNotCarryAcrossDocuments` in
  `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` (that file is under
  the 1400-line test cap; `tst_layers_panel.cpp` is at 1399 lines and has no
  room); extend `refreshKeepsCurrentRowOutsideSelection` to a multi-selection
  whose active path is not the first selected path, and verify they fail
  before the state and projection move

## 2. The engine reports the copies it made

- [x] 2.1 Fix `pictura_render::duplicate_paths` in `crates/pictura-render/src/document_ops/layer_ops/properties.rs`: resolve and clone every source before the first insert, insert the copies in panel order, and replay the inserts to report each copy at its final path, including a copy whose ancestor container shifted
- [x] 2.2 Move the golden in `duplicate_paths_returns_new_paths` (now in `tests_stack.rs`) to the corrected paths (`["3", "1"]` for its two-layer document) and record the old and the new values in the task result, and verify `cargo test -p pictura-render duplicate_paths` passes

## 3. The engine names the target of every operation

- [x] 3.1 Add `duplicate_target`, `merge_target`, `group_target` and `ungroup_target` to `PictureViewRust` in `crates/pictura-app/src/cxxqt_object/impl_layers/target_transitions.rs`, moving the ungroup rule's index arithmetic out of the edit closure into a function over the pre-operation group/survivor data and the post-operation document
- [x] 3.2 Make the selection-only rules update the history's belief after setting the target; do not update it inside an `edit` closure, because the edit captures the pre-operation target and then records the post-operation one, and verify `cargo test -p pictura-app history` passes
- [x] 3.3 Call the rules from the bridge methods: `duplicate_layers` (`impl_layers.rs`), the merge family (`impl_layers_merge.rs`), `group_layers` and `ungroup_layers` (`target_transitions.rs`), and verify the app crate's target-rule tests pass
- [x] 3.4 Give the adjustment layer a target in `add_adjustment` (`impl_filters.rs`), give Select Similar and Select Linked theirs in `impl_layers_select.rs` so both menus share them (the render matcher keeps excluding the reference; the app rule adds it back, and each selection rule notifies the view so the panel projects it), leave an unlinked Select Linked target unchanged, and verify `tst_adjustments_panel::clickAdjustmentCreatesAndSelectsLayer` and the new select tests pass

## 4. The panel's per-document state moves into Rust

- [x] 4.1 Add the solo snapshot and the expanded group paths to `PictureViewRust` (`cxxqt_object/state.rs`), clear them with the rest of the document-scoped state (`document_session/session.rs`), drop the solo snapshot on every `Edit::Structural` commit and explicitly in `add_adjustment` (which records a recomposite, not a structural edit), and verify `document_scoped_state_is_cleared_with_the_document` passes
- [x] 4.2 Add the bridge functions `solo_set`, `solo_restore`, `solo_clear`, `solo_active`, `expansion_set` and `expansion_paths`, computing the soloed paths (the layer, its ancestors and a group's visible descendants) and the visibility snapshot in the engine; `solo_set` and `solo_restore` each apply one labelled visibility edit, and verify the bridge compiles and the app crate's tests pass
- [x] 4.3 Replace the panel's `soloActive_`, `soloPath_` and `soloSnapshot_` members with the bridge calls, keeping the solo's visibility, its undo step and the self-test's exit code 106 intact; a document switch ends the outgoing document's mask solo view (`ui/mask-solo-view`) but does not clear the layer solo snapshot, and `isolatedPaths_` stays the transient Selected-filter set
- [x] 4.4 Replace the panel's `expandedPaths_` member with the bridge, keeping the name-chain re-find in the panel and writing the remapped set back after each refresh, and verify `addingLayerKeepsGroupExpanded`, `renameKeepsGroupExpanded`, `duplicateGroupNamesStayDistinct` and `dropIntoOpenGroupKeepsExpanded` still pass

## 5. The panel projects the target

- [x] 5.1 Add the remembered target to `LayersPanel` (the selected paths and the active path it last agreed with), update it when the panel writes the target and after a refresh projects it, clear it on a view or document-generation change, and keep the target's active path when the current row moves outside the selection instead of resetting it to the first selected path, and verify the new document-switch case
- [x] 5.2 In `LayersPanel::refresh()`, compare the live target with the remembered one before updating it: when the target moved, move the current row onto the target's active path; when it did not, keep the previous current row. The `wasEditing` restoration follows the same moved check, and verify `currentRowFollowsTheTarget`, `documentSwitchCurrentRow` and `refreshKeepsCurrentRowOutsideSelection` all pass
- [x] 5.3 Move the projector — the target-moved rule, the remembered target and the expansion's read-back — into a new `crates/pictura-app/cpp/panels/layers_panel_target.cpp`, add it to `CMakeLists.txt` explicitly, and verify the file-size guard passes and the panel's suites still build

## 6. The commands and the menus stop choosing

- [x] 6.1 Delete the post-op selection in `LayersPanel::duplicateSelection` (`layers_panel_actions.cpp`) and in the panel's row and panel menus (`layers_panel_menu.cpp`, including `convertToSmartObject` and `newSmartObjectViaCopy`), and verify `duplicateMultiSelection` and `duplicateNestedLayer` pass
- [x] 6.2 Delete the post-op selection after Merge Layers, Merge Visible, Select Similar and Select Linked in `frame_menus.cpp`, the Select Similar target write in `frame_menus_select.cpp`, and after a smart-object copy in `frame_menus_smart_object.cpp`; extend `newViaCopyAddsIndependentLayer` to assert the copy is the target, and verify the merge, select and smart-object suites pass
- [x] 6.3 Make `PicturaMainWindow::selectLayerPath` write the target once through `select_layer_path` and refresh the panel, and drop the `setView` and the `selectPaths` calls it makes on every selection
- [x] 6.4 Read the target in the Duplicate enabled-provider (`frame_menus.cpp`); the Align and Distribute providers already read it, and verify the commands are enabled from the target
- [x] 6.5 Delete the adjustment fill icon's row scan and its `selectPaths` in `frame_build.cpp`, and verify the Adjustments panel still selects the layer it creates
- [x] 6.6 Delete the post-op selection after a background conversion in `layers_panel_events.cpp` and in `LayersPanel::openBackgroundConversion`, and verify the background suites pass

## 7. Integration verification

- [x] 7.1 Run `openspec validate --all --strict` and verify the change and the corpus validate
- [x] 7.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo nextest run --workspace`, and verify they pass
- [x] 7.3 Build once and run `ctest --test-dir build -R '^tst_' --output-on-failure` and the app self-test, verifying the panel, target, merge and command suites pass and the self-test's summary reports no failure
- [x] 7.4 Run `bash scripts/verify-fast.sh` and verify it passes, including the file-size and test-plan guards
- [x] 7.5 Archive the change with `openspec archive rust-decides-layer-target` so the pull request leaves no open change, and verify `scripts/check-openspec-archived.sh` passes
