# Test Plan

The panel's current row is a projection of the document's layer target. The
scenario is that the user empties the target and the row highlight goes with it.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lt_deselect_current` | Select a row, then clear the selection through the tree's selection model (the Ctrl-click gesture) | The target and the tree's current index are both empty; no row is highlighted | `tst_layer_target::userDeselectClearsCurrentRow` |
| `lt_deselect_current` | Select a row, then click the empty space below the rows | The target and the tree's current index are both empty; no row is highlighted | `tst_layer_target::clickingEmptySpaceClearsCurrentRow` |
