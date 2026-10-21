# Test Plan

Covers the `ui/layers-panel` delta. The scenarios are command and panel-state
checks, so they are automated with Qt Test against the real
`PicturaMainWindow`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lpd_nested` | New document, add a group and a child layer, expand the group, select the child, dispatch `LayerDuplicateLayer` | A `"<name> copy"` row sits directly above the source inside the same group, the row count grows by one, and the history grows by one | `tst_layer_commands::duplicateNestedLayer` |
| `lpd_multi` | New document with two named top-level layers, select both, dispatch `LayerDuplicateLayer` | Both sources gain an adjacent `"<name> copy"` row, the row count grows by two, and the history grows by one | `tst_layer_commands::duplicateMultiSelection` |
| `lpd_empty` | Clear the Layers panel selection, refresh the registry, dispatch `LayerDuplicateLayer` | The action is disabled, and the row and history counts are unchanged | `tst_layer_commands::duplicateWithoutSelection` |
