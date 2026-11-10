# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Open the brush picker on the canvas | Activate the Brush tool and route a canvas right-click. | A `brushPickerRequested` signal fires once. | `tst_canvas_context_menus::brushPickerOpens` |
| A preset chosen from the canvas applies | Open the picker from the Brush tool, force a different size, then choose preset 0. | A brush tip or size change signal fires and the tip reflects the preset. | `tst_canvas_context_menus::brushPresetApplies` |
| `ccm_stamp_open` | For each of Clone Stamp, Pattern Stamp, History Brush, and Art History Brush, activate the tool and route a canvas right-click. | The route returns true and one `brushPickerRequested` signal fires per tool. | `tst_canvas_context_menus::stampToolsOpenBrushPicker` |
| `ccm_stamp_apply` | For each stamp tool, open the picker, force a different size, then choose preset 0. | A size change signal fires and the tool's shared paint tip becomes the preset size. | `tst_canvas_context_menus::stampPresetApplies` |
| Picker look and placement on a real display | Open each stamp tool on a real X11/Wayland display, right-click over the canvas, and choose a preset. | The picker appears at the pointer with Size, Hardness, and the preset grid, and the next stroke uses the chosen tip. | manual: offscreen Qt Test verifies routing and tip state but not the rendered overlay or the next painted stroke on a real compositor. |
