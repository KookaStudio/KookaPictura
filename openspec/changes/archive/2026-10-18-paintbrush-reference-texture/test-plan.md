# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The count is 48 and the last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Set a different foreground, Mode, Opacity, and Flow; choose each of the four presets in turn | Each preset applies its tip and its default colour, Mode, Opacity, and Flow; both pencils report hardness 65 and grain 50 / 75 | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings` |
| The airbrush paints a light, soft stroke | Paint a straight 45 px, hardness 0, Flow 50 stroke at 25 % and 10 % spacing | The 25 % stroke's peak alpha is 150–220; the 10 % stroke's peak is at least 20 higher | `pictura-paint::airbrush_soft_round_spacing_keeps_the_stroke_airy` |
| A preset sets the tip only | Set a non-default foreground, Mode, Opacity, and Flow; choose `Soft Round 5` | The tip changes and the paint settings and foreground are unchanged | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared; both pencils' grain is 50 / 75 and hardness 65 | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 50 | The grained dab has partial alpha, no holes in its core, about 74 % of the plain coverage, and renders identically twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
| The Non-Photo Blue Pencil paints a mottled stroke | Paint a dab with grain 0 and grain 75 | The grained dab has partial alpha, no holes in its core, about 61 % of the plain coverage, and renders identically twice | `pictura-paint::grain_breaks_up_the_dab_alpha`, `pictura-paint::grain_strength_matches_the_pencil_presets` |
| Grain breaks up the dab alpha | Paint a dab with grain 75 and the same dab with grain 0 | The grained dab has partial alpha, no zero-alpha core pixels, at least half the plain coverage, less total coverage, and repeats exactly | `pictura-paint::grain_breaks_up_the_dab_alpha` |
| Grain strength follows the pencil presets | Paint a dab with grain 50 and another with grain 75, each against a plain dab | The grain-50 dab keeps about 75 % of the plain coverage and the grain-75 dab about 60 % | `pictura-paint::grain_strength_matches_the_pencil_presets` |
| Grain is clumped | Sample `grain_noise` over a 64 × 64 layer | Values stay in 0–1, repeat exactly, and a 2 px box average keeps more than 60 % of the pixel spread | `pictura-paint::grain_noise_is_deterministic_and_clumpy` |
