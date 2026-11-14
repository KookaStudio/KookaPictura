# Test Plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ts_menu_enabled` | Open a document, make a selection, open the Select menu | `Transform Selection` is enabled | `tst_transform_selection::menuEnabledWithSelection` |
| `ts_menu_no_selection` | Open a document with no selection (and an all-zero selection) | `Transform Selection` is disabled and invoking it changes nothing | `tst_transform_selection::menuDisabledWithoutSelection` |
| `ts_menu_modal` | Begin a Free Transform, then open the Select menu | `Transform Selection` is disabled and the pending session survives | `tst_transform_selection::menuDisabledWhileModal` |
| `ts_begin` | Select a block and invoke `Select > Transform Selection` | Box spans the selection bounds, the transform page is shown, history is unchanged | `tst_transform_selection::beginPlacesTheBoxOnTheSelectionBounds` |
| `ts_scale_shift` | Drag a corner handle with Shift, commit | The committed selection bounds match the proportional drag within 1 px | `tst_transform_selection::scaleWithShiftCommitsProportionally` |
| `ts_width_200` | Enter W = 200 % with the link on, apply | Committed width and height are twice the original within 1 px | `tst_transform_selection::widthFieldDoublesTheBorder` |
| `ts_rotate_pivot` | Set the pivot to the top-left locator, rotate 90°, apply | The source top-left corner stays within 1 px of the pivot | `tst_transform_selection::rotationKeepsTheReferenceCorner` |
| `ts_commit_state` | Scale the border and commit | Selection equals the resampled border and one `Transform Selection` state exists | `tst_transform_selection::commitRecordsOneStateAndReplacesTheBorder`, `pictura-app::selection_transform_commit_replaces_the_mask` |
| `ts_pixels_untouched` | Commit a border transform over a painted layer | Every layer channel byte is unchanged | `tst_transform_selection::commitLeavesPixelsUntouched`, `pictura-app::selection_transform_keeps_layer_bytes` |
| `ts_undo` | Commit a border transform, undo once | Coverage equals the pre-transform mask byte for byte | `tst_transform_selection::undoRestoresThePreviousBorder`, `pictura-app::selection_transform_undo_restores_the_mask` |
| `ts_identity` | Open the session and apply without moving the box | Selection unchanged, no history state | `tst_transform_selection::identityCommitRecordsNothing`, `pictura-app::selection_transform_identity_records_nothing` |
| `ts_off_canvas` | Drag the box fully off the canvas, commit, undo | Empty selection, one state, undo restores the previous coverage | `tst_transform_selection::offCanvasCommitClearsTheSelection`, `pictura-render::transform_selection_clips_to_canvas` |
| `ts_soft` | Feather a selection, scale it, commit | Boundary coverage keeps values strictly between 0 and 255 | `tst_transform_selection::softCoverageStaysSoft`, `pictura-render::transform_selection_keeps_soft_coverage` |
| `ts_escape` | Begin a border transform, gesture, press Escape | Session ends, selection byte-identical, no state added | `tst_transform_selection::escapeRestoresTheBorder`, `pictura-app::selection_transform_cancel_keeps_the_mask` |
| `ts_preview_follows` | Drag a handle while watching the canvas | The transformed ants follow the pointer, the old border is hidden, history is unchanged | `tst_transform_selection::previewFollowsTheDrag`, `pictura-app::selection_transform_contour_follows_the_box` |
| `ts_contour_50` | Commit a rotated border and inspect the committed contour | The outline sits on the resampled mask's 50 % line within 0.5 px | `tst_transform_selection::committedOutlineMatchesTheMask` |
| `ts_preview_only` | Gesture several times, then cancel | Selection and history equal their pre-session values | `tst_transform_selection::cancelKeepsHistoryAndSelection` |
| `ts_warp_enter` | Begin a border transform, click the Warp button | Mesh spans the selection bounds, warp fields show, border unchanged | `tst_transform_selection::warpEntersOnTheSelection`, `pictura-app::selection_transform_warp_preview_and_commit` |
| `ts_warp_edit` | Choose a warp preset, then drag an anchor and the surface | The border preview follows the warp, no history state | `tst_transform_selection::warpPresetAndDragReshapeThePreview` |
| `ts_warp_commit` | Warp the border, apply | Selection equals the warped coverage, one `Transform Selection` state, pixels unchanged | `tst_transform_selection::warpCommitResamplesTheMask`, `pictura-render::warp_selection_moves_coverage`, `pictura-app::selection_transform_warp_commit` |
| `ts_warp_leave` | Warp, then leave Warp mode | The border preview does not move and the box hugs the warped bounds | `tst_transform_selection::warpLeaveKeepsTheBorder`, `pictura-render::warp_dest_rect_matches_the_mask` |
| `ts_warp_compose` | Apply Distort, enter Warp, warp, apply | The committed border is the mesh warp followed by the box frame | `tst_transform_selection::warpComposesWithPerspective` |
| `ts_distinct_free` | With a selection, press `Ctrl+T` and commit a scale | Selected pixels move and the selection is dropped | `tst_transform_selection::ctrlTStillLiftsPixels` |
| `ts_distinct_border` | With the same selection, use `Select > Transform Selection` | Only the border changes and the selection stays active | `tst_transform_selection::borderTransformKeepsTheSelection` |
| `ts_lock_select` | Pending border transform, open the Select menu | Deselect, Inverse, and Modify are disabled | `tst_transform_selection::selectMenuLockedWhilePending` |
| `ts_prompt` | Switch tools with Apply / Don't Apply / Cancel | Apply commits the border, Don't Apply cancels it, Cancel keeps it | `tst_transform_selection::toolSwitchPromptsForTheBorder` |
| `sm_surface` | Open the Select menu with a document | Transform Selection is present and live | `tst_transform_selection::menuEnabledWithSelection`, `pictura-script::golden_catalogue_matches_the_captured_table` |
| `sm_transform_selection` | Open the Select menu with no selection | Transform Selection is visible and disabled | `tst_transform_selection::menuDisabledWithoutSelection` |
| `tc_again` | Rotate 90° Clockwise, then Again | 180° total, second state `Transform Again` | `tst_transform_commands::againRepeatsTheLastTransform` |
| `tc_again_disabled` | Document open, no transform committed | Again is disabled | `tst_transform_commands::againIsUnavailableBeforeATransform` |
| `tc_again_pivot` | Move, set the pivot to the box top-left, commit, Again | The repeat keeps the pivot on the box top-left | `tst_transform_commands::againUsesTheSourcePivot` |
| `tc_again_selection` | Commit a 90° border rotation, then Again | Border rotated 180°, selection target kept, second state `Transform Again` | `tst_transform_selection::againRepeatsABorderTransform` |
| `tc_warp_open` | Pending border scale, invoke `Edit > Transform > Warp` | The same session enters Warp with its box and scale intact | `tst_transform_selection::warpKeepsAnOpenBorderSession` |
| Beginning a session shows the transform page | Begin Free Transform while the Brush is active | The transform page is shown | `tst_transform_options::pageSwitchesWithTheSession` |
| Ending a session restores the tool page | Commit or cancel the session | The tool page returns | `tst_transform_options::pageSwitchesWithTheSession` |
| The fields seed from the session | Begin on a 100×50 layer | X/Y are the layer centre, W/H 100 %, angle 0° | `tst_transform_options::fieldsSeedFromTheSession` |
| `tob_seed_border` | Begin on a 100×50 selection | X/Y are the selection centre, W/H 100 %, angle 0° | `tst_transform_selection::fieldsSeedFromTheSelectionBounds` |
| The locator moves the pivot | Click the bottom-right locator | The pivot equals the live quad's bottom-right corner | `tst_transform_options::pivotLocatorMovesTheReferencePoint` |
| Apply commits and records one state | Change a field, click Apply | Session ends, one state labelled for the target, tool page returns | `tst_transform_options::applyCommitsCancelRestores` |
| `tob_apply_border` | Change a border field, click Apply | One `Transform Selection` state, tool page returns | `tst_transform_selection::applyRecordsTransformSelection` |
| Cancel restores the document | Change a field, click Cancel | Document is bit-identical to its pre-session value | `tst_transform_options::applyCommitsCancelRestores` |
| The interpolation combo reaches the session | Choose Nearest Neighbor | The session reports `nearest` | `tst_transform_options::interpolationComboReachesTheSession` |
