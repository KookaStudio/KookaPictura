# Test plan for Levels canvas controls

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| lev_pick_endpoint | Edit an input, configure non-default neutral targets, pick colored patches with black and white, and cancel; include boundary samples. | Prior edits are replaced by input endpoints and output floor or ceiling; samples reach their targets, and Cancel restores the document. | tst_levels_canvas_controls::levelsPickEndpoints |
| lev_pick_gray | Pick colored midtones with default and custom gray targets, including a gamma-limit case, then accept. | Channels approach the target subject to gamma limits, and accepting creates a single history entry. | tst_levels_canvas_controls::levelsPickGray |
| lev_pick_limits | Open the right-click menu and select every size; sample an averaged patch and a 101×101 neighborhood clipped by the image edge, then reject transparent, out-of-range endpoint and gray-endpoint picks with Preview on, including a pending preview; open on Grayscale. | Point is the default and all seven sizes are offered; the averages are used, rejected picks preserve parameters and restore Preview, and gray point is disabled in Grayscale. | tst_levels_canvas_controls::levelsPickLimits |
| lev_clip_drag | Alt-drag both input endpoints on a known patch, release Alt and the mouse, repeat with Preview off, and drag on a large fit-to-window image with Cache Levels set to two. | The temporary maps show the expected channel colors and clear without losing the slider edit, including when the display pyramid is too shallow. | tst_levels_canvas_controls::levelsAltDragClips |
| lev_clip_other | Drag gamma and output with Alt and an input endpoint without Alt. | No clipping map appears. | tst_levels_canvas_controls::levelsOtherDragsStayNormal |
