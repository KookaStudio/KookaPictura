# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lt_single` | Click one layer in the panel | That path is the only selected path and the active path | `tst_layer_target::panelSelectionUpdatesTarget` |
| `lt_multi` | Ctrl/Cmd-click two layers with a mask edit target on the first | Both paths selected, last-clicked is active, mask target cleared | `tst_layer_target::multiSelectionKeepsAnchor` |
| `lt_deselect` | Run Select > Deselect Layers | Selected set and active path are both empty | `tst_layer_target::deselectClearsTarget` |
| `lt_stale` | Select one layer, delete it, read the target | The removed path is gone and the anchor is empty | `pictura-app::stale_paths_are_dropped` |
| `lt_stale_anchor` | Select two layers, delete the active one | The other selected path becomes active | `pictura-app::stale_anchor_coerces_to_a_survivor` |
| `lt_no_history` | Select, extend, and clear the layer selection | Document stays clean and history depth is unchanged | `tst_layer_target::selectionDoesNotDirtyOrRecord` |
| `lt_autoselect_hidden` | Filter out a layer, then Move Auto-Select on its pixels | That hidden layer becomes the target and Move acts on it | `tst_layer_target::moveAutoSelectHiddenLayer` |
| `lt_edit_single` | Select one layer, read the edit target | The layer is the edit target | `pictura-app::single_only_when_one` |
| `lt_edit_multi` | Select two layers with one active, read the edit target | No single edit target; both paths remain selected | `tst_layer_target::multiSelectionHasNoEditTarget` |
| `lt_op_create` | Create a layer/group, paste, place, create type/shape, make a smart object via copy | Each created node is selected and active | `tst_layer_target::createdNodesBecomeTarget`, `tst_layer_target::pasteBecomesTargetAndUndo`; manual: place/type/shape/smart-object creation need external raster or text fixtures |
| `lt_op_duplicate` | Duplicate a layer; repeat with Alt-drag | The copy is selected and active each time | `tst_layer_target::duplicateBecomesTarget`; manual: Alt-drag needs a live pointer gesture |
| `lt_op_delete` | Delete the selected middle layer | The deterministic nearest survivor is selected | `tst_layer_target::deleteSelectsSurvivor` |
| `lt_op_group` | Group two layers with a pixel outside the group selected, then ungroup the group | The group/children are selected; the outside pixel remains selected | `tst_layer_target::groupAndUngroupSelectResult` |
| `lt_op_merge` | Merge Down, Merge Layers, Merge Visible, Flatten | Each result is the only selected layer | `tst_layer_target::mergeSelectsResult`, `tst_layer_target::mergeVisibleAndFlatten`; manual: Merge Down goes through the context-sensitive menu command |
| `lt_op_reorder` | Move, arrange, and reverse the selected layer | The same layer stays selected at its new path | `tst_layer_target::reorderKeepsLayerSelected`, `tst_layer_target::arrangeAndReverseKeepTarget` |
| `lt_op_background` | Run Layer from Background on the selected layer | The converted layer is selected at its new bottom path | `tst_layer_target::backgroundFromLayerKeepsTarget` |
| `lt_op_cross_document` | Drag a layer into another open document | The copy is selected and active in the destination | `tst_layer_target::crossDocumentCopySelectsCopy` |
| `lt_op_refused` | Delete the Background or a fully locked layer | Target unchanged | `tst_layer_target::refusedOperationLeavesTarget` |
| `lt_undo_structural` | Duplicate, delete, or group, then undo without changing selection | Target names the restored layer, not the old path occupant | `tst_layer_target::undoRestoresStructuralTarget`, `pictura-app::undo_restores_structural_target` |
| `lt_undo_pixel` | Paint, select another layer, undo the paint | The later selection stays selected | `tst_layer_target::undoKeepsLaterSelection`, `pictura-app::undo_keeps_a_later_selection` |
| `lt_undo_first_edit` | Open a document, paint once, undo | The opening target is still selected | `tst_layer_target::undoFirstEditAfterOpen` |
| `lt_redo` | Undo a structural operation, then redo it | Target follows the operation's result again | `tst_layer_target::redoRestoresPostOperationTarget`, `pictura-app::redo_restores_post_operation_target` |
| `lt_undo_named_snapshot` | Restore a named history snapshot | The target captured with the snapshot is restored | `tst_layer_target::namedSnapshotRestoresTarget`, `pictura-app::named_snapshot_restores_layer_target` |
| `lt_open` | Open a PSD whose topmost pixel layer sits below a group or adjustment | That pixel layer is selected and active | `pictura-app::open_target_prefers_the_topmost_pixel_layer`, `pictura-app::open_target_finds_a_pixel_inside_the_top_group` |
| `lt_open_no_pixel` | Open a document containing only groups or adjustments | The topmost top-level layer is selected and active | `pictura-app::open_target_falls_back_to_the_top_layer` |
| `lt_switch` | Switch between two open documents and back | Each document shows its own previous target | `tst_layer_target::documentSwitchRestoresTarget` |
| `lt_new` | Create a new document | Its initial layer is selected and active | `tst_layer_target::newDocumentSelectsFirstLayer` |
| `lpr_bg_active` | Open a one-layer Background document and refresh | Background row selected and active | `tst_layers_panel::backgroundOnlyStaysActive` |
| `lpr_active_restore` | Refresh with the target off the first displayed row | Target row selected, not the first row | `tst_layers_panel::activeLayerNotFirstRow` |
| `lpr_active_deselect` | Deselect layers, then refresh | Panel keeps no selected row | `tst_layer_target::deselectSurvivesRefresh` |
| `lpr_filter_target` | Filter out the target row, then clear the filter | Target retained while hidden; row selected again after clearing | `tst_layer_target::filterHidingTargetKeepsIt` |
| `lpr_filter_command` | Filter out a selected row, then run a selection-based layer command | The command acts on that hidden selected layer | `tst_layer_target::filteredTargetCommands` |
| `lpr_multi_visual` | Select several layers | Every visible selected row is shown selected; active row is current | `tst_layer_target::multiSelectionIsShownInFull` |
| `ltf_active_layer` | Run a tool edit with one layer selected | Exactly that layer changes and the other layers are unchanged | `tst_layer_target::singleSelectionIsEditTarget` |
| `ltf_active_layer_push` | Change the panel selection, then run a tool edit | The edit targets the newly selected layer | `tst_layer_target::panelSelectionUpdatesTarget` |
| `ltf_active_layer_refuse` | Run a paint/filter edit with zero and with two layers selected | Refused both times with no history state | `tst_layer_target::zeroOrMultiSelectionRefusesToolEdit` |
| `ltf_multi_commands` | Select two layers, run Group/Merge/Delete and change a property | Structural commands act on both; paint is refused | `tst_layer_target::multiSelectionKeepsStructuralCommands` |
