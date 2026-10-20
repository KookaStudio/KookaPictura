# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Normal blends by coverage | Paint a Normal stroke at full opacity over an opaque base | Fully covered pixels become the paint colour | `pictura-paint::default_brush_paints_center_with_configured_color` |
| Clear reduces alpha | Paint Clear at full coverage on an opaque pixel | Alpha becomes 0 | `pictura-paint::clear_drives_opaque_alpha_to_zero` |
| Behind leaves opaque pixels untouched | Paint Behind over an opaque pixel | Colour unchanged | `pictura-paint::behind_leaves_opaque_rgb_unchanged` |
| Multiply darkens by the backdrop | Paint Multiply grey 128 at full opacity over an opaque coloured base and over a transparent layer | Opaque pixels equal `base·128/255`; the transparent pixel becomes grey 128 | `pictura-paint::multiply_darkens_by_the_backdrop` |
| Wet-edge transfer curve | Evaluate the curve at 0, 128, 255 and scan 0..=255 | 0, ~103, ~80; one peak | `pictura-paint::wet_edge_curve_peaks_then_falls` |
| Wet edges lighten the core and darken the rim | Paint the oval preset's stroke and a single dab over white | Interior 18–22 below white, dab centre 24–28, rim darker than interior | `pictura-paint::wet_edges_lighten_the_core_and_darken_the_rim` |
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Set different paint settings; choose each of the four presets | Each preset applies its tip, colour, Mode, Opacity, and Flow | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings`, `tst_brush_tip_preview::presetUpdatesTheOptionsBar` |
| The airbrush paints a light, soft stroke | Paint the airbrush stroke at 25 % and 10 % spacing | The 25 % core is lighter | `pictura-paint::airbrush_soft_round_spacing_keeps_the_stroke_airy` |
| The oval preset paints a wet-edged stroke | Choose the oval preset, then `Soft Round 5` | 45 px, hardness 100, roundness 50, angle −50, spacing 5, Wet Edges on; then off | `tst_brush_tip_preview::ovalPresetSetsWetEdges` |
| A preset sets the tip only | Set a non-default foreground, Mode, Opacity, and Flow; choose `Soft Round 5` | The tip changes and the paint settings and foreground are unchanged | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared; both pencils' grain is 50 / 75 and hardness 65 | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 50 | About 74 % of the plain coverage, no core holes, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
| The Non-Photo Blue Pencil paints a mottled stroke | Paint a dab with grain 0 and grain 75 | About 61 % of the plain coverage, no core holes, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
