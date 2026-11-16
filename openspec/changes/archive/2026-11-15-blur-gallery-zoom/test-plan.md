# Test Plan

One row per scenario in the delta spec. Every row is automated.

## imaging/blur-gallery

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `blur_zoom_field` | Open the workspace, read the bottom-left field, type `200%` and press Enter, then choose Fit in View. | The field opens on the fitted percentage, `200%` sets the preview to 200 %, and Fit in View restores the fit. | `tst_blur_gallery_dialog::zoomFieldWheelAndShortcuts` |
| `blur_zoom_wheel` | Send a wheel-up and then a wheel-down event to the preview canvas. | The preview zoom rises and then falls. | `tst_blur_gallery_dialog::zoomFieldWheelAndShortcuts` |
| `blur_zoom_shortcuts` | With the workspace active, press `Ctrl+=` and then `Ctrl+-`. | The preview zoom rises and then falls. | `tst_blur_gallery_dialog::zoomFieldWheelAndShortcuts` |
