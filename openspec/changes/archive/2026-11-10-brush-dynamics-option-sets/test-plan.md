# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Fade ramps to the minimum | Evaluate a Fade(4) size control for dab indices 0, 2, 4, and 8. | The factor falls from the base to the minimum and stays there. | `pictura-paint::fade_reaches_the_minimum_and_stays` |
| Pressure raises the controlled value | Evaluate size with Pen Pressure at 0.25 and 0.75 and a 20 % minimum. | The 0.75 dab is larger and neither exceeds the base. | `pictura-paint::pressure_raises_the_controlled_value` |
| Control Off and zero jitter are unchanged | Paint the same stroke with the old defaults and with the new dynamics block at defaults. | Pixels are identical. | `pictura-paint::default_dynamics_paint_the_same_pixels` |
| Direction follows the stroke tangent | Evaluate Angle Control Direction at tangents 0° and 90°. | The tip angle follows each tangent. | `pictura-paint::direction_angle_follows_the_tangent` |
| Minimum diameter floors size jitter | Evaluate size jitter 100 % with Minimum Diameter 40 % over many draws. | Every factor is at least 0.4. | `pictura-paint::minimum_diameter_floors_size_jitter` |
| Tilt projects the tip | Evaluate a projection tip at zero tilt and at 45° tilt. | The projected tip is narrower and turned. | `pictura-paint::projection_turns_and_flattens_with_tilt` |
| Both Axes chooses the distribution | Evaluate scatter offsets for a horizontal tangent with Both Axes on and off. | On spreads in y; off stays on the perpendicular. | `pictura-paint::both_axes_selects_the_scatter_plane` |
| Count jitter stays in range | Evaluate count jitter 100 % over many steps. | Every count is 1–16. | `pictura-paint::count_jitter_stays_in_range` |
| Pressure lowers opacity | Evaluate Transfer opacity at pressure 0.5 with 100 % jitter. | The factor is at most 0.5. | `pictura-paint::pressure_lowers_transfer` |
| Zero jitter leaves transfer alone | Evaluate Transfer with zero jitter. | Opacity and flow factors are 1.0. | `pictura-paint::zero_transfer_jitter_is_a_no_op` |
| Per-stroke color is constant | Paint a stroke with Apply Per Tip off and FG/BG jitter 100 %, then read the painted colors. | Every painted pixel has the same color. | `pictura-paint::per_stroke_color_is_constant` |
| Per-tip color varies | Paint a long stroke with Apply Per Tip on and Hue Jitter 100 %. | At least two dabs differ in hue. | `pictura-paint::per_tip_color_varies` |
| Purity desaturates | Evaluate a saturated color at Purity −100. | Red, green, and blue are equal. | `pictura-paint::purity_desaturates` |
| Override pressure feeds controls | Evaluate size with Pressure Override at 50 % and Pen Pressure control, using a zero device pressure. | The factor matches pressure 0.5. | `pictura-paint::pose_override_replaces_the_device_input` |
| Tablet tilt reaches the stroke | Construct a `DynamicsInput` from tilt and rotation and evaluate Pen Tilt and Rotation controls. | The signals follow the supplied values. | `pictura-paint::tilt_and_rotation_reach_the_signals` |
| A locked set survives a preset | Lock Scattering with a custom value, apply a preset, and read the model. | The custom scatter remains and other sets change. | `tst_brush_dynamics::lockedSetSurvivesPreset` |
| An unlocked set follows the preset | Apply a preset with Shape Dynamics unlocked. | The preset's shape values replace the old ones. | `tst_brush_dynamics::unlockedSetFollowsPreset` |
| A dynamics round trip | Change every dynamics value and padlock, save the session, and start a new window. | Every value and lock is restored. | `tst_brush_dynamics::dynamicsRoundTrip` |
| Old settings still load | Restore JSON with only the five legacy dynamics fields. | Legacy values load and new controls are at defaults. | `tst_brush_dynamics::legacySettingsKeepDefaults` |
| A dynamics row opens its page | Select Shape Dynamics and read the visible page's controls. | The size, angle, roundness, and minimum controls are visible and enabled. | `tst_brush_dynamics::shapeDynamicsPageOpens` |
| A set checkbox writes the model | Clear the Scattering checkbox and read the model. | The set is disabled and its stored values remain. | `tst_brush_dynamics::setCheckboxWritesTheModel` |
| A padlock retains a set | Click the Color Dynamics padlock and apply a preset. | The row shows the lock and Color Dynamics is unchanged. | `tst_brush_dynamics::lockedSetSurvivesPreset` |
| Roundness reaches the brush | Run the existing Brush panel roundness check. | Roundness 30 reaches the controller and the preview paints. | `tst_brush_dynamics::roundnessReachesTheBrush` |
| A flattened tip paints wide | Paint a 20 px, 30 % round dab. | It is wider than it is tall. | `tst_brush_dynamics::flattenedTipPaintsWide` |
