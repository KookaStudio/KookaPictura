# Test Plan

One row per scenario in the delta spec. Every row is automated.

## imaging/blur-gallery

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `blur_scrollbars_appear` | Open the workspace at fit, then zoom to four times the fit. | Both scrollbars are hidden at fit and shown with a positive range when zoomed. | `tst_blur_gallery_dialog::scrollbarsAndMiddleDragPan` |
| `blur_scrollbars_scroll` | At the zoomed view, set both scrollbars to their maximum. | The picture's rect moves left and up. | `tst_blur_gallery_dialog::scrollbarsAndMiddleDragPan` |
| `blur_middle_drag_pans` | Reset both scrollbars and drag with the middle button left and up. | Both scrollbar values rise, so the picture pans right and down. | `tst_blur_gallery_dialog::scrollbarsAndMiddleDragPan` |
