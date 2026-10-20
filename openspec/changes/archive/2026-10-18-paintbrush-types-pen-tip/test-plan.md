# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The four additional presets are listed | Open the Brush Preset picker and read the grid's tooltips | The count is 48 and the last four names are the issue's four presets in order | `tst_brush_tip_preview::fourAdditionalPresets` |
| A preset sets the tip only | Set Mode multiply, Opacity 35, Flow 50, and a blue foreground; choose `Paintbrush Oval 45 pixels Multiply` | The tip is 45 px, hardness 60, roundness 35, spacing 15, no dynamics or grain; mode, opacity, flow, and foreground are unchanged | `tst_brush_tip_preview::presetSetsTipOnly` |
| The pencil presets set the pen-tip model | Choose `2B Pencil`, `Non-Photo Blue Pencil`, then `Soft Round 5` | The model is set, set, then cleared; both pencils’ grain is 30 | `tst_brush_tip_preview::pencilPresetsSetThePenTipModel` |
| The 2B Pencil paints a grainy stroke | Paint a dab with grain 0 and grain 80 | The grained dab has partial alpha, no holes in its core, at least half the plain coverage, less total coverage, and renders identically twice | `pictura-paint::grain_breaks_up_the_dab_alpha` |
| A pencil preset shows the pen tip | Brush tool, choose `2B Pencil` | The canvas child overlay is visible at (8, 8) and paints a light barrel over a dark nib | `tst_brush_tip_preview::pencilPresetShowsThePenTip` |
| A non-pencil preset hides the pen tip | Choose `2B Pencil`, then `Soft Round 5` | Visible, then hidden | `tst_brush_tip_preview::nonPencilPresetHidesThePenTip` |
| Another tool hides the pen tip | With `2B Pencil`, select Move, then Brush | Hidden, then visible | `tst_brush_tip_preview::anotherToolHidesThePenTip` |
| The overlay drags without painting | Press the overlay, move 20 px, release; then drag far past the canvas edge | The overlay moves by 20 px, stays inside the canvas, and the canvas emits no mouse press | `tst_brush_tip_preview::overlayDragsWithoutPainting` |
| The preview follows the active document | Open a second document, apply `2B Pencil`, switch tabs | Only the active document's canvas shows the overlay | `tst_brush_tip_preview::previewFollowsTheActiveDocument` |
