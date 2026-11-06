# Test Plan

Covers the `ui/ui-scaling` delta for `status-bar-text-scale`. The new scenario
makes the status bar's footer font follow the UI scale and reach every readout;
the rest of the requirement is unchanged and keeps its existing tests. No manual
rows are expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `uis_live_font_qss` | Compare `QApplication::font().pixelSize()` at 100 % and after `setUiScalePercent(150)`, and read the stylesheet's menu-bar padding at 100 % and 200 % | The application font pixel size doubles and the stylesheet length doubles | `tst_ui_scale::fontAndStyleSheetFollowScale` |
| `uis_live_chrome` | Show the frame at 100 %, record the status bar minimum height, tool-slot width, menu-corner icon size, and panel-column bounds, then `setUiScalePercent(150)` and re-read them | Every metric matches `Theme::scaled()` of its 100 % value | `tst_ui_scale::frameChromeFollowsScale` |
| `uis_status_bar_font` | Create the main window at 100 %, read `QStatusBar` and the `statusSizeLabel`, `statusBackendLabel`, and `toolHintBar` widgets, compare each font pixel size with `QApplication::font().pixelSize() - 2`, and compare each widget's height with `QFontMetrics(font).height()`; then `setUiScalePercent(75)`, process events, and repeat | Both at 100 % and at 75 % the bar and every readout use the application font minus two pixels and each readout is at least as tall as its font metrics | `tst_ui_scale::statusBarTextFollowsScale` |
| `uis_canvas_unaffected` | Open a 256×256 document, record the canvas zoom and size, then `setUiScalePercent(200)` | The zoom and the canvas size are unchanged | `tst_ui_scale::canvasUnaffectedByScale` |
| `uis_persist` | Set the scale to 130 %, reload the session, and rebuild the frame | The stored and restored percentages are both 130 | `tst_ui_scale::sessionPersistsScale` |
