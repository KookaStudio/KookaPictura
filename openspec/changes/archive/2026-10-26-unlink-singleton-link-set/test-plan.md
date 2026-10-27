# Test plan

Every scenario in the delta spec has a row. The four carried-over scenarios are
covered by existing tests; the new behaviour gets `tst_linked_layers`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Select Similar matches the active layer | Run Select Similar on a pixel layer | Same-kind layers are selected, no pixels change | `tst_layer_target::selectSimilarKeepsActiveSelected` |
| Link and Select Linked | Link two layers, run Select Linked Layers on one | Both members are selected | `tst_layer_target::selectLinkedSelectsMembers` |
| Unlink removes membership | Unlink a linked layer | That layer is no longer linked | `tst_linked_layers::stripLinkButtonToggles` |
| Unlinking a member clears a two-layer set | Link two layers, unlink one, read both rows and `select_linked` | Neither row is linked and both `select_linked` calls are empty | `tst_linked_layers::unlinkClearsTheSet` |
| Link sets are session-only | Run a structural layer operation on linked layers | The link sets are cleared | `tst_layers_panel_bundle::anchoredInsertionClearsLinks` |
