# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| bfn_menu_bar_font | Build a frame at 100 percent; read the menu bar font and the Window menu font. | Both resolve to `Noto Sans` at 12 pixels and match the application font. | `tst_ui_scale::menuBarUsesBundledFont` |
| bfn_menu_bar_scale | Set the UI scale to 150 percent; read the menu bar and Window menu fonts. | Both resolve to `Noto Sans` at 18 pixels. | `tst_ui_scale::menuBarUsesBundledFont` |
