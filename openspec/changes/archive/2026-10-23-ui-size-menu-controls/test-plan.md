# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| uis_menu_single_row | Build a frame and open the Window menu; inspect its actions and the row's layout. | One `uiSizeControl` row is present with the caption `UI size`; the minus/readout/plus share a row and reset sits on a lower row; the separate step rows are removed and the old readout action is hidden. | `tst_ui_scale::windowMenuSingleControl` |
| uis_menu_buttons | Open the Window menu, press the plus button twice and the minus once. | Scale goes 100 -> 120 -> 110; the menu remains visible. | `tst_ui_scale::windowMenuSingleControl` |
| uis_menu_resize | Open the Window menu, press the plus button, and pump events. | The menu grows to its size hint and the row stays inside the menu rect without clipping. | `tst_ui_scale::windowMenuResizesWithScale` |
| uis_menu_step | Dispatch Enlarge at 100 and Decrease at 90. | Scale becomes 110 then 80. | `tst_ui_scale::windowMenuStepsAndBounds` |
| uis_menu_bounds | Set scale to 75, 200, and 100; inspect button enablement. | Minus, plus, and reset are disabled at their bounds. | `tst_ui_scale::windowMenuStepsAndBounds` |
| uis_menu_readout | Set scale to 130; read the Window command text and row label. | Both show 130 percent. | `tst_ui_scale::windowMenuReadout` |
| uis_shortcut_bindings | Read the registry bindings for the three UI and three zoom commands. | UI commands use Ctrl+Alt; zoom commands keep Ctrl-only bindings. | `tst_ui_scale::uiScaleShortcuts` |
| uis_shortcut_step | Send Ctrl+Alt++ at 100, then Ctrl+Alt+0. | Scale becomes 110 then 100. | `tst_ui_scale::uiScaleShortcuts` |
| uis_tools_scroll | Raise the scale to 200 and pump events. | `toolsScroll` exists and the frame minimum height is below the screen height. | `tst_ui_scale::windowStaysResizableAtHighScale` |
| uis_window_resizable | At 200 percent, resize the frame to 800x500 and then 700x450. | The frame accepts both heights and has no maximum-size cap. | `tst_ui_scale::windowStaysResizableAtHighScale` |
| uis_menu_below_options | Build a frame, find the UI Size row's widget action and the Options action in the Window menu. | The row's action follows Options and every row after it is hidden, so the row is the last visible row. | `tst_ui_scale::windowMenuControlSitsBelowOptions` |
