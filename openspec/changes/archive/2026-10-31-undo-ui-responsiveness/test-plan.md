# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `hrr_undo_debounced` | Paint a stroke, let the panels settle, undo | Undo returns with one command refresh and the Histogram unchanged; it updates after the debounce | `tst_edit_refresh::anUndoLeavesThePanelsToTheDebounce` |
| `hrr_cancelled_session` | Commit a stroke, start another, undo mid-stroke | `sessionCancelled` fires and the Histogram is current on return | `tst_edit_refresh::anUndoThatCancelsAStrokeRefreshesPanelsAtOnce` |
| `hrr_tool_gesture` | Press with the Lasso, report a restore | The gesture preview is gone | `tst_edit_refresh::aHistoryRestoreDropsToolGestures` |
| `hrr_plane_sample` | Fork a band, read every sample | Logical values, no contiguous view built | `pictura-core::sample_reads_a_diverged_plane_without_materializing` |
| `hrr_thumbnail_no_materialize` | Build thumbnails from a diverged layer and mask | Materialization count stays zero | `pictura-app::thumbnails_of_a_diverged_layer_materialize_no_plane` |
| `hrr_thumbnail_pixels` | Compare diverged and flat copies, with gray and positioned thumbnails | Identical images | `pictura-app::thumbnails_of_a_diverged_layer_match_its_flat_copy` |
| `hrr_mask_region` | Edit a mask, undo, redo | Region of at most two tiles a side at the document position; bytes match | `pictura-app::undoing_a_mask_edit_reports_its_tiles_in_document_coordinates` |
| `hrr_mask_moved` | Move a mask, edit it, undo twice | The edit undo is bounded, the move undo is whole-document | `pictura-app::a_moved_mask_restores_everywhere` |
| `hrr_mask_walk` | Move a mask and back, edit, jump to the start | Whole-document restore | `pictura-app::a_walk_across_a_mask_move_restores_everywhere` |
| `hrr_channel_everywhere` | Edit a document channel, undo | Whole-document restore | `pictura-app::a_document_channel_edit_restores_everywhere` |
| Brush ring tracks the pointer | On a real display, open a 4000² document, paint, then move the mouse across `Ctrl+Z`, redo and History panel jumps | The ring keeps following the pointer | manual: offscreen Qt has no real pointer or compositor timing |
