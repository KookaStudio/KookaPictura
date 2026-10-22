# Test Plan — rust-decides-layer-target

One row per scenario in this change's `specs/` deltas. The scenario cell is the
scenario's tag. Coverage names the automated test that checks it: a Qt Test case
(`tst_<suite>::<case>`) or a Rust unit test (`<crate>::<fn>`). A row reads
`manual:` only when the check has no such owner — the app self-test is one, so
the rows it covers say so.

## `specs/document/layer-target/spec.md`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lt_ui_projection` | Select two layers and run `Layer ▸ Duplicate Layer…` | Both copies are selected, one of them is active, and the panel shows that selection — the panel projects the engine's target rather than choosing one | `tst_layer_commands::duplicateMultiSelection`, `pictura-app::duplicate_target_names_every_copy` |
| `lt_ui_no_rewrite` | Run a structural command, then refresh the panel | The target is the one the engine set; the refresh does not capture or overwrite it | `tst_layer_target::refreshDoesNotRewriteTheTarget` |
| `lt_ui_current_row` | Run a command that moves the target's active path to another layer that is displayed | The panel's current row is the new active path | `tst_layer_target::currentRowFollowsTheTarget` |
| `lt_ui_stray_row` | Move the panel's current row outside a multi-selection whose active path is not the first selected path, then refresh while the row is displayed | The current row stays on the row the user put it on and the target is unchanged | `tst_layers_panel_fixes::refreshKeepsCurrentRowOutsideSelection` |
| `lt_op_target_copies` | Duplicate two layers and read the target | The target names both copies at their final paths, and the active path is one of them | `pictura-render::duplicate_paths_returns_new_paths`, `pictura-render::duplicate_paths_reports_every_copy_at_its_final_path`, `pictura-app::duplicate_target_names_every_copy` |
| `lt_op_target_engine` | Run a duplicate, a merge, a merge of the visible layers, a group and an ungroup, and read the target each time | Each leaves the target its own rule names, with no user interface involved | `pictura-app::duplicate_target_names_every_copy`, `pictura-app::merge_target_selects_the_result`, `pictura-app::group_target_selects_the_group`, `pictura-app::ungroup_target_selects_the_children` |
| `lt_op_duplicate_nested` | Duplicate layers in different containers, including a source whose ancestor shifts | Every copy is reported at its final path and the target names every copy | `pictura-render::duplicate_paths_shifts_a_nested_copy_with_its_ancestor`, `pictura-render::duplicate_paths_copies_a_source_a_later_copy_would_hide`, `pictura-app::duplicate_target_names_every_copy` |
| `lt_op_create` | Create a layer, a group, a type layer, a shape, paste, or make a smart object via copy | The created node is selected and active | `tst_layer_target::createdNodesBecomeTarget`, `tst_layer_target::pasteBecomesTargetAndUndo`, `tst_type_tools::horizontalTypeTool`, `tst_smart_object_actions::newViaCopyAddsIndependentLayer` |
| `lt_op_duplicate` | Duplicate one layer | The copy is selected and active | `tst_layer_target::duplicateBecomesTarget` |
| `lt_op_duplicate_many` | Duplicate two selected layers | Every copy is selected, one of them is active, and no layer that was not copied is selected | `tst_layer_commands::duplicateMultiSelection`, `pictura-app::duplicate_target_names_every_copy` |
| `lt_op_adjustment` | Create an adjustment from the Adjustments panel icon grid, the panel menu and the Layer menu | The created layer is the only selected layer and is active, and the Properties panel shows its controls | `tst_adjustments_panel::clickAdjustmentCreatesAndSelectsLayer` |
| `lt_op_select_similar` | Run Select Similar on a layer | The active layer and every matching layer are selected, with the original layer active | `tst_layer_target::selectSimilarKeepsActiveSelected` |
| `lt_op_select_linked` | Run Select Linked Layers on a linked layer | Every member of its link set is selected, with the original layer active | `tst_layer_target::selectLinkedSelectsMembers` |
| `lt_op_select_linked_unlinked` | Run Select Linked Layers on a layer with no link set | The target is unchanged | `tst_layer_target::selectLinkedSelectsMembers` |
| `lt_op_delete` | Delete the selected layer | The nearest surviving layer is selected and active | `tst_layer_target::deleteSelectsSurvivor` |
| `lt_op_group` | Group a selection, then ungroup it | The new group, then the spliced-out children, are selected with one of them active, and a selected layer outside the group stays selected | `tst_layer_target::groupAndUngroupSelectResult`, `pictura-app::group_target_selects_the_group`, `pictura-app::ungroup_target_selects_the_children` |
| `lt_op_merge` | Merge Down, Merge Layers, Merge Visible and Flatten Image | The resulting layer is the only selected layer and is active | `tst_layer_target::mergeSelectsResult`, `tst_layer_target::mergeVisibleAndFlatten`, `pictura-app::merge_target_selects_the_result` |
| `lt_op_reorder` | Move the selected layer up, down, arrange and reverse it | The same layer remains the target at its new path | `tst_layer_target::reorderKeepsLayerSelected`, `tst_layer_target::arrangeAndReverseKeepTarget` |
| `lt_op_background` | Convert a layer with `Layer from Background` | The converted layer remains selected and active at its new path | `tst_layer_target::backgroundFromLayerKeepsTarget` |
| `lt_op_cross_document` | Drag a layer from another open document into this one | The copy is selected and active in the destination document | `tst_layer_target::crossDocumentCopySelectsCopy` |
| `lt_op_refused` | Run an operation the document refuses | The selected set and the active path are unchanged | `tst_layer_target::refusedOperationLeavesTarget` |

## `specs/ui/layers-panel/spec.md`

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lpr_bg_active` | Open a document whose only layer is the locked Background | The Background row is the selected and active row | `tst_layers_panel::backgroundOnlyStaysActive` |
| `lpr_active_restore` | Refresh a document whose target is not the first displayed row | The panel selects the target's path rather than the first displayed row | `tst_layers_panel::activeLayerNotFirstRow` |
| `lpr_active_follows` | Run an operation that moves the document target's active path to a displayed row | The panel's current row is the new active path | `tst_layer_target::currentRowFollowsTheTarget` |
| `lpr_active_stray` | Move the current row outside a multi-selection whose active path is not the first selected path, then refresh while it is displayed | The current row stays where the user put it and the active path is unchanged | `tst_layers_panel_fixes::refreshKeepsCurrentRowOutsideSelection` |
| `lpr_switch_current` | Switch to another open document | The panel shows that document's target and current row | `tst_layer_target::documentSwitchCurrentRow` |
| `lpr_active_deselect` | Clear the layer selection, then refresh | The panel has no selected row | `tst_layer_target::deselectSurvivesRefresh`, `tst_layer_target::emptySelectPathsClearsTarget` |
| `lpr_filter_target` | Hide the target's row behind a filter, then move the current row onto a displayed row | The document still reports the layer as selected and active, the current-row move leaves the target alone, and clearing the filter selects its row again | `tst_layer_target::filterHidingTargetKeepsIt` |
| `lpr_filter_command` | Run a selection-based command while the target's row is hidden | The command acts on the hidden layer | `tst_layer_target::filteredTargetCommands` |
| `lpr_multi_visual` | Select several layers | Every selected row is shown as selected and the active path's row is the current row | `tst_layer_target::multiSelectionIsShownInFull` |
| `lpd_nested` | Duplicate a layer nested inside a group | A copy appears directly above the source inside the same group, and exactly one history state is added | `tst_layer_commands::duplicateNestedLayer` |
| `lpd_multi` | Duplicate two selected layers | Both gain a copy directly above themselves, exactly one history state is added, and both copies are selected with one of them active | `tst_layer_commands::duplicateMultiSelection` |
| `lpd_empty` | Clear the target and look at `Layer ▸ Duplicate Layer…` | The command is disabled, and running it leaves the rows and the history unchanged | `tst_layer_commands::duplicateWithoutSelection` |
| `m39_solo` | Alt-click solos a layer: `Alt`-click a layer's eye in a document with several visible layers | Only that layer and its ancestor groups are visible, one undo state is added, and the prior visibility is remembered | manual: checked by the app self-test token `solo_solo` (106); the self-test is not a test-plan reference type |
| `m39_solo` | A second Alt-click restores exactly: `Alt`-click again while solo is active | Every row's prior visibility is restored exactly, including layers hidden before the first `Alt`-click | manual: checked by the app self-test token `solo_solo` (106); the self-test is not a test-plan reference type |
| `m39_solo` | Solo survives undo via its history state: undo the solo operation | The prior visibility of every layer is restored | manual: checked by the app self-test token `solo_solo` (106); the self-test is not a test-plan reference type |
| `m39_solo_document` | Solo a layer, switch to another document, and switch back | The first document is still soloed, and a second `Alt`-click restores its prior visibility | `tst_layers_panel_fixes::soloBelongsToItsDocument`, `pictura-app::document_scoped_state_is_cleared_with_the_document` |
| `lpr_add_keeps_expanded` | Expand a group, then add a layer elsewhere so the group's path shifts | The group stays expanded with its children visible | `tst_layers_panel_fixes::addingLayerKeepsGroupExpanded` |
| `lpr_rename_keeps_expanded` | Rename an expanded group in place | It stays expanded, because the rename leaves its index path intact | `tst_layers_panel_fixes::renameKeepsGroupExpanded` |
| `lpr_dup_group_names` | Expand one of two same-named sibling groups, then shift their paths | The group that was expanded is the one that stays expanded | `tst_layers_panel_fixes::duplicateGroupNamesStayDistinct` |
| `lpr_drop_into_expanded` | Drop a layer onto an open group | The move is one undo step and the group stays expanded with the dropped layer visible | `tst_layers_panel_fixes::dropIntoOpenGroupKeepsExpanded` |
| `lpr_expansion_document` | Expand a group, switch to another document, and switch back | The other document's groups are not expanded, and switching back restores the first document's expanded group | `tst_layers_panel_fixes::expansionDoesNotCarryAcrossDocuments`, `pictura-app::document_scoped_state_is_cleared_with_the_document` |

## Notes

- The `duplicate_paths_returns_new_paths` golden moves: the engine's copies
  were reported at the paths they had when they were inserted, so only the last
  copy's path was right. The two-top-level-layer fixture moves from
  `["2", "1"]` to `["3", "1"]`. The nested case moves the child copy from
  `"1/1"` to `"2/1"` while its group copy stays `"1"`; the shifted-source case
  grows from `["2"]` to `["3/1", "2"]`. The task result records the old and the
  new values.
- The `m39_solo` rows stay with the app self-test, whose tokens the
  test-plan checker cannot name. The self-test's solo case keeps its exit code
  106 and its three assertions.
