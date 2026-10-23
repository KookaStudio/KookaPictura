# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| uis_default | Read `Theme::uiScalePercent()` and `Theme::scaled(12)` on a fresh application. | 100 percent; 12 px remains 12 px. | `tst_ui_scale::themeScaleContract` |
| uis_normalise | Call `Theme::normalizedUiScalePercent(99)`, `(84)`, `(250)`. | 100, 80, 200. | `tst_ui_scale::themeScaleContract` |
| uis_scaled_pixels | Set 75 percent and scale 1, 10, 12. | Rounded products, each at least 1. | `tst_ui_scale::themeScaleContract` |
| uis_live_font_qss | Set 100, capture font size and QSS menu padding, set 200, compare. | Font doubles; the `QMenuBar::item` padding doubles. | `tst_ui_scale::fontAndStyleSheetFollowScale` |
| uis_live_chrome | Read status-bar height, tool slot size, corner icon size, and panel minimum width at 100 and 150 percent. | Each grows by the scale. | `tst_ui_scale::frameChromeFollowsScale` |
| uis_canvas_unaffected | Open a document, record zoom, change the scale, read zoom and canvas size. | Unchanged. | `tst_ui_scale::canvasUnaffectedByScale` |
| uis_persist | Set 130, save the session, load it again. | 130 percent. | `tst_ui_scale::sessionPersistsScale` |
| uis_menu_step | Dispatch Enlarge at 100 and Decrease at 90. | 110 and 80. | `tst_ui_scale::windowMenuStepsAndBounds` |
| uis_menu_bounds | Set 75, 200, 100 and inspect the three actions. | Decrease, Enlarge, Reset disabled respectively. | `tst_ui_scale::windowMenuStepsAndBounds` |
| uis_menu_readout | Set 130 and read the readout action label. | `UI Size: 130%`. | `tst_ui_scale::windowMenuReadout` |
| uis_pref_list | Open the Interface page and enumerate the UI Size combo. | 75, 80, 90, 100, 110 … 200. | `tst_ui_scale::preferencesMirrorScale` |
| uis_pref_sync | Change from the combo, then dispatch Enlarge; read the combo again. | Both surfaces show the resulting percentage. | `tst_ui_scale::preferencesMirrorScale` |
| bfn_app_font | Read `QApplication::font()` at 100 percent. | `Noto Sans`, 12 px. | `tst_ui_scale::fontAndStyleSheetFollowScale` |
| bfn_font_scale | Set 150 and read the application font. | 18 px. | `tst_ui_scale::fontAndStyleSheetFollowScale` |
| bfn_chrome_size | Set 200 and apply the tab-bar font; compare with the application font. | Two pixels under. | `tst_ui_scale::fontAndStyleSheetFollowScale` |
