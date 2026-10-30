# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ce_filter_params` | `filter` gaussian-blur `[2]` | ok; one more state | `tst_control_editing::filterTakesParameters` |
| `ce_filter_invalid` | `[]`, `[99999]`, on gaussian-blur | Both `invalid_param` | `tst_control_editing::badFilterParametersAreRefused` |
| `ce_filter_list` | `list_filters` | Ranges present; displace unavailable | `tst_control_editing::filtersAreListed` |
| `ce_adjust_apply` | `adjust_apply` posterize on a gradient-free doc | ok; one more state | `tst_control_editing::adjustmentApplies` |
| `ce_adjust_invalid` | Unknown kind | `invalid_param` | `tst_control_editing::unknownAdjustmentIsRefused` |
| `ce_adjust_list` | `list_adjustments` | Every page non-empty | `tst_control_editing::adjustmentsAreListed` |
| `ce_run_size` | `image.image.size` 20×10 | Size applied | `tst_control_editing::resizeRunsWithoutADialog` |
| `ce_run_unlisted` | `file.save` | `refused` | `tst_control_editing::unlistedCommandIsRefused` |
| `ce_run_key` | `in` key | `invalid_param` | `tst_control_editing::unlistedKeyIsRefused` |
| `ce_run_budget` | 30000×30000 | `invalid_param`; size unchanged | `tst_control_editing::oversizedResizeIsRefused` |
| `ce_layer_tree` | `add_group`; `layer_tree`; `list_layers` | Rows with depth; `active` | `tst_control_editing::layerTreeReportsPaths` |
| `ce_layer_active` | Valid, invalid path | Active set; `invalid_param` | `tst_control_editing::activeLayerIsSetByPath` |
| `ce_export` | Dirty doc; `export_png` | File; state unchanged | `tst_control_editing::exportLeavesTheDocumentAlone` |
| `ce_export_gate` | No Save grant; socket `export_png` | `refused`; no file | `tst_control_editing::exportIsGatedLikeASave` |
| `ce_history` | List; jump back | Labels; earlier state | `tst_control_editing::historyListsAndJumps` |
| `ce_stale_history` | Undo with old index | `stale_history` | `tst_control_editing::staleUndoIsRefused` |
| `ce_stale_document` | Socket `filter` with another `doc` | `stale_document` | `tst_control_editing::staleDocumentIsRefused` |
| `ce_one_state` | Table of mutators | Each adds one state | `tst_control_editing::eachMutatorAddsOneState` |
| `ce_lasso` | `lasso` 3 points; 4097 points | Selection; `invalid_param` | `tst_control_editing::lassoIsOneCall` |
| `ce_crop_person` | Pending crop not started by the session; live `commit_crop` | `human_busy` | manual: a pending crop needs real canvas drags, and the offscreen fixture has no stable route to one |
