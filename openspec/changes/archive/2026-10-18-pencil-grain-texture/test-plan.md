# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The count is 48 and the last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Set a different colour, Mode, Opacity, and Flow, then choose each of the four presets | Each preset applies its tip, colour, Mode, Opacity, and Flow; `2B Pencil` sets grain 80 and `Non-Photo Blue Pencil` grain 30 | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings`, `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The airbrush paints a light, soft stroke | Paint with `Airbrush Soft Round 50% flow` at 25 % spacing and again at 10 % | The 25 % core is lighter and below full opacity | `pictura-paint::airbrush_soft_round_spacing_keeps_the_stroke_airy` |
| A preset sets the tip only | Set a non-default colour, Mode, Opacity, and Flow, then choose a CS6 preset | Only the tip changes | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 80, then repeat the grained dab | Partial alpha, no core holes, 50–70 % of the plain coverage, less total coverage, identical twice | `pictura-paint::grain_breaks_up_the_dab_alpha` |
| Grain breaks up the dab alpha | Paint a dab with grain 0 and grain 80 | The grained dab has partial alpha, no zero-alpha core pixels, at least half the plain coverage, and repeats exactly | `pictura-paint::grain_breaks_up_the_dab_alpha` |
| Grain is clumped | Sample `grain_noise` over a 64 × 64 layer | Values stay in 0–1, repeat exactly, and a 2 px box average keeps more than 60 % of the pixel spread | `pictura-paint::grain_noise_is_deterministic_and_clumpy` |
