# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vmask_current_saved` | Draw a square, save it as a path, keep that row selected, then run Layer ▸ Vector Mask ▸ Current Path on a pixel layer. Separately, author a mask from given subpaths in the engine. | The layer gains a vector mask whose outline is the saved square; the composite hides pixels outside it. | `tst_layers_mask_commands::currentPathUsesTheSelectedSavedPath`, `pictura-render::path_kind_authors_the_given_subpaths` |
| `vmask_current_none` | With a non-empty Work Path but no row selected (`paths_set_active(-2)`), refresh the registry and dispatch Current Path. | The action is disabled and no vector mask is added. | `tst_layers_mask_commands::currentPathNeedsASelectedPath` |
| `vmask_row_listed` | Add a Reveal All vector mask to a pixel layer, refresh the Paths panel, then select a second layer without a mask. | The panel lists `<layer> Vector Mask`, then no longer lists it. | `tst_paths_panel::vectorMaskRowFollowsTheLayer` |
| `vmask_row_edit` | Activate the mask row, move anchor 0 of subpath 0 with `path_move_anchor`, commit the drag, then undo. | The mask outline's anchor moves, the composite at the vacated corner changes, one history state is recorded, and undo restores the anchor. | `tst_paths_panel::vectorMaskRowEditsTheMask` |
| `vmask_pen_adds` | Activate the mask row and draw a closed triangle with `path_append_corner`, `path_commit_anchor`, and `path_close`. | The mask gains one closed subpath; `paths_has_work_path` stays false. | `tst_paths_panel::penDrawsIntoTheVectorMask` |
| `vmask_saved_unaffected` | Keep a masked pixel layer selected, activate a saved path row, turn the layer target on as Path Selection does, and move subpath 0. | The saved path moves; the mask outline is unchanged. | `tst_paths_panel::savedPathKeepsPriorityOverTheMask` |
| `vmask_delete_row` | Activate the mask row and press Delete Path. | The layer has no vector mask and the row is gone. | `tst_paths_panel::deletingTheVectorMaskRow` |
| `vmask_flags_kept` | Author a Hide All mask, set it disabled and unlinked, then rewrite its outline with `set_layer_shape_paths`. | The flags word still has invert, disabled, and not-linked set. | `pictura-render::rewriting_the_outline_keeps_the_mask_flags` |
| `vmask_thumb_selects_row` | Click a masked pixel layer's vector-mask thumbnail, then click it again. | `paths_active` is `-4`, then `-2`. | `tst_layers_mask_commands::vectorThumbnailSelectsTheMaskPath` |
| `vmask_thumb_outline` | Render the thumbnail of a mask revealing a centred square. | The pixel inside is white, the one outside is grey, and the one on the edge is dark. | `pictura-app::vector_mask_thumbnail_strokes_the_outline` |
