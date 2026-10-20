# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The count is 48 and the last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| The additional presets apply their paint settings | Set a different foreground, Mode, Opacity, and Flow; choose each of the four presets in turn | Each preset applies its tip and its default colour, Mode, Opacity, and Flow; the blue pencil uses `#73d6ff`, the others `#47484a`, and the oval uses Multiply/35/50 | `tst_brush_tip_preview::additionalPresetsApplyTheirPaintSettings`, `tst_brush_tip_preview::presetUpdatesTheOptionsBar` |
| The airbrush paints a light, soft stroke | Paint a straight 45 px, hardness 0, Flow 50 stroke at 25 % and 10 % spacing | The 25 % stroke's peak alpha is 150–220; the 10 % stroke's peak is at least 20 higher | `pictura-paint::airbrush_soft_round_spacing_keeps_the_stroke_airy` |
| A preset sets the tip only | Set a non-default foreground, Mode, Opacity, and Flow; choose `Soft Round 5` | The tip changes and the paint settings and foreground are unchanged | `tst_brush_tip_preview::existingPresetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared; both pencils' grain is 30; the colours are `#47484a` and `#73d6ff` | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 80 | The grained dab has partial alpha, no holes in its core, at least half the plain coverage, less total coverage, and renders identically twice | `pictura-paint::grain_breaks_up_the_dab_alpha` |
