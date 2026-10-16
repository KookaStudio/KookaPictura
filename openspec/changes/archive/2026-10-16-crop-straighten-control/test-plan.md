# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lcs_straighten_control` | Crop tool; click the Straighten icon, then the label | Icon with `Straighten` label; each click toggles the line | `tst_crop_options::straightenToggleArmsLineMode` |
| `lcs_straighten_ctrl` | Hold and release Ctrl; repeat with the line armed from the control; press with Ctrl held and no key event; release Ctrl mid-line | Armed while held; a control-armed line stays armed; a Ctrl press draws; a line being drawn finishes | `tst_crop_straighten::holdingCtrlArmsTheLine`, `tst_crop_straighten::ctrlPressDrawsALineWithoutAKeyEvent`, `tst_crop_straighten::ctrlReleaseMidLineFinishesIt` |
| `lcs_straighten_angle` | Draw a 5° line, then a 3° line; commit | Angle −5°, then −8° (within 0.1°); the commit crops to the box | `tst_crop_straighten::referenceLineLevelsWithoutEmptyCorners`, `tst_crop_straighten::secondLineAddsToTheAngle` |
| `lcs_straighten_release` | Drag a line, then release | Image still during the drag with the `∠` readout; rotates on release, readout cleared | `tst_crop_straighten::referenceLineLevelsWithoutEmptyCorners` |
| `lcs_straighten_axis` | Draw near-vertical and right-to-left lines | Levels against the nearer axis, never beyond ±45° | `tst_crop_grip::straightenAngleFromLine` |
| `lcs_straighten_fit` | Straighten a full-canvas box; a small box; a box off the content | Shrinks about its centre with its ratio, a corner touching; never grows; off-content box unchanged | `tst_crop_grip::fitBoxInsideRotatedContent`, `tst_crop_straighten::referenceLineLevelsWithoutEmptyCorners` |
| `lcs_box_fixed` | Apply a straighten angle | Box stays axis-aligned and in place; content rotates | `tst_crop_preview::straightenRotatesTheContent` |
| `lcs_move_keeps_content` | Move the box after straightening | Content stays; only the box moves | `tst_crop_preview::movingTheBoxKeepsThePivot` |
| `lcs_canvas_bbox` | Straighten a non-square document | Canvas grows to the rotated bounds | `tst_crop_preview::canvasGrowsToTheRotatedBounds` |
| Default angle is a no-op | Crop with angle 0 | Axis-aligned crop | `tst_crop_straighten::angleZeroIsAnAxisAlignedCrop` |
| `lcs_cursor_handle` | Hover a handle | Resize cursor | `tst_crop_cursor::cursorZones` |
| `lcs_cursor_box` | Hover the box body | Default arrow | `tst_crop_cursor::cursorZones` |
| `lcs_cursor_rotate` | Hover the rotate band, past the canvas edge | Rotate cursor | `tst_crop_cursor::cursorZones`, `tst_crop_cursor::rotateCursorZones` |
| `lcs_cursor_far` | Hover beyond the band | Default arrow | `tst_crop_cursor::farZoneDefaultAndApply` |
| `lcs_cursor_newcrop` | No box drawn | New-crop crosshair | `tst_crop_cursor::cursorZones` |
| `lcs_cursor_modifier` | Press Shift over a zone | Same zone cursor | `tst_crop_cursor::cursorZones` |
| `lcs_cursor_straighten` | Arm from the control, then by holding Ctrl | Straighten cursor | `tst_crop_straighten::referenceLineLevelsWithoutEmptyCorners`, `tst_crop_straighten::holdingCtrlArmsTheLine` |
| Icon and cursor artwork | View `crop.straighten` and `cursor.straighten` at 1x and 2x, light and dark themes | Legible, crisp, hotspot at the crosshair centre | manual: visual judgement; a golden would churn on any artwork tweak |
| Ctrl with a focused panel | Focus the Layers panel, hold Ctrl, drag on the canvas | A line is drawn | manual: offscreen QPA does not reproduce window-manager key routing (code path automated above) |
| CS6 box choice | Compare the fitted box with CS6 on the same image | Close to CS6 | manual: no CS6 oracle; centre-anchored fit is a declared approximation (design.md) |
