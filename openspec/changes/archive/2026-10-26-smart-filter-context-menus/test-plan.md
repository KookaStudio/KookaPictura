# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sfcm_group_menu` | Open the group row's menu on a layer with two filters | Disable Smart Filters, Delete Filter Mask, Clear Smart Filters | `tst_layers_smart_filters::smartFiltersGroupMenu` |
| `sfcm_mask_gate` | Open the group menu on a layer with no filter mask | Delete Filter Mask is disabled | `tst_layers_smart_filters::deleteFilterMaskFollowsTheMask` |
| `sfcm_group_actions` | Choose Disable Smart Filters, then Clear Smart Filters | The group is disabled, then every filter is removed | `tst_layers_smart_filters::menuTogglesAndClearsFilters` |
| `sfcm_filter_menu` | Open a filter row's menu | Edit Smart Filter Blending Options…, Edit Smart Filter…, Disable Smart Filter, Delete Smart Filter | `tst_layers_smart_filters::smartFilterRowMenu` |
| `sfcm_edit_gate` | Open the menu on a camera-raw filter and an unmodeled one | Edit Smart Filter… enabled then disabled | `tst_layers_smart_filters::editSmartFilterIsGatedOnKind` |
| `sfcm_edit` | Choose Edit Smart Filter… on a smart filter | The panel requests the filter's dialog for that path and index | `tst_layers_smart_filters::editSmartFilterRequestsTheDialog` |
| `sfcm_filter_actions` | Choose Disable Smart Filter, then Delete Smart Filter | The filter is disabled, then removed | `tst_layers_smart_filters::filterRowDisablesAndDeletes` |
| Blending options dialog | — | — | manual: the item ships disabled; the dialog is #961 |
