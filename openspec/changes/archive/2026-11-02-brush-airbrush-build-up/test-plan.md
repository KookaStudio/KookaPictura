# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `brush_airbrush_off` | Hold a low-flow Brush still with Airbrush off | Pixels unchanged | `tst_brush_airbrush::airbrushOffStaysStill` |
| `brush_airbrush_on` | Hold a low-flow Brush still with Airbrush on, release | Pixels build; one history state | `tst_brush_airbrush::airbrushOnBuilds` |
| `brush_airbrush_two_views` | Click the options-bar toggle, then uncheck the Brush panel row | Both views and the model agree | `tst_brush_airbrush::optionsBarAndPanelShareTheFlag` |
| `brush_hold_stops` | End a held Brush stroke by release, tool switch, cancel | Pixels stop changing | `tst_brush_airbrush::holdStopsOnRelease`, `tst_brush_airbrush::holdStopsOnToolSwitch`, `tst_brush_airbrush::holdStopsOnSessionCancel` |
| `brush_shortcut` | Press `Shift+Alt+P` with the Brush active | Flag, bar toggle, and Brush panel row flip | `tst_brush_airbrush::shiftAltPTogglesTheBrush` |
| `stamp_shortcut` | Press `Shift+Alt+P` with each stamp tool active | That tool's flag flips; the others stay | `tst_brush_airbrush::shiftAltPTogglesAStamp` |
| `brush_shortcut_ignored` | Press `Shift+Alt+P` with Move and Pencil active | No flag changes | `tst_brush_airbrush::shiftAltPIgnoresToolsWithoutTheOption` |
| `brush_preview_hold` | Hold a 600 px Brush on a 1024² document with the GPU off; compare with a one-dab stroke | Held commit is darker than the one-dab commit | `tst_brush_airbrush::previewHoldReachesTheCommit` |
| `brush_mask_hold` | Hold a mask Brush exactly and through the preview | Coverage builds; one history state | `tst_brush_airbrush::maskHoldBuilds`, `tst_brush_airbrush::maskPreviewHoldReachesTheCommit` |
| `brush_gpu_hold` | Hold a 600 px GPU Brush and commit | Pixels include the build-up | `tst_brush_airbrush::gpuHoldBuilds` |
| Spacing after a hold | Move on after holding | Path spacing unchanged | `pictura-paint::repeat_last_keeps_path_spacing` |
| Flow 0 | Hold at Flow 0 | No change | `pictura-paint::hold_paints_nothing_at_flow_zero` |
| Raster equivalence | Hold on banded and serial strokes | Same bytes | `pictura-paint::hold_matches_serial_raster` |
| App focus lost mid-press | Hold with Airbrush on, Alt-Tab away | Painting stops at once | manual: offscreen Qt has no real application focus change |
| Feel at 20 dabs/s | Hold at 10% Flow with a soft tip | Build-up looks smooth | manual: visual judgement; the CS6 rate is undocumented |
