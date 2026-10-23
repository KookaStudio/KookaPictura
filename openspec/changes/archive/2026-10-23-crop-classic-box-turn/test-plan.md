# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cs_menu_items` | Crop tool; open the cog menu | Checkboxes, Color combo, Opacity slider field at 75 %, `P` and `H` labels | `tst_crop_settings::menuListsItsItems` |
| `cs_classic_turn_drag` | Classic; draw a tall box; turn it a quarter turn; drag its body down | The box moves down on screen, the image stays | `tst_crop_settings::boxesTurnOverTheFixedImageInClassic` |
| `cs_classic_turn_gesture` | Classic; sweep the pointer a quarter turn clockwise around the box; repeat in Modern | Box turns clockwise (content -90 degrees); Modern content +90 | `tst_crop_settings::rotationGestureTurnsTheBoxWithThePointer` |
| `cs_picker` | Choose Custom in the Color combo | The project Color Picker opens | manual: the picker is a modal dialog |
| `lcs_straighten_angle` | Modern; draw a 5° line then a 3° line | Angle -5, then -8 | `tst_crop_straighten::secondLineAddsToTheAngle` |
| `lcs_straighten_release` | Drag a line, then release | Image still during the drag; rotates on release | `tst_crop_straighten::referenceLineLevelsWithoutEmptyCorners` |
| `lcs_straighten_axis` | Near-vertical and right-to-left lines | Levels against the nearer axis | `tst_crop_grip::straightenAngleFromLine` |
| `lcs_straighten_classic_absolute` | Classic; draw a 5° line then a 3° line | Box levelled to -3, not -8 | `tst_crop_straighten::classicSecondLineIsAbsolute` |
