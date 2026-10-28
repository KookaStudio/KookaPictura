# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lp_sf_drag` | Drag the bottom filter row above the top one; Undo; drag a filter row onto a layer row | The dragged filter is the top row and its effect shows over the other; Undo restores; the layer drop is refused | `tst_smart_filter_apply::dragReordersSmartFilters` |
| The parent and one child per filter appear | Open a smart object with a filter chain | Smart Filters parent and one row per filter | `tst_layers_smart_filters::treeShowsGroupAndToggles` |
| `lp_sf_order` | Apply Gaussian Blur, then Emboss | Emboss is the top row | `tst_smart_filter_apply::filterRowsShowNewestFirst` |
| Toggling a filter eye updates the composite | Flip a filter eye | Composite updates; one state | `pictura-app::app_setter_changes_the_composite_and_records_one_state` |
| Toggling the group eye bypasses the chain | Flip the group eye off | The unfiltered source shows | `pictura-app::disabled_group_bypasses_the_chain` |
| A toggle survives a write and re-read | Toggle, save, reopen | The flag survives | `pictura-app::toggles_persist_on_a_preserved_smart_object` |
| Synthetic rows are not real layers | Double-click a filter row | No rename, no layer change; the dialog opens | `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` |
| Dragging by hand | In `./build/pictura`, drag a filter row up and down past the others | The drop indicator only appears inside the filter list; the canvas updates after each drop | manual: the visible drop indicator and mouse-driven drag are not observable offscreen |
