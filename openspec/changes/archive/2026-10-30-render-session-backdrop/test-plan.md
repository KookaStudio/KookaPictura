# Test Plan

One row per spec scenario. `Coverage` names the automated test or `manual:` with
a reason. New tests are written before the implementation task that makes them
pass.

## compositing/render-session

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `rs_preview_region` | Present a canvas-sized adjustment preview while the canvas reports level 2 and a visible rect smaller than the document | Only that rect at level 2 is composited; no pixels outside it are composited at full resolution | `pictura-app::preview_patches_only_the_presented_level`, `pictura-app::preview_patches_the_presented_stored_level` |
| `rs_preview_level0` | Present the same preview at level 0 over a known rect | The presented rect equals the same rect of `composite_rgba` | `pictura-render::level0_preview_equals_the_full_region` |
| `rs_preview_approximate` | Present at level 2 and compare with level 2 of the exact composite | The frames may differ; the commit then replaces the level | `pictura-render::reduced_preview_need_not_equal_the_exact_level` |
| `rs_preview_display_only` | Read `doc.composite`, `level0`, and the stored levels before and after a presented preview | `doc.composite` is unchanged; `level0` and the stored levels are unchanged outside the presented rectangle | `pictura-app::preview_leaves_the_composite_unchanged` |
| `rs_commit_exact` | Preview an adjustment, then commit | `doc.composite == composite_rgba(doc)`, every level equals a rebuild, one history state | `pictura-app::commit_produces_the_exact_frame` |
| `rs_commit_noop` | Drag the value away and back to the value before the gesture, then commit | The history count does not increase; composite unchanged | `pictura-app::unchanged_preview_records_nothing`, `pictura-app::noop_round_trip_records_nothing` |
| `rs_backdrop_exact` | Resume a root-level target from a cached prefix and compare with `composite_rgba` for every blend mode, clipping run, and knockout case in the `document_oracle`, `clipping_oracle`, and `knockout_oracle` suites | Byte-identical, including blend modes, clipping and knockout fixtures | `pictura-render::backdrop_plus_suffix_equals_full_composite`, `pictura-render::backdrop_matches_document_oracle`, `pictura-render::backdrop_matches_clipping_oracle`, `pictura-render::backdrop_matches_knockout_oracle` |
| `rs_backdrop_reuse` | Run two preview frames with an unchanged prefix | The second frame reuses the prefix and equals the first | `pictura-render::second_frame_reuses_the_backdrop` |
| `rs_invalidate_pixel` | Edit a layer below the target's pixels, then preview | Prefix digest changes; result equals the full composite | `pictura-render::prefix_digest_changes_on_a_pixel_edit` |
| `rs_invalidate_property` | Change opacity, blend, fill, or visibility below the target | Prefix digest changes; result equals the full composite | `pictura-render::prefix_digest_changes_on_a_property_edit` |
| `rs_invalidate_reorder` | Reorder layers, then preview | Prefix digest changes; backdrop rebuilt | `pictura-render::prefix_digest_changes_on_a_reorder` |
| `rs_invalidate_above` | Change only the target or a layer above it | Prefix digest is unchanged; backdrop reused | `pictura-render::prefix_digest_ignores_edits_at_or_above` |
| `rs_resume_clipping` | Target a clipped layer | Full path taken; result equals the full composite | `pictura-render::clipped_target_falls_back_to_full` |
| `rs_resume_nested` | Target a layer inside a group | Full path taken; result equals the full composite | `pictura-render::nested_target_falls_back_to_full` |
| `rs_resume_knockout` | Put a knockout layer above the target and assert the resume path is taken (no full composite ran) | The resume path is taken with the bottom-layer-alone base and the result equals the full composite | `pictura-render::knockout_above_the_resume_point_is_exact` |
| `rs_resume_valid` | Target a non-clipped root layer in an RGB 8-bit document | Resumed result equals the full composite | `pictura-render::root_level_target_resumes_and_matches_full` |
| `rs_coalesce` | Emit several slider values within one frame from the Properties panel | At most one preview composite; the frame shows the latest value | `tst_properties_preview::adjustmentDragCoalescesPreview` |
| `rs_coalesce_commit` | Coalesce previews, then release the slider before the timer fires and commit | The committed state and history reflect the latest value | `tst_properties_preview::commitUsesTheLatestValue`, `tst_properties_preview::commitFlushesThePendingPreview` |
| `rs_fallback_no_level` | Preview on a document whose pyramid has only level 0 | Exact path runs; frame equals the full composite | `pictura-app::no_stored_level_falls_back_to_exact` |
| `rs_fallback_budget` | Request a preview whose backdrop fits but backdrop plus scratch exceeds the budget | No backdrop cached; exact path runs | `pictura-render::over_budget_backdrop_is_refused` |
| `rs_fallback_state` | Solo a mask, then preview; repeat with a live stroke | Exact path runs; the displayed frame is the state's frame | `pictura-app::solo_mask_and_live_stroke_fall_back` |
| `rs_fallback_content` | Preview a stack containing a live type layer, a proxy-less smart object, or a vector fill | The preview and the commit use the exact path and equal the full composite | `pictura-render::unreducible_content_falls_back_to_full` |
| `rs_fallback_empty` | Preview with an empty visible rectangle | No preview is presented and no damage is marked | `pictura-render::empty_rect_is_a_noop` |
| `rs_fallback_no_presented_level` | Preview when the canvas reports no presented level | The exact path runs; the frame equals the full composite | `pictura-render::no_presented_level_falls_back_to_exact` |

## compositing/current-picture

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `cp_after_edit` | Commit a pixel edit, then request the picture | The document composite is served | `pictura-app::picture_is_served_after_a_committed_edit` |
| `cp_after_preview` | Present a preview that updates the composite, then request the picture | The document composite is served | `pictura-app::picture_is_served_after_a_preview` |
| `cp_after_history` | Undo/redo/jump, then request the picture | The restored composite is served | `pictura-app::picture_is_served_after_history_navigation` |
| `cp_layer_stroke` | Request the picture with a layer stroke live | A fresh stack composite is served | `pictura-app::a_live_layer_stroke_is_composited` |
| `cp_mask_stroke` | Request the picture with a mask stroke live | A fresh stack composite is served | `pictura-app::a_live_mask_stroke_is_composited` |
| `cp_display_preview` | Request the picture with a display-only preview live | A fresh stack composite is served, not the pre-preview composite | `pictura-app::a_display_preview_is_composited` |
| `cp_lifted_transform` | Request the picture with a lifted transform live | A fresh composite of the lifted stack is served | `pictura-app::a_lifted_transform_is_composited` |
| `cp_non_rgb` | Request the picture for a Grayscale document | A four-channel stack composite is served | `pictura-app::a_grayscale_document_is_composited` |
| `cp_size_mismatch` | Request the picture with a mismatched composite | A fresh composite at the document size is served | `pictura-app::a_mismatched_composite_is_composited` |
| `cp_layerless` | Request the picture for a document with no layers | The embedded composite is served | `pictura-app::a_layerless_document_keeps_its_embedded_composite` |

## compositing/described-damage

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| A filter on a bounded layer updates only its rectangle | Apply a filter to a bounded pixel layer | Only that rect composites; result equals a full recomposite | `pictura-app::bounded_edits_equal_full_recomposite` |
| An opacity change on a bounded layer updates only its rectangle | Change a bounded layer's opacity | Only that rect composites; result equals a full recomposite | `pictura-app::layer_opacity_region_refresh_equals_a_full_recomposite` |
| `dd_preview_region` | Drag a bounded layer's opacity with a composite-updating preview | Only that rect composites; result equals a full recomposite | `tst_layers_preview_coalescing::opacityPreviewUsesTheRegionPath` |
| `dd_canvas_region` | Change a canvas-sized layer's opacity with a composite-updating change | The region path is used; no full-document recomposite | `tst_layers_preview_coalescing::canvasSizedOpacityUsesTheRegionPath` |
| `dd_display_preview` | Preview a canvas-sized adjustment display-only, then commit | The preview does not update the composite; the commit follows the committed change's bound and equals a full recomposite | `pictura-app::preview_leaves_the_composite_unchanged`, `pictura-app::display_preview_commit_follows_the_bound` |
| A paste updates only the pasted rectangle | Paste an image at a bounded rect | Only that rect composites; result equals a full recomposite | `tst_edit_clipboard::editClipboard` |
| A paint commit updates only the stroke's rectangle | Commit a paint stroke | Only the stroke rect composites; result equals a full recomposite | `pictura-app::paint_commit_region_patch_equals_a_full_recomposite` |
| A transform commit updates the union of source and destination | Commit a layer move | The union rect composites; result equals a full recomposite | `pictura-app::layer_move_region_refresh_equals_a_full_recomposite` |
