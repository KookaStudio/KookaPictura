# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sfa_undo` | Apply a filter to a Smart Object, Edit > Undo | No Smart Filter | `tst_smart_filter_apply::undoRemovesSmartFilter` |
| `sfa_reedit_kind` | Right-click a generic, a gallery, and a Blur Smart Filter | Edit Smart Filter… is enabled on the first two only | `pictura-app::edit_kind_reports_generic_and_gallery`, `tst_smart_filter_apply::editSmartFilterEnabledForGenericEntry` |
| `sfa_reedit_preview` | Double-click the lower of two filters, drag a value, Cancel | The upper filter disappears during the edit; Cancel restores both | `pictura-app::edit_preview_hides_filters_above` |
| `sfa_reedit_commit` | Change the lower filter's value and OK | Still two rows; the lower has the new value; no mask appears | `pictura-app::edit_commit_replaces_in_place` |
| `sfa_reedit_dialog` | Double-click a filter under another, tick Don't show again, OK | Notice, then the prefilled dialog; the notice does not come back | `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` |
| `sfcm_filter_menu` | Right-click a filter row | The four items in order | `tst_layers_smart_filters::smartFilterRowMenu` |
| `sfcm_edit_gate` | Right-click a camera-raw and an unmodeled filter | Edit enabled, then disabled | `tst_layers_smart_filters::editSmartFilterIsGatedOnKind` |
| `sfcm_edit` | Edit Smart Filter… on Lens Correction | Its dialog opens prefilled | `tst_layers_smart_filters::editSmartFilterRequestsTheDialog` |
| `sfcm_double_click` | Double-click a generic filter row | Its dialog opens prefilled | `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` |
| `sfcm_filter_actions` | Disable then Delete a filter | Disabled, then removed | `tst_layers_smart_filters::filterRowDisablesAndDeletes` |
| The parent and one child per filter appear | Open a smart object with a filter chain | Smart Filters parent and one row per filter | `tst_layers_smart_filters::treeShowsGroupAndToggles` |
| `lp_sf_order` | Apply Gaussian Blur, then Emboss | Emboss is the top row | `tst_smart_filter_apply::filterRowsShowNewestFirst` |
| Toggling a filter eye updates the composite | Flip a filter eye | Composite updates; one state | `pictura-app::app_setter_changes_the_composite_and_records_one_state` |
| Toggling the group eye bypasses the chain | Flip the group eye off | The unfiltered source shows | `pictura-app::disabled_group_bypasses_the_chain` |
| A toggle survives a write and re-read | Toggle, save, reopen | The flag survives | `pictura-app::toggles_persist_on_a_preserved_smart_object` |
| Synthetic rows are not real layers | Double-click a filter row | No rename, no layer change; the dialog opens | `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` |
| Gallery re-edit by hand | Double-click a Filter Gallery row, change an effect, OK | The gallery opens on its stack; the row is replaced in place | manual: the Filter Gallery dialog's thumbnail renders and modal loop are not driven offscreen in this suite |
| Codec in-place replace | Replace the first of two preserved entries | Position, enable flag and blendOptions kept | `pictura-codec::replace_keeps_position_flag_and_blend_options` |
| Render in-place replace | Replace index 0 of two | Order kept; out of range refused | `pictura-render::replace_at_index_keeps_order` |
