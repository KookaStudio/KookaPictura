# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A smooth tip is unchanged | Paint the same stroke with the default config before and after the bristle field exists. | Pixels are identical. | `pictura-paint::smooth_tip_is_unchanged` |
| Zero bristles still paint | Paint one dab with a bristle tip at Bristles 0. | At least one pixel changes. | `pictura-paint::zero_bristles_still_paint` |
| Flat and round marks differ | Build a Flat and a Round dab at the same size and count; measure a cross-section. | The Flat section is wider and flatter than the Round one. | `pictura-paint::flat_and_round_marks_differ` |
| Bristles changes the streak count | Build dabs at Bristles 10 and 90. | The high value has more tracks. | `pictura-paint::bristles_changes_the_streak_count` |
| Length and thickness change the mark | Compare the reach and half-width of two tips with different Length/Thickness. | Higher Length reaches farther; higher Thickness is wider. | `pictura-paint::length_and_thickness_change_the_mark` |
| A bristle stroke shows streaks | Paint a straight bristle stroke and scan a perpendicular cross-section. | More than one painted run with gaps. | `pictura-paint::bristle_stroke_shows_streaks` |
| Spacing still separates dabs | Paint two bristle dabs at 100 % spacing. | The dabs do not cover the midpoint between them. | `pictura-paint::spacing_still_separates_dabs` |
| Low stiffness deforms the tip | Build the same tip at Stiffness 0 and 100. | The low-stiffness track tips are displaced. | `pictura-paint::low_stiffness_deforms_the_tip` |
| High stiffness keeps the shape | Build the tip at Stiffness 100 with two tangents. | Track roots stay on the same radial family. | `pictura-paint::high_stiffness_keeps_the_shape` |
| A wet bristle stroke varies across the tip | Run a bristle Mixer stroke across a colour boundary with Wet > 0. | One cross-section has more than one colour. | `pictura-paint::wet_bristle_stroke_varies_across_the_tip` |
| A dry bristle stroke stays crisp | Run the same stroke with Wet 0. | Every painted pixel is the loaded colour. | `pictura-paint::dry_bristle_stroke_stays_crisp` |
| The same bristle stroke replays | Paint the same bristle stroke twice. | Both documents are byte-identical. | `pictura-paint::bristle_stroke_replays_exactly` |
| Bristle declines the GPU | Start a bristle stroke with a GPU adapter present. | The stroke commits through the CPU bristle raster. | manual: needs a live Vulkan adapter; the gating is a one-line `cfg.bristle.is_none()` in `gpu_paint_mode` and the committed-pixel parity is covered by `bristle_stroke_replays_exactly` |
| Roundness reaches the brush | Existing Brush panel roundness check. | Roundness 30 reaches the controller and the preview paints. | `tst_brush_dynamics::roundnessReachesTheBrush` |
| A flattened tip paints wide | Paint a 20 px, 30 % round dab. | It is wider than it is tall. | `tst_brush_dynamics::flattenedTipPaintsWide` |
| A dynamics row opens its page | Select Shape Dynamics. | Its controls are visible and enabled. | `tst_brush_dynamics::shapeDynamicsPageOpens` |
| A set checkbox writes the model | Clear the Scattering checkbox. | The set disables and keeps its stored values. | `tst_brush_dynamics::setCheckboxWritesTheModel` |
| A padlock retains a set | Lock Color Dynamics and apply a preset. | Color Dynamics is unchanged. | `tst_brush_dynamics::lockedSetSurvivesPreset` |
| Bristle controls write the model | Toggle Bristle Tip, choose Flat Fan, and set the four fields. | The controller reports every value and the preview paints. | `tst_bristle_brushes::bristleControlsWriteTheModel` |
| Bristle disables hardness | Toggle Bristle Tip on. | Hardness and Roundness are disabled; the bristle fields are enabled. | `tst_bristle_brushes::bristleDisablesHardness` |
| One flip mirrors the angle | Set angle 30° with Flip X on. | The painted angle is −30°. | manual: the existing `PaintOptions::angle()` path is unchanged by this change and has no Qt case; the options-persistence round trip covers Flip X/Y storage |
| Opening the picker from the Eraser | Click the Eraser bar's tip button. | The picker opens with the full default set. | manual: legacy self-test 549 opens the picker and checks the size mapping |
| The slider midpoint is 100 px | Set the Size slider to its middle. | The brush size is 100 px. | manual: legacy self-test 549 checks `brush_size_scale::sizeAt(kSteps / 2) == 100` |
| A spatter preset paints scattered | Choose Spatter 24 and click once. | Paint lands well outside the 24 px tip. | manual: legacy self-test 549 paints the spatter preset |
| The four additional presets are listed | Open the picker and read the last four entries. | The four issue-#731 names are last. | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Choose each issue-#731 preset. | Tip and paint settings match the table. | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings` |
| The airbrush paints a light, soft stroke | Paint the airbrush preset. | Its core stays below full opacity. | manual: the existing profile is unchanged and the legacy profile check covers it; no new Qt case in this change |
| A preset sets the tip only | Choose an existing CS6 preset after changing paint settings. | Paint settings are unchanged. | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose each pencil preset. | The pen-tip model follows. | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint `2B Pencil`. | Alpha is clumpy and deterministic. | manual: the grain is unchanged by this change; the existing profile checks cover it |
| The Non-Photo Blue Pencil paints a mottled stroke | Paint `Non-Photo Blue Pencil`. | Alpha is mottled and deterministic. | manual: the grain is unchanged by this change; the existing profile checks cover it |
| The oval preset paints a wet-edged stroke | Choose `Paintbrush Oval 45 pixels Multiply`. | Tip and Wet Edges match; a later CS6 preset turns Wet Edges off. | `tst_brush_tip_preview::ovalPresetSetsWetEdges` |
| The 2B Pencil paints the reference profile | Paint a straight `2B Pencil` stroke. | Cross-section profile is in range. | manual: the profile is unchanged by this change; the existing profile checks cover it |
| The Non-Photo Blue Pencil paints the reference profile | Paint a straight `Non-Photo Blue Pencil` stroke. | Cross-section profile is in range. | manual: the profile is unchanged by this change; the existing profile checks cover it |
| Bristle presets are listed and applied | Choose `Bristle Flat 40`. | The controller reports a bristle Flat tip at 40 px and the preset values. | `tst_bristle_brushes::bristlePresetApplies` |
| A round preset clears the bristle tip | Choose a bristle preset, then `Soft Round 5`. | The controller reports a smooth tip. | `tst_bristle_brushes::roundPresetClearsBristle` |
| `top_round_trip` | Existing tool-options restart case. | Every changed value restores. | `tst_tool_options_persistence::optionsSurviveARestart` |
| `top_bad_entries` | Existing bad-entry case. | Values clamp and defaults remain. | `tst_tool_options_persistence::badEntriesKeepDefaults` |
| `top_silent_restore` | Existing silent-restore case. | No listener fires. | `tst_tool_options_persistence::restoreDoesNotNotify` |
| `top_missing_default` | Existing missing-field case. | Omitted fields read defaults. | `tst_tool_options_persistence::missingFieldsRestoreDefaults` |
| Bristle fields survive a restart | Set a bristle tip, save, and open a new window. | Every bristle value restores. | `tst_bristle_brushes::bristleRoundTrip` |
| Old settings keep a smooth tip | Restore a brush group without bristle fields. | The toggle is off and the values are defaults. | `tst_bristle_brushes::legacySettingsKeepSmoothTip` |
| `uistalt_marquee_options_persist` | Existing marquee restart case. | Each marquee restores its own style and size. | `tst_tool_options_persistence::ellipticalOptionsRoundTrip` |
