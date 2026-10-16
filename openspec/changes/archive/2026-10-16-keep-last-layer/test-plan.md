# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ll_last_layer_disabled` | Two layers, delete one; check the trash, row menu, Layer menu | Enabled with two layers; greyed out with one | `tst_last_layer::theLastLayerCannotBeDeleted` |
| `ll_last_layer_refused` | Delete every layer; delete the last layer; delete the only group, then its child | One layer remains; no state; the child may go | `tst_last_layer::theLastLayerCannotBeDeleted`, `pictura-render::delete_paths_keeps_the_last_layer` |
| `ll_remove_layer_guarded` | Two layers: `remove_layer(1)`, then `remove_layer(0)` and `remove_layer(5)` | First deletes as one state; the others return false with no state | `tst_last_layer::removeLayerKeepsTheLastLayer` |
| `ll_can_delete_agrees` | Five documents × six selections: compare availability with a delete on a copy | Equal in every case | `pictura-render::can_delete_paths_agrees_with_a_real_delete` |
