# Test Plan

One row per scenario in the delta spec. Every row is automated.

## document/workspace-persistence

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| panels_tab_field_focus | With no active layer, show the Properties panel's Canvas section, focus the width spin box, type `64`, and press `Tab` through the shortcut map; then press `Shift+Tab`. | The document width is 64, focus is on the height spin box, then back on the width spin box, and the Tools panel and panel column stay visible throughout. | `tst_screen_modes::tabInFieldMovesFocus` |
| panels_tab_canvas_toggle | Focus the canvas and press `Tab` twice through the shortcut map. | The first press hides the Tools panel and the panel column; the second restores them. | `tst_screen_modes::tabOnCanvasTogglesPanels` |

## ui/delete-key

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| dk_after_field | Select a duplicated pixel layer, focus a line edit, then focus a plain `StrongFocus` widget and press Delete on it. | The Delete shortcut is disabled while the line edit has focus and enabled after, and the layer count drops to 1 with one new history state. | `tst_edit_shortcuts::deleteKeyActsAfterAFieldLosesFocus` |
