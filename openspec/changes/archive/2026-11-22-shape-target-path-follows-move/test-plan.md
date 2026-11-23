# Test plan: shape-target-path-follows-move

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `vm_target_path` | With a Work Path and the Move tool active, refresh the canvas. | The target path curve draws without anchors or handles. | `tst_view_menu::targetPath` |
| `vm_target_path_off` | Toggle Target Path off and on, then turn Extras off. | The curve hides, returns, and hides while the Show checkmark is unchanged. | `tst_view_menu::targetPath` |
| `vm_target_path_edit` | Edit the active path and select another saved path. | The persistent curve shows the new target and geometry. | `tst_view_menu::targetPath` |
| `vm_target_path_path_tool` | Activate the Pen or Path Selection tool with Target Path on. | Only the tool overlay draws; the base curve is absent. | `tst_view_menu::targetPath` |
| `vm_target_path_layer_move` | Create a shape layer, refresh so its outline is the target path, move it with the Move tool, and process the present. | The target path curve follows the moved outline instead of staying at the old position. | `tst_view_menu::shapeTargetPathFollowsMove` |
