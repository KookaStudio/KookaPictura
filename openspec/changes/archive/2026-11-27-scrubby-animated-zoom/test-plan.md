# Test plan

| Scenario | Steps and expected result | Coverage |
|---|---|---|
| Zoom in by click | Click the canvas with Zoom; magnification increases. | `tst_zoom_tool::plainClickZoomsInAtClick` |
| Zoom out by modified click | Alt-click the canvas; magnification decreases. | `tst_zoom_tool::altClickZoomsOutAtCentre` |
| lct_zoom_cursor | Click away from centre; the clicked image point stays fixed. | `tst_zoom_tool::plainClickZoomsInAtClick` |
| zta_zoom_out_centre | Alt-click or wheel out; viewport centre remains anchored. | `tst_zoom_tool::altClickZoomsOutAtCentre` |
| zta_marquee | Disable scrub, drag a rectangle and release; the marquee appears and its bounds fit the viewport. | `tst_zoom_tool::marqueeFillsViewport` |
| zta_click_steps | Press and release without moving; zoom changes once. | `tst_zoom_tool::plainClickZoomsInAtClick` |
| lct_wheel_shift | Wheel with Shift; zoom step doubles. | `tst_zoom_tool::retainedWheel` |
| lct_wheel_alt_pan | Wheel with Alt; canvas pans horizontally. | `tst_zoom_tool::retainedWheel` |
| lct_wheel_ctrl_alt_pan | Wheel with Ctrl and Alt; canvas pans vertically. | `tst_zoom_tool::retainedWheel` |
| lct_wheel_side_pan | Use horizontal wheel; canvas pans horizontally. | `tst_zoom_tool::retainedWheel` |
| lct_pan_margin | Pan beyond each edge; the required visible margin remains. | `tst_zoom_tool::retainedClamp` |
| lct_zoom_clamp | Zoom near an edge; the offset respects its clamp. | `tst_zoom_tool::retainedClamp` |
| lct_fit_within | Fit and centre a document; the image stays visible. | `tst_zoom_tool::retainedClamp` |
| scrub_direction | Enable scrub and drag right then left, including with a rotated view; zoom rises then falls about the press point. | `tst_zoom_tool::scrubDirection`, `tst_zoom_tool::rotatedAnchor` |
| scrub_limits_cancel | Reach each limit and reverse: zoom responds immediately. Cancel, deactivate or switch document/view mid-gesture; later moves do not update the old or replacement view. | `tst_zoom_tool::scrubLimitsAndCancel`, `tst_document_navigate::zoomGestureDocumentSwitch` |
| scrub_multiwindow | Scrub with Zoom All alone, Shift alone, and neither; only the first two fan out the effective clamped factor. With resize enabled the floating window resizes. | `tst_zoom_tool::scrubMultiwindow`, `tst_document_navigate::scrubZoomsAllWindows`, `tst_document_navigate::shiftScrubZoomsAllWindows`, `tst_document_navigate::scrubClampedAndLocal`, `tst_document_navigate::scrubResizesFloatingWindow` |
| scrub_history | Record pixels and history before and after a scrub; both remain unchanged. | `tst_zoom_tool::scrubLeavesHistory` |
| animated_hold | Enable the preference on a raster canvas and advance the hold delay and ticks deterministically for both directions; zoom changes about the correct anchor, including a rotated view, respects limits, fanout and resize, leaves history and pixels unchanged, and release adds no step. A short click still steps once. | `tst_zoom_tool::animatedHold`, `tst_zoom_tool::rotatedAnchor`, `tst_document_navigate::animatedZoomFanoutAndResize` |
| animated_fallback | Disable the preference or inject an unavailable capability with GPU compute available; hold gives one discrete step and no ticks, and unavailable preference explains its disabled state. Reopen General to check persistence. | `tst_zoom_tool::animatedFallback` |
| animated_interrupt | During a hold release, cancel, deactivate, close or switch document/view, or initiate zoom/pan; advance a queued tick and verify no old or replacement view changes. | `tst_zoom_tool::animatedHold`, `tst_zoom_tool::animatedInterrupt`, `tst_document_navigate::zoomGestureDocumentSwitch` |
| animated_drag | Move past the threshold before the delay with scrub on and off; scrub or marquee takes over, with no timed zoom or extra click step. | `tst_zoom_tool::animatedDrag` |
