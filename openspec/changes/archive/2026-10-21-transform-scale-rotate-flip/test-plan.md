# Test plan — Wire Edit > Transform Scale/Rotate/Again and the exact rotations and flips

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tc_menu` | Open the Edit > Transform submenu | Items in CS6 order with separators after Again, Warp, and the quarter turns; Again shows Shift+Ctrl+T | `tst_transform_commands::menuListsTheTransformCommands` |
| `tc_enable` | Refresh with a raster layer current, then with the Background current | Group enabled for the raster layer; Scale/Rotate/exact/flip disabled for the Background | `tst_transform_commands::commandsEnableOnATransformableTarget` |
| `tc_scale` | Dispatch Scale; hover outside the box; drag the top-right handle outward | Outside hover grabs nothing; width grows, height and angle unchanged | `tst_transform_commands::scaleScalesWithoutRotating` |
| `tc_rotate` | Dispatch Rotate; press a handle; drag a quarter turn about the pivot | Angle is 90°, scale stays 100% | `tst_transform_commands::rotateRotatesWithoutScaling` |
| `tc_mode_switch` | Begin Free Transform, set a width, dispatch Scale | Session stays open with the same box, mode is `scale` | `tst_transform_commands::choosingAModeMidSessionKeepsTheBox` |
| `tc_rot90` | Dispatch Rotate 90° Clockwise on a 12×10 layer | Box becomes 10×12; one `Rotate 90° Clockwise` state | `tst_transform_commands::rotate90ClockwiseSwapsTheBox` |
| `tc_rot180_flip` | Rotate 180°, Flip Horizontal, Flip Vertical on an off-centre mark | Mark at the opposite corner, then the horizontal mirror, then the vertical mirror | `tst_transform_commands::exactRotationsAndFlipsMoveTheContent` |
| `tc_selection` | Select one of two marks; dispatch Rotate 90° Clockwise | Selected mark moves; unselected mark stays | `tst_transform_commands::selectionTransformsOnlyItsPixels` |
| `tc_session` | Begin Free Transform; dispatch Rotate 90° Clockwise | Session stays open; angle 90°; no history state | `tst_transform_commands::openSessionUpdatesInPlace` |
| `tc_again` | Dispatch Rotate 90° Clockwise, then Again | Total 180°; second state is `Transform Again` | `tst_transform_commands::againRepeatsTheLastTransform` |
| `tc_again_disabled` | Refresh before any transform | Again is disabled | `tst_transform_commands::againIsUnavailableBeforeATransform` |
