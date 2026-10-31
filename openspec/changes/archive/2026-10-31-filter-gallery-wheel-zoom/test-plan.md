# Test Plan

One row per spec scenario. `Coverage` names the automated test or `manual:` with
a reason. New tests are written before the implementation task that makes them
pass.

## imaging/filter-gallery

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `fg_zoom_preset` | Pick 200 % from the drop-down | The preview zooms to 200 % and the box shows "200%" | `tst_filter_gallery::zoomDropdownAndWheel` |
| `fg_zoom_wheel` | Turn the wheel up over the preview | The zoom steps to the next level up | `tst_filter_gallery::zoomDropdownAndWheel` |
| `fg_zoom_wheel_anchor` | At 200 %, pan, then turn the wheel up at a point away from the viewport centre | The document point under that point is unchanged (within one pixel) | `tst_filter_gallery::wheelZoomAnchorsAtTheCursor` |
| `fg_zoom_zoomout_centre` | After zooming in, turn the wheel down | The document point at the viewport centre is unchanged (within one pixel) | `tst_filter_gallery::wheelZoomAnchorsAtTheCursor` |
| `fg_zoom_centre_anchor` | Pan, then zoom with the −/+ buttons and the drop-down | The document point at the viewport centre is unchanged (within one pixel) | `tst_filter_gallery::buttonsAndDropdownAnchorAtTheCentre` |
| `fg_zoom_named` | Choose Actual Pixels, then Fit in View | 100 %, then the whole picture fits the pane | `tst_filter_gallery::zoomDropdownAndWheel` |

## Audit (evidence, not a pass/fail gate)

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| Sibling preview canvases | Read the wheel and `setZoom` paths of `LiquifyCanvas`, `AwaCanvas`, `VpCanvas`, `LensCorrectionCanvas` | Each anchors the wheel at `event->position()` and the buttons/drop-down at the widget centre, so no change is needed | manual: a code audit; the behaviour is already covered by each dialog's own suite |
