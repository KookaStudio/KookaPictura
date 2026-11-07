# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| qm_entry_clear | Enter without a normal selection and paint black. | Mask starts clear and accepts protection; no editing constraint is inherited. | `tst_quick_mask::entryWithoutPixelSelection` |
| qm_entry_selection | Enter from a rectangle selection. | Original bytes initialize mask coverage and its outline disappears. | `tst_quick_mask::entryWithPixelSelection`, `tst_quick_mask_regressions::entryHidesCachedAnts` |
| qm_entry_empty | Enter from an explicitly empty selection. | Whole mask is protected, distinct from implicit full coverage. | `tst_quick_mask::entryWithEmptySelection` |
| qm_cached_ants | Render an ordinary rectangle outline; enter Quick Mask through Q. | Actual canvas ants clear; mask keeps the original selection. | `tst_quick_mask_regressions::entryHidesCachedAnts` |
| qm_temporary_clip | Create a temporary rectangle or lasso, brush/fill/filter, deselect, exit. | Only constrained coverage changes; Q ignores the editing constraint. | `tst_quick_mask_regressions::editingSelectionOnlyClipsMask`, `pictura-app::quick_mask_clipping_keeps_coverage_independent`, `pictura-app::quick_mask_soft_clip_is_applied_once` |
| qm_geometry_reset | Resize, canvas-resize, rotate or crop while active, exit with Q, then undo. | Existing geometry resets clear both states; undo restores correctly sized mask coverage. | `tst_quick_mask_regressions::geometryResetDoesNotLeakStaleCoverage`, `pictura-app::geometry_selection_clear_discards_quick_mask_and_clip` |
| qm_keyboard_undo | Paint through canvas events, then keyboard undo/redo and grab the rendered canvas. | Visible overlay and history agree after each step. | `tst_quick_mask_regressions::keyboardUndoRepaintsMask` |
| qm_independent_history | Capture selection/paint changes, undo/redo before and after Q exit. | Mask and clip restore independently; temporary clip never becomes the final selection. | `pictura-app::quick_mask_independent_history`, `tst_quick_mask_regressions::editingSelectionHistorySurvivesExit` |
| qm_retain_strokes | Start from a rectangular mask and paint separate areas with actual brush gestures. | Old and untouched mask coverage survives; image is unchanged. | `tst_quick_mask_regressions::brushRetainsExistingMask` |
| qm_bounded_strokes | Watch change/damage signals and frame revision during several dabs; use fragmented coverage. | No full change/frame refresh during dabs; bounded damage and no plane materialization. | `tst_quick_mask_regressions::strokesUseBoundedPresentation`, `tst_quick_mask_regressions::zoomedOutStrokesPatchCache`, `pictura-app::quick_mask_presentation_avoids_materialization` |

The live control-server reproduction records pointer times and rendered samples before and after. No image golden changes or independent CS6 parity claim is planned.
