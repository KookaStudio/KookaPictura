# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| ds_menu_rounded | Apply the theme, show a `QMenu` with one icon action, force the first action selected, and grab the popup image. | The pixel at the top-left corner is transparent, so the rounded outline and inset keep the selected row out of the corner. | `tst_design_system::menuPopupChrome` |
| ds_menu_icon_padding | Render a `QMenu` whose first action carries a solid red 16 px icon; scan the grabbed image for the leftmost red pixel. | The icon's left edge is at least 8 px from the popup's left edge at 100 percent scale; the check-mark rule uses the same offset. | `tst_design_system::menuPopupChrome` |
