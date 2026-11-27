# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| pt_select | Shift-click components and anchors across components, toggle one, then ordinary-click another. | Selection and overlay follow toggles and replacement; geometry and history stay unchanged. | `tst_path_selection_tools::multiSelectToggle` |
| pt_marquee | Marquee-select components and anchors across components. | Enclosed items are visibly selected without edits or history. | `tst_path_selection_tools::multiSelectMarquee` |
| pt_clear | Select items; clear with empty click, tool switch, target replacement with matching indices, and undo/refresh. | Selection clears without history or stale targets. | `tst_path_selection_tools::multiSelectClear` |
| pt_affine | Scale, rotate and skew multiple curved components using gestures, numeric inputs and offset pivot; inspect preview and undo/redo. | Outline and coordinates match each affine map; other geometry stays fixed; one state per commit. | `tst_path_component_transform::affineControls`, `tst_path_component_transform::fractionalBounds`, `tst_path_component_transform::multiComponentControls` |
| pt_nonlinear | Distort, perspective-transform and warp curved components and undo/redo. | Control points match the mode map; unselected geometry stays fixed; one state per commit. | `tst_path_component_transform::projectiveWarp`, `pictura-app::fractional_warp_uses_the_path_source_frame` |
| pt_targets | Transform Work, saved, shape and vector-mask targets including a live shape; undo/redo. | Selected paths and display update; mask flags and unrelated pixels persist; live metadata is restored on undo. | `tst_path_component_transform::targetsAndMetadata` |
| pt_cancel | Preview then cancel; commit identity and attempt invalid map, including a live shape. | Geometry, mask flags, live metadata and history remain unchanged. | `tst_path_component_transform::cancelIdentityInvalid`, `tst_path_component_transform::targetsAndMetadata` |
| pt_enable | Exercise missing selection, degenerate bounds, locked/uneditable target and active modal session. | Commands are disabled and session ownership remains exclusive. | `tst_path_component_transform::availability` |
| pt_target_change | Change target/document or undo during preview. | Original target restored, stale selection cleared and no state added. | `tst_path_component_transform::targetChangeCancels` |
| pt_again_path | Commit rotation with offset pivot; repeat on multiple selected components and undo/redo. | Source-frame pivot convention and one-state history hold. | `tst_path_component_transform::repeatComponents` |
| pt_again_points | Repeat on anchors across components; undo/redo. | Only selected anchors and handles move; history restores them. | `tst_path_component_transform::repeatPoints` |
| pt_again_enable | Try incompatible selections and modal/locked targets; cancel, identity-commit and non-repeatable commit after a valid record. | Gating is correct; previous record survives. | `tst_path_component_transform::repeatAvailability` |
| pt_again_document | Give two documents different records; switch, close and replace documents. | Each document uses its own record; discarded records cannot leak. | `tst_path_component_transform::repeatDocuments` |
