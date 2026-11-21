# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pen_output_mode_bar` | Select the Pen, then the Freeform Pen; change each page's mode | Both bars list Shape, Path, and Pixels with Shape selected; changing one page changes the other's index | `tst_pen_tools::penModeBar` |
| `pen_output_shape` | Set Mode Shape and close a 40 x 40 Pen square | One new shape layer, empty Work Path, foreground fill inside, "Pen Tool" state | `tst_pen_tools::penShapeMode` |
| `pen_output_path` | Set Mode Path and close a 40 x 40 Pen square | A saved "Path 1", and the Work Path still holds the closed four-anchor square | `tst_pen_tools::penPathMode` |
| `pen_output_pixels` | Set Mode Pixels and close a 40 x 40 Pen square | No new layer, the square edge is painted, "Stroke Path" state | `tst_pen_tools::penPixelsMode` |
| Existing Pen editing keeps the Work Path | Draw, close, resume, and edit anchors with Mode Path | The Work Path remains and anchor edits still apply | `tst_pen_tools::penTool`, `tst_pen_tools::penAutoAddDelete`, `tst_pen_tools::anchorPointTools`, `tst_path_selection_tools::pathSelectionTool`, `tst_type_on_path::clickOnOpenPathStartsOnPath` |
