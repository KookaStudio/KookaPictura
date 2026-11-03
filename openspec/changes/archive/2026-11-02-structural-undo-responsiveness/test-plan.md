# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `iph_undo_damage` | Stroke, undo | Bytes match, rectangle covers the tiles | `pictura-app::crossing_a_segment_reports_the_changed_tile` |
| `iph_geometry_damage` | Resize, undo | Whole document | `pictura-app::undoing_a_resize_reports_the_whole_document` |
| `iph_structure_damage` | Structure change with a changed tile, undo | Rectangle covers it | `pictura-app::crossing_a_segment_reports_the_changed_tile` |
| `hrr_mask_region` | Edit a mask, undo, redo | Two-tile rectangle, bytes match | `pictura-app::undoing_a_mask_edit_reports_its_tiles_in_document_coordinates` |
| `hrr_mask_moved` | Move a mask, edit it, undo twice | `Composite`, bytes match | `pictura-app::a_moved_mask_reports_the_composite` |
| `hrr_mask_walk` | Move a mask and back, edit, jump | `Composite` | `pictura-app::a_walk_across_a_mask_move_reports_the_composite` |
| `hrr_mask_delete` | Delete a masked layer, undo | `Composite`, not whole | `pictura-app::undoing_a_masked_delete_stays_bounded` |
| `hrr_mask_solo` | Solo a mask, restore across a mask move | Solo source shows the restored mask | `pictura-app::a_composite_restore_rebuilds_the_soloed_mask` |
| `hrr_channel_empty` | Edit a channel, undo | Empty region, bytes match | `pictura-app::a_document_channel_edit_reports_no_rectangle` |
| `hrr_channel_crossing` | Resize a layer with a channel change, undo | Region of the composite tile | `pictura-app::crossing_a_segment_with_a_changed_channel_stays_bounded` |
| `hrr_no_pixel_reorder` | Reorder equal layers, undo | Empty region | `pictura-app::undoing_a_reorder_of_equal_layers_reports_no_rectangle` |
| `hrr_no_pixel_frame` | Add a neutral layer, undo | Frame revision unchanged | `tst_large_document::aNeutralLayerUndoKeepsTheFrame` |
| `hrr_jump_oracle` | Jump across structural, geometry and mask edits | Composite equals a full composite | `pictura-app::a_jump_lands_on_an_exact_stored_composite` |
| `hrr_jump_then_undo` | Jump, stroke, undo | Region | `pictura-app::an_undo_after_a_whole_jump_stays_bounded` |
| `hrr_coarse_levels` | Build coarse and exact pyramids | Sizes match, first level holds corner pixels | `pictura-render::coarse_pyramid_matches_the_exact_levels` |
| `hrr_deferred_lands` | Resize a large document, undo, wait | Coarse first, `pyramidSettled`, then exact | `tst_large_document::aLargeGeometryUndoLandsTheExactPyramid`, `pictura-app::a_landed_pyramid_equals_a_rebuild` |
| `hrr_deferred_stale` | Land an old generation | Dropped, still deferred | `pictura-app::a_superseded_pyramid_is_dropped` |
| `hrr_deferred_settle` | Update while deferred | Exact, result dropped | `pictura-app::a_write_settles_the_deferred_pyramid` |
| `hrr_small_sync` | Restore a small document | Nothing deferred | `pictura-app::a_small_document_does_not_defer` |
| `hrr_pointer_manual` | On a real display, open a 16507×16196 document, resize it, then move the mouse across `Ctrl+Z`, a History panel jump, and undo of New Layer and a reorder | The ring keeps following the pointer, and the canvas sharpens within a moment | manual: offscreen Qt has no real pointer or compositor timing |
