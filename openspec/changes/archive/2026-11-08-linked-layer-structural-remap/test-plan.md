# Test plan

Every scenario in the delta specs has a row. The remap units cover the path
arithmetic; the Qt rows drive the real panel commands and canvas.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Select Similar matches the active layer | Run Select Similar on a pixel layer | Same-kind layers are selected, no pixels change | manual: unchanged by this change; covered by the existing Select Similar suite |
| Link and Select Linked | Link two layers and run Select Linked Layers | Both are selected | manual: unchanged by this change; covered by `tst_linked_layers::stripLinkButtonToggles` |
| Unlink removes membership | Unlink a linked layer | It leaves the set | manual: unchanged by this change; covered by `tst_linked_layers::unlinkClearsTheSet` |
| Unlinking a member clears a two-layer set | Unlink one of a linked pair | Neither row shows a link | manual: unchanged by this change; covered by `tst_linked_layers::unlinkClearsTheSet` |
| Link sets are session-only | Save and reopen linked layers | No links are persisted | manual: persistence is unchanged and has no save/reopen link fixture |
| `llm_link_remap` | Insert a layer above a linked pair, then delete a neighbour | The set follows the shifted paths and stays linked | `tst_linked_layers::linkSurvivesAddingAndDeletingLayers`, `pictura-app::an_insertion_above_shifts_the_linked_paths`, `pictura-app::a_deletion_shifts_the_paths_below_it`, `pictura-app::deleting_a_linked_member_drops_the_set` |
| `llm_link_structural_clear` | Reorder, group, ungroup, merge, or rasterize a linked layer | The link set is cleared | manual: the clearing rule is unchanged and each operation has its own existing suite |
| `lmtc_box_shown` | Check Show Transform Controls on a raster layer | The box hugs the content, no pivot, no session | manual: unchanged by this change; covered by `tst_transform_controls::checkingTheBoxShowsControls` |
| `lmtc_box_hidden` | Uncheck the box while shown | The overlay ends | manual: unchanged by this change; covered by `tst_transform_controls` |
| `lmtc_box_follows` | Select another layer or change its content | The box rebuilds | manual: unchanged by this change; covered by `tst_transform_controls::boxFollowsTheActiveLayer` |
| `lmtc_box_linked` | Select one of two linked layers | The box spans the union and follows link/unlink | `tst_linked_layers::linkedControlsBoxResizesTheSet` |
| `lmtc_box_refused` | Select a Background, group, adjustment, or locked layer | No box | manual: unchanged by this change; covered by `tst_transform_controls` |
| `lmtc_box_selection` | Activate a selection | The box hides | manual: unchanged by this change; covered by `tst_transform_controls` |
| `lmtc_box_no_history` | Show and hide the box | No history state | manual: unchanged by this change; covered by `tst_transform_controls` |
| `lmtc_box_idle` | Show the box on a canvas-sized layer with a small square | The box hugs the square, the layer keeps its rect | manual: unchanged by this change; covered by `tst_transform_controls::idleBoxLeavesTheLayerAlone` |
| `lmtc_handle_drag` | Drag a corner handle | Scale on release, one state, box remains | manual: unchanged by this change; covered by `tst_transform_controls::handleDragScalesAndCommits` |
| `lmtc_handle_drag_linked` | Drag a handle on a linked set | Both members scale about the anchor, one state, links survive | `tst_linked_layers::linkedControlsBoxResizesTheSet` |
| `lmtc_identity` | Press and release a handle without moving | No history state | manual: unchanged by this change; covered by `tst_transform_controls` |
| `lmtc_ctrl_t` | Press Ctrl+T while the box is shown | The modal session begins | manual: unchanged by this change; covered by `tst_transform_controls::ctrlTTakesTheBoxOver` |
