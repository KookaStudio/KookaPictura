# Test plan — damage-module

One row per scenario in the change's spec deltas
(`specs/compositing/described-damage/spec.md`,
`specs/document/edit-transaction/spec.md`, and
`specs/document/edit-history/spec.md`). Coverage names the test that proves the
row; `manual:` is used only where the check cannot run in CI.

Tests named here are created by `tasks.md`; `scripts/check-test-plan.py` reports
only "no such test" until the implementation tasks land them.

Run rows with:

```bash
cargo nextest run -p pictura-render damage
cargo nextest run -p pictura-render layer_source
cargo nextest run -p pictura_app
cargo nextest run -p pictura_app history
cargo test -p pictura_app structural_delete_profile -- --ignored --nocapture
cargo test -p pictura_app stroke_capture_profile -- --ignored --nocapture
cargo test -p pictura_app mask_feather_preview_profile -- --ignored --nocapture
ctest --test-dir build -R '^tst_(damage_bounds|layer_style|layers_panel|layers_mask_commands|layers_smart_filters|merge_commands|align_distribute)$' --output-on-failure
```

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| A dimension change recomposites fully | Resize, crop, rotate, flip, or change the canvas size | The whole document is recomposited and the whole canvas is marked damaged | `pictura-app::dimension_change_recomposites_fully` |
| An unbounded adjustment recomposites fully | Apply an adjustment layer with an unbounded mask or a non-zero default color | The whole document is recomposited | `pictura-app::unbounded_adjustment_recomposites_fully` |
| A layer with an effect block recomposites fully | Edit a layer with an effect block inside a knockout/pass-through stack whose pixels cannot be confined to a rectangle | The whole document is recomposited | `pictura-app::unbounded_effect_recomposites_fully`, `pictura-app::knockout_or_passthrough_effect_is_unbounded` |
| `dd_effect_bounded` | Change a Layer Style value on a layer with a bounded effect block | Only the union of the before and after reach is recomposited | `pictura-app::effect_reach_is_bounded`, `tst_layer_style::previewIsBounded` |
| `dd_structural_delete` | Delete a 500×500 pixel layer from a 6000×4000 eight-layer document | Only the layer's before influence is composited, the result equals a full recomposite, and one region blit is emitted | `pictura-app::bounded_delete_refreshes_only_its_region`, `tst_damage_bounds::structuralDeleteEmitsOneRegion` |
| `dd_structural_reorder` | Reorder a layer whose influence is smaller than the canvas | Only the union of its before and after influence is composited and the result equals a full recomposite | `pictura-app::bounded_reorder_refreshes_only_its_region`, `tst_damage_bounds::structuralReorderEmitsOneRegion` |
| `dd_structural_clip_run` | Delete or move a non-clipped base with clipped layers directly above it | The reported influence includes the clipped run and the result equals a full recomposite | `pictura-render::clipping_run_is_influenced`, `pictura-app::bounded_delete_refreshes_only_its_region` |
| `dd_structural_group` | Group bounded layers, then ungroup the group | Only the union of the affected before/after influence is composited and the result equals a full recomposite | `pictura-app::bounded_group_and_ungroup_refresh_only_their_region`, `tst_damage_bounds::structuralGroupEmitsOneRegion` |
| `dd_structural_duplicate` | Duplicate a bounded layer | Only the union of the source's before influence and the copy's after influence is composited and the result equals a full recomposite | `pictura-app::bounded_duplicate_refreshes_only_its_region`, `pictura-render::duplicate_and_insertion_are_bounded` |
| `dd_structural_insert` | Insert a bounded-content layer into a document with existing layers | Only the inserted layer's after influence is composited and the result equals a full recomposite | `pictura-app::bounded_non_neutral_insertion_refreshes_only_its_region`, `pictura-render::duplicate_and_insertion_are_bounded` |
| `dd_structural_large` | Run a bounded structural edit whose disjoint damage areas sum to at least three quarters of the canvas | The whole document is recomposited and no history hint is passed | `pictura-app::large_bounded_structural_edit_rebuilds_fully` |
| `dd_structural_live` | Commit a bounded structural edit while a live gesture or an opacity/fill/mask preview is pending | The whole document is recomposited and no history hint is passed | `pictura-app::structural_edit_during_live_session_rebuilds`, `pictura-app::structural_edit_with_pending_preview_rebuilds` |
| `dd_structural_mask_solo` | Commit a bounded structural edit while a mask is soloed | The whole document is recomposited, so the solo source is re-resolved or cleared | `pictura-app::structural_region_edit_with_solo_mask_rebuilds` |
| `dd_structural_unbounded` | Run a structural edit affecting an unmasked adjustment, an unexpectedly missing path, or a root-level/group knockout base | The whole document is recomposited | `pictura-app::unbounded_structural_rebuilds`, `pictura-render::unexpected_missing_path_is_unbounded`, `pictura-render::knockout_base_change_is_unbounded`, `tst_damage_bounds::unboundedStructuralRebuilds` |
| `dd_damage_property` | Generate delete, reorder, duplicate, group, ungroup, visibility, effect, clipping, knockout, adjustment, and feathered-mask edits on a small document | Every changed pixel is inside the reported damage or the edit reports no bound, and at least one case is bounded | `pictura-render::random_structural_edits_are_covered` |
| `dd_damage_feather` | Bound a masked adjustment with a feather radius | The damage includes the mask extent grown by the feather reach | `pictura-render::feathered_adjustment_is_bounded` |
| `dd_damage_ops` | Commit a bounded merge, type, shape, rasterize, smart-object, warp, batch-changed, smart-filter, or non-neutral insertion edit | The stored composite equals a full recomposite and pixels outside the damage keep their value | `pictura-app::bounded_edits_equal_full_recomposite`, `pictura-app::bounded_merge_refreshes_only_its_region`, `pictura-app::bounded_type_rasterize_refreshes_only_its_region`, `pictura-app::bounded_batch_refreshes_only_its_region`, `tst_damage_bounds::structuralDeleteEmitsOneRegion` |
| `et_one_owner` | Commit an edit through the transaction | The declared compositing runs, one history state is captured, and the document is left dirty with the revision advanced | `pictura-app::one_commit_refreshes_captures_and_versions` |
| `et_reports_outcome` | Return an outcome from the mutation | The transaction returns the same outcome | `pictura-app::the_declared_outcome_is_returned` |
| `et_mutation_inside` | Refuse or change nothing inside the transaction | The document is unchanged, no history state is captured, and no change signal is emitted | `pictura-app::a_refused_mutation_records_nothing`, `pictura-app::an_unchanged_edit_records_nothing` |
| `et_bounded` | Declare one bounded rectangle | Only that rectangle is composited, outside pixels keep their value, and the result equals a full recomposite | `pictura-app::a_bounded_outcome_requests_one_region` |
| `et_paint_regions` | Declare several disjoint paint rectangles | They are composited in one pass and one region blit covers their bounding box | `pictura-app::a_multi_rect_outcome_requests_one_blit` |
| `et_meta` | Commit a metadata-only or selection edit | One history state is captured, no compositor runs, no damage is marked, and one notification is emitted | `pictura-app::a_metadata_outcome_notifies_without_compositing` |
| `et_record` | Commit after a preview already presented the change | One history state is captured, no compositor runs, and no additional notification is emitted | `pictura-app::an_already_presented_edit_records_without_notifying` |
| `et_bounded_notify` | Commit a bounded batch, mask-refine, or smart-filter edit that changes panel state | Exactly one notification is emitted and the panel/command registry refreshes | `pictura-app::bounded_panel_commit_notifies` |
| `et_structural` | Invalidate positional links and declare bounded damage | Links are cleared, only the declared rectangles are composited, one region blit and one notification are emitted | `pictura-app::a_bounded_structural_outcome_clears_links_and_uses_a_region`, `tst_damage_bounds::structuralDeleteEmitsOneRegion` |
| `et_structural_full` | Invalidate positional links and declare no damage | Links are cleared, one full recomposite runs, and the whole canvas is rebuilt | `pictura-app::an_unbounded_structural_outcome_rebuilds` |
| `et_unbounded` | Declare an unbounded pixel edit | The whole document is recomposited once and equals the refreshed display | `pictura-app::a_full_outcome_rebuilds_once` |
| `et_inserted` | Commit a content-neutral insertion | Links are cleared, one state is captured and one notification emitted; composite, pyramid, and frame revision are unchanged | `pictura-app::an_inserted_outcome_drops_links_without_compositing`, `pictura-app::neutral_insertion_still_yields_inserted` |
| `et_history_hint` | Commit a bounded edit | The history capture receives the same rectangles the edit refreshed | `pictura-app::a_bounded_edit_hints_its_refresh_rects` |
| `et_history_hint_move` | Commit a bounded move without advancing the content revision | The history capture receives the source/destination union | `pictura-app::move_commit_hints_its_refresh_rects` |
| `et_history_hint_empty` | Commit an edit with an empty rectangle list | The history capture receives no hint and compares every plane | `pictura-app::empty_hint_is_no_hint` |
| `et_history_hint_stroke` | Present some or all stroke regions, then commit the rest | The history capture receives the whole stroke's rectangles, not only the pending region, including when no region is pending | `pictura-app::paint_commit_hints_all_stroke_tiles`, `pictura-app::paint_commit_with_no_pending_region_hints_the_whole_stroke` |
| `eh_hint_equal` | Capture the same change with a covering hint and without a hint | Both captures store the same plane deltas and tiles | `pictura-app::hinted_and_unhinted_captures_match` |
| `eh_hint_skips` | Change a plane outside the hint while the rest lies inside | The hinted delta contains only tiles intersecting the hint; the omitted tile is a caller-contract violation, not a correctness claim | `pictura-app::hinted_capture_skips_outside_tiles` |
| `eh_hint_undo` | Undo a hinted capture | The restored document and composite are byte-identical to the pre-change state | `pictura-app::hinted_capture_undo_is_bit_identical` |
| `eh_hint_geometry` | Capture across a tracked-plane geometry change with a hint | A full segment starts regardless of the hint | `pictura-app::hinted_capture_falls_back_on_geometry_change` |
| `eh_hint_empty` | Capture with no hint or an empty hint | Every tracked plane is compared in full, matching an unhinted capture | `pictura-app::empty_hint_captures_everything` |
| `eh_hint_random` | Capture randomized bounded edits once with covering hints and once without | Every pair stores the same deltas/tiles, and undo restores the same state | `pictura-app::random_hinted_and_unhinted_captures_match` |
| `eh_hint_document_channel` | Change a document channel inside the hint | The capture compares and stores the corresponding canvas tiles for that channel | `pictura-app::document_channel_hint_matches` |
| Delete 500×500 under 20 ms at 6000×4000 | Run the ignored `structural_delete_profile` | The composite phase is below 20 ms and the canvas bytes are layer-sized | manual: profile timings are machine-dependent |
| Stroke capture scales with dirty tiles | Run the ignored `stroke_capture_profile` with a short and a long stroke | `record_history_capture` scales with the stroke's dirty tiles, not the document | manual: profile timings are machine-dependent |
| Mask feather preview stays region-sized | Run the ignored `mask_feather_preview_profile` while dragging feather | The refresh composites the layer's influence, not the document | manual: profile timings are machine-dependent |
