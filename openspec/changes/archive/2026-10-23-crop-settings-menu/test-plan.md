# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cs_menu_items` | Crop tool; open the cog menu | Classic (`P`), Show Cropped Area (`H`), Auto Center Preview, Enable Crop Shield, Color, Opacity (75 %), Auto Adjust Opacity | `tst_crop_settings::menuListsItsItems` |
| `cs_classic_gates` | Toggle Classic on, then off | Show Cropped Area and Auto Center Preview disabled and unchecked, then enabled and checked | `tst_crop_settings::classicModeGatesTheModernItems` |
| `cs_classic_drag` | Classic on; draw a box, drag inside it | The box moves | `tst_crop_settings::classicDragMovesTheBox` |
| `cs_keys` | Press `P`, `H`, `H`, `P`, `H` | Classic flips; Show Cropped Area flips only in Modern; tool stays Crop | `tst_crop_settings::keysToggleClassicAndShowArea` |
| `cs_show_area_off` | Modern; Show Cropped Area off | Outside is opaque; Classic still dims | `tst_crop_settings::hidingTheOutsideIsOpaque` |
| `cs_shield_off` | Enable Crop Shield off | Alpha 0 | `tst_crop_settings::shieldCanBeTurnedOff` |
| `cs_shield_opacity` | Opacity 40, 250, -5 | Alpha 102, 255, 0; stored value clamped | `tst_crop_settings::opacitySetsTheAlpha` |
| `cs_shield_color` | Custom red, then Match Canvas | Outside tinted red, then the canvas colour | `tst_crop_settings::customColourTintsTheShield` |
| `cs_auto_adjust` | Drag a box with Auto Adjust on, then off | Half alpha during the drag, full on release; unchanged when off | `tst_crop_settings::autoAdjustThinsADrag` |
| `cs_auto_center` | Modern; draw a box off centre; repeat with Auto Center off | Box centred after release; view stays put when off | `tst_crop_settings::autoCenterPansToTheBox` |
| `cs_persist` | Change every setting; reload from the session store | Values restored | `tst_crop_settings::settingsPersist` |
| Colour picker and menu look | Choose Custom in the menu; compare the menu with CS6 | Picker opens, swatch shows the colour; compact rows | manual: the modal colour dialog and visual spacing cannot be driven offscreen |
