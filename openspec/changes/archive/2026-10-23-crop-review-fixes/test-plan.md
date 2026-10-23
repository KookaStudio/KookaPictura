# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cs_defaults` | Construct fresh `CropOptions`, `CropSettings`, `SessionState` | Classic off; Show Cropped Area, Auto Center Preview on; show mode Auto | `tst_crop_settings::defaultsMatchCs6` |
| `cs_keys` | Show the window; type `P`, `H`, `H`, `P`, `H` with `QTest::keyClick(windowHandle())` | Settings flip as specified; Crop stays active (P is a tool-letter shortcut too) | `tst_crop_settings::keysToggleClassicAndShowArea` |
| `cs_persist` | Change opacity, colour, toggles, show mode, orientation; reset; reload the session | Values come back | `tst_crop_settings::settingsPersist` |
| `cs_runtime_patch` | With a box shown, apply `{"crop":{"shieldOpacity":40}}` | Menu field 40; shield alpha 102 | `tst_crop_settings::runtimePatchRefreshesTheMenu` |
| `cs_classic_turn_drag` | Turn a Classic box 90°; drag its body down on screen | The box moves; the image does not | `tst_crop_settings::boxesTurnOverTheFixedImageInClassic` |
| `cs_classic_turn_gesture` | Sweep a quarter turn clockwise in Classic, then Modern | Box turns -90°; Modern content turns +90° | `tst_crop_settings::rotationGestureTurnsTheBoxWithThePointer` |
| `cs_classic_turn_commit` | 100 x 100 white canvas extended to 200 x 100 by Canvas Size; Classic 20 x 80 box across the edge; turn 90°; commit | 20 x 80 result, one end each half | `tst_crop_settings::turnedClassicCommitTakesTheContentUnderTheBox` |
| `cof_show_mode` | Auto, Always, Never from the grid menu; rest and drag the box | Guides drawn as specified | `tst_crop_settings::gridMenuDrivesTheOverlay` |
| `cof_cycle_keys` | Type `O`, `Shift+O` on Grid and on Triangle; use the menu's cycle entries | Overlay and orientation step; Orientation entry enabled only on Triangle / Golden Spiral | `tst_crop_settings::keysToggleClassicAndShowArea`, `tst_crop_settings::gridMenuDrivesTheOverlay` |
