# Test Plan

One row per scenario in the change's `compositing/layer-management` delta.
Engine rows run headless; UI rows run in the Qt Test suite offscreen.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Merge Down composites two layers | Build a masked upper layer over a raster lower layer and call `merge_scope(Down)` | One layer whose pixels and alpha match the compositor oracle, including the mask and the union of nontransparent areas | `pictura-render::merge_down_replaces_pair_and_inherits_lower`, `pictura-render::merge_down_applies_masks_into_alpha` |
| Merge Layers replaces the selection at the topmost position | Select two or more layers and call `merge_scope(Selected)` | One layer at the topmost selected position, composite pixels, Normal/255 | `pictura-render::merge_layers_lands_topmost_and_resets_blend` |
| The merge label follows a single selection | Select one layer, refresh the command registry, read the `layer.merge.layers` action text | Action text is `Merge Down` | `tst_merge_commands::layerMenuContextSensitive` |
| The merge label follows a multiple selection | Select two layers, refresh, read the action text and the Layer menu entries | Action text is `Merge Layers`; no separate `Merge Down` action exists | `tst_merge_commands::layerMenuContextSensitive` |
| No separate Merge Down item remains | With one layer selected, inspect the command table and the Layer menu | Exactly one merge action exists; `layer.merge.down` has no action or handler | `tst_merge_commands::layerMenuContextSensitive` |
| The merge command is disabled with no selection | Clear the panel selection, refresh, read the action | Disabled and labelled `Merge Layers` | `tst_merge_commands::layerMenuContextSensitive` |
| Ctrl+E merges down with one selected layer | Select one layer, assert `Ctrl+E` maps to `layer.merge.layers`, dispatch it | Row count drops by one, history grows by exactly one, undo restores the pair | `tst_merge_commands::layerMenuContextSensitive` |
| Ctrl+E merges only the selected layers | Select two layers above a third, assert `Ctrl+E` maps to `layer.merge.layers`, dispatch it | Only the two selected layers collapse; the third survives; history grows by exactly one | `tst_merge_commands::layerMenuContextSensitive` |
| A multi-selection cannot run Merge Down | With two layers selected, check for `layer.merge.down` and dispatch the retired id | No action/handler; dispatch returns false and history is unchanged | `tst_merge_commands::layerMenuContextSensitive` |
| The Layers panel merge label follows the selection | Open the live Layers panel header menu with one selection, then with two selections | Entry reads `Merge Down` for one and `Merge Layers` for two, never both labels at once | `tst_merge_commands::layersPanelContextSensitiveLabel` |
| The Layers panel merge entry is disabled when the merge cannot run | Open the live Layers panel header menu with only the bottom layer selected | Entry is disabled and reads `Merge Down`; with a valid pair it is enabled | `tst_merge_commands::layersPanelContextSensitiveLabel` |
| An adjustment target is refused | Run Merge Down over an adjustment target and Merge Layers with an adjustment bottom in the engine tests; dispatch a no-layer-below refusal through the Qt bridge | Both engine refusals leave the document unchanged; the bridge refusal records no history state | `pictura-render::merge_down_refuses_adjustment_targets`, `pictura-render::merge_layers_refuses_adjustment_bottom`, `tst_merge_commands::layerMenuContextSensitive` |
| Merge Down needs a layer below | Select the bottom layer only and run Merge Down | Refused; the document and history are unchanged | `pictura-render::merge_down_refuses_without_a_layer_below`, `tst_merge_commands::layerMenuContextSensitive` |
| A merge is one undo step | After each merge in the Qt test, call undo and compare the recovered tree, kinds, names, masks, and sampled pixels | The exact pre-merge state is restored in one undo step | `tst_merge_commands::layerMenuContextSensitive` |

No manual-only rows.
