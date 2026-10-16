# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lmc_vector_menu` | Vector menu rows; disable, enable, delete, undo, rasterize | Three rows; the toggle label follows the state; each one state | `tst_layers_mask_commands::vectorMaskMenu` |
| `lmc_trash_vector_prompt` | Trash on a doubly-masked layer: cancel, OK with "Don't show again", then again | Prompt "Delete Vector Mask?"; only the vector mask goes; no prompt the last time | `tst_layers_mask_commands::trashDeletesTheVectorMask` |
| `lmc_trash_selected_row` | Select the doubly-masked layer, Ctrl+Down, trash | One prompt; that layer's vector mask gone; layer count unchanged | `tst_layers_mask_commands::trashActsOnTheSelectedRow` |
| Real prompt and vector-thumbnail right-click | Click the trash on a doubly-masked layer; right-click a vector-mask thumbnail | The modal prompt and the popup show at the pointer | manual: offscreen QPA does not show popups or modal dialogs; their contents and actions are automated above |
