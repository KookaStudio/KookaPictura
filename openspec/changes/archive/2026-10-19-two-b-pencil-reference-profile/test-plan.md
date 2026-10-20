# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Set different paint settings; choose each of the four presets | Each preset applies its tip, colour, Mode, Opacity, and Flow | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings`, `tst_brush_tip_preview::presetUpdatesTheOptionsBar` |
| The airbrush paints a light, soft stroke | Paint the airbrush stroke at 25 % and 10 % spacing | The 25 % core is lighter | `pictura-paint::airbrush_soft_round_spacing_keeps_the_stroke_airy` |
| The oval preset paints a wet-edged stroke | Choose the oval preset, then `Soft Round 5` | 45 px, hardness 100, roundness 50, angle −50, spacing 5, Wet Edges on; then off | `tst_brush_tip_preview::ovalPresetSetsWetEdges` |
| A preset sets the tip only | Set a non-default foreground, Mode, Opacity, and Flow; choose `Soft Round 5` | The tip changes and the paint settings and foreground are unchanged | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared; the 2B Pencil is 8 px at hardness 25, the blue pencil 9 px at 65; grain 50 / 75 | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 50 | About 74 % of the plain coverage, no core holes, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
| The Non-Photo Blue Pencil paints a mottled stroke | Paint a dab with grain 0 and grain 75 | About 61 % of the plain coverage, no core holes, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
| The 2B Pencil paints the reference profile | Paint a straight `2B Pencil` stroke over white | Cross-sections cover 6–8 px, integrate to 3.2–4.4 px, and peak at 0.70–0.88 | `pictura-paint::two_b_pencil_paints_the_reference_profile` |
| Grain breaks up the dab alpha | Paint a dab with grain 0 and grain 75 | Partial alpha, no core holes, at least half the plain coverage, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha` |
| Grain strength follows the pencil presets | Paint dabs with grain 50 and 75 | About 75 % and 60 % of the plain coverage | `pictura-paint::grain_strength_matches_the_pencil_presets` |
| Grain is clumped | Sample the grain noise across 64×64 pixels | A 2 px box average keeps over 60 % of the spread; neighbour correlation 0.55–0.72 | `pictura-paint::grain_noise_is_deterministic_and_clumpy` |
