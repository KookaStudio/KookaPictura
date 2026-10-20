# Test Plan

One row per scenario in the change's deltas. Engine rows run headless; UI rows
run in the Qt Test suite offscreen.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| An adjustment source merges down | Build a pixel lower layer and an Invert adjustment above it, run `merge_scope(Down)` on the adjustment path | One pixel layer with the effect baked in, no adjustment data, lower name/blend/opacity, one composite | `pictura-render::merge_down_bakes_an_adjustment_source`, `tst_merge_commands::adjustmentSourceMergesDown` |
| An adjustment above the bottom of a selection merges | Build top pixel, adjustment, bottom pixel, select all three, run `MergeScope::Selected` | One pixel layer at the topmost selected position with the adjustment applied to the selected layers below it | `pictura-render::merge_layers_bakes_an_adjustment_above_the_bottom`, `tst_merge_commands::adjustmentAboveBottomMergesLayers` |
| An adjustment target is still refused | Put a pixel above an adjustment and run Merge Down; put an adjustment at the bottom of a selection and run Merge Layers | Both refused, document unchanged, no history state | `pictura-render::merge_down_refuses_adjustment_targets`, `pictura-render::merge_layers_refuses_adjustment_bottom`, `tst_merge_commands::adjustmentTargetStillRefused` |
| Merge Visible selects the result over a hidden survivor | Three layers, hide the top one, select the bottom two, run Merge Visible | The hidden layer survives unselected; the merged pixel layer is the sole selection | `tst_merge_commands::mergeVisibleSelectsMergedResult` |
| Merge Down selects the result | Three layers, select the middle one, run Merge Down | The merged layer is the sole selection, not row 0 | `tst_merge_commands::mergeDownSelectsResult` |
| Merge Layers selects the result | Three layers, select the top and third rows, run Merge Layers | The merged layer is the sole selection, not the surviving middle layer | `tst_merge_commands::mergeLayersSelectsResult` |
| Ctrl+E follows an extended selection | Select only the Background, then extend the selection to another layer without opening the Layer menu | The merge action becomes enabled; `Ctrl+E` merges the selection and records one history state | `tst_merge_commands::shortcutFollowsSelection` |
