# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `bdr_exact` | Paint 12 samples of 260 px dabs over a textured offset layer per mode / tip / dynamics / lock / stamp source, banded and forced serial | Same changed flag and dirty rect per sample; byte-identical documents | `pictura-paint::a_banded_large_dab_matches_the_serial_raster` |
| `bdr_fill_repeat` | Fill a 64² layer twice with a half-opaque colour at half strength in Dissolve | ~¼ of the pixels painted wholly; both fills identical | `pictura-paint::dissolve_paints_whole_pixels_in_proportion_and_repeats` |
| `bdr_coverage_cols` | Rasterize an inner and a left-overhanging ellipse at 30 px and 90 px wide | First 30 columns agree per row; the rest empty | `pictura-core::coverage_does_not_depend_on_the_columns_beside_the_path` |
| `btc_stroke` | 200 px brush, 8 dabs over a textured document, presenting after some, then end | Frame not rebuilt; stroke in the composite; composite == full CPU recomposite | `tst_tool_commit::aStrokeCommitsOnlyWhatIsPending` |
| `btc_cancel` | Paint and present a stroke, then cancel | Stored composite equals its pre-stroke pixels | `tst_tool_commit::aCancelledStrokePutsTheCompositeBack` |
| `btc_shape` | Add an ellipse shape layer with a 6 px outside stroke | Frame not rebuilt; centre is the fill; outside pixel unchanged; composite == full recomposite | `tst_tool_commit::aShapeLayerCompositesItsBounds` |
| `btc_fill` | Draw a gradient inside a rectangular selection | Frame not rebuilt; composite == full recomposite | `tst_tool_commit::aFillCompositesItsRect` |
| `btc_styled` | Give a layer a Color Overlay, then draw a gradient | Frame rebuilt; composite == full recomposite | `tst_tool_commit::aLayerStyleRecompositesTheDocument` |
| `btc_style_scope` | Search a tree with a styled layer nested in a group, exempting paths | Nested style found; exempt layer's own style not; group exemption spares not its child | `pictura-app::a_style_anywhere_but_the_exempt_layer_counts` |
| `vms_beyond` | Sample a rectangle mask built for a 2-row clip above, below and inside the path, both fill rules | Inside rows inside; beyond rows outside | `pictura-core::a_row_beyond_the_path_is_outside_whatever_the_cached_clip` |
| `ctl_tool_options` | `tool_options` with a brush size, then without options, then with a number | 321 reported and used; options returned; `invalid_param` | `tst_tool_commit::toolOptionsPatchAndReadBack` |
| `ctl_pointer_steps` | Brush; `pointer` down, moves, up in image space; then one more move | One stroke recorded along the moves; the stray move paints nothing | `tst_tool_commit::pointerDownMoveUpPaints` |
| `ctl_pointer_stale` | `pointer` down, a second down, click, then a plain move | Second down `invalid_param`; the move paints nothing | `tst_tool_commit::pointerDownMoveUpPaints` |
| `prof_compare` | `python3 scripts/profile-tools.py --save before.json` on one build, `--compare before.json` on the next | Per-scenario timings and a before → after table | manual: needs a built app and wall-clock timings that vary by machine; the before/after tables in `design.md` are its output |
