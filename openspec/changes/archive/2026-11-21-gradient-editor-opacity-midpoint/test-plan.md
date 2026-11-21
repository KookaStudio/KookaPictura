# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `gt_editor_opens` | Click the main part of the Gradient tool's sample. | A `GradientEditorDialog` opens with the tool's current stops. | `tst_fill_tools::gradientEditorFromTheSample` |
| `gt_editor_applies` | Set the start opacity stop to 0 %, move the colour midpoint, accept, then drag a gradient. | The start is transparent and the even-mix point is shifted. | `tst_fill_tools::gradientEditorAppliesOpacityAndMidpoint` |
| `gt_editor_cancels` | Open the editor, edit a stop, cancel, then drag. | The drag matches the ramp before the editor opened. | `tst_fill_tools::gradientEditorCancelKeepsTheRamp` |
| `gt_preset_replaces` | Accept an edited ramp, then choose a built-in preset and drag. | The drag matches the preset and the edited spec is empty. | `tst_fill_tools::gradientPresetReplacesTheEditedRamp` |
| `ge_opacity_stops` | Sample and draw a black-to-white gradient with 0 % / 100 % opacity stops. | Start alpha is 0, end alpha is 255, colour still runs black to white, and the drawn pixels take those alphas. | `pictura-paint::opacity_stops_set_the_alpha`, `pictura-paint::opacity_stops_reach_the_pixels` |
| `ge_midpoint` | Sample a black-to-white gradient whose start midpoint is 25 %. | 25 % is mid grey and 50 % is lighter than mid grey. | `pictura-paint::midpoint_biases_the_even_mix` |
| `ge_stop_editing` | Add an opacity stop above the ramp, then drag it off the bar. | The opacity list grows by one and returns to its original size. | `tst_image_adjustments::gradientEditorEditsOpacityAndMidpoint` |
| `ge_stop_selection` | Select an opacity stop in the editor. | Opacity, Location, Delete, and Midpoint act on it; Color is disabled. | `tst_image_adjustments::gradientEditorEditsOpacityAndMidpoint` |
| Reverse preserves the new fields | Build a gradient with an off-centre midpoint and opacity stops, reverse it. | Positions, midpoints, and opacity stops mirror. | `pictura-paint::midpoint_and_opacity_survive_reverse` |
