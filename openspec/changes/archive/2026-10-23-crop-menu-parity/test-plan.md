# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cs_menu_items` | Crop tool; open the cog menu | Checkable items, Color combo, Opacity slider field at 75 %, `P` and `H` column | `tst_crop_settings::menuListsItsItems` |
| Menus look alike | Open the guide-overlay and Settings menus | Same padding, check marks and separators | manual: visual spacing cannot be asserted offscreen |
