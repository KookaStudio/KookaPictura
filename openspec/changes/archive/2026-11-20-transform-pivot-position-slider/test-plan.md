# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| The locator moves the pivot | Begin a Free Transform on a filled layer and click the top-left locator cell. | The pivot is the live quad's top-left corner and the quad is unchanged. | `tst_transform_options::pivotLocatorMovesTheReferencePoint` |
| `tob_pivot_highlight` | Click a locator cell, inspect its `pivotActive` property and the centre cell's, then drag the pivot between anchors. | The clicked cell is active and the centre is not; a pivot between anchors leaves no cell active. | `tst_transform_options::pivotLocatorMovesTheReferencePoint` |
| `tob_xy_moves_target` | Type a new X and then Y in absolute mode, check the live quad, then repeat with relative positioning on. | The target translates so the reference point is at the typed position before Apply; relative values are offsets from the fixed session origin. | `tst_transform_options::xAndYTranslateTheTarget`, `tst_transform_options::relativePositioningOffsetsFromTheOrigin`, `pictura-app::translate_to_moves_a_similarity_quad` |
| Apply commits and records one state | Change a field and click Apply. | The session ends, one labelled state is recorded, and the tool page returns. | `tst_transform_options::applyCommitsCancelRestores` |
| `tob_apply_border` | Change a selection-border transform and click Apply. | One `"Transform Selection"` state is recorded and the tool page returns. | `tst_transform_selection::applyRecordsTransformSelection` |
| Cancel restores the document | Change a field and click Cancel. | The session ends and the document is bit-identical to the pre-session value. | `tst_transform_options::applyCommitsCancelRestores` |
| The interpolation combo reaches the session | Choose Nearest Neighbor in the interpolation combo. | The session reports `nearest`. | `tst_transform_options::interpolationComboReachesTheSession` |
| `tob_scale_slider_split` | Set the W popup slider to its midpoint, a quarter, and its maximum. | W is 100% at the midpoint, about 50% at a quarter, 1000% at the maximum; H has the same 1000 maximum. | `tst_transform_options::widthSliderIsSplitAtHundredPercent` |
