# Test Plan

One row per scenario in the change's spec deltas. Coverage names the automated
test as `<crate-dir>::<fn>`; no row is manual.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `dd_structural_delete` | Delete a bounded 500×500 pixel layer from a large document and composite the reported before influence. | Only the layer's influence rectangle changes and the region equals a full recomposite. | `pictura-render::structural_delete_uses_influence` |
| `dd_structural_reorder` | Reorder a bounded layer and union its before/after influence. | Only the union changes and the region equals a full recomposite. | `pictura-render::structural_reorder_uses_influence` |
| `dd_structural_clip_run` | Delete or move a non-clipped base with clipped layers above it. | The influence includes the clipped run, because the run is released or re-based. | `pictura-render::clipping_run_above_a_deleted_base_is_included` |
| `dd_structural_clip_member` | Reorder a full-canvas clipped member inside its run. | The damage is the base's reach, not the member's full-canvas reach. | `pictura-render::clipped_member_reorder_is_bounded_by_the_base` |
| `dd_structural_clip_out` | Move a clipped member out of its run so it becomes unclipped. | The damage includes its before base reach and its after unclipped reach. | `pictura-render::moving_a_clipped_layer_out_covers_old_base_and_own_reach` |
| `dd_structural_clip_base` | Move a non-clipped base with a clipped run above it. | The before damage includes the released run's members. | `pictura-render::moving_the_base_out_keeps_the_released_run_covered` |
| `dd_structural_clip_release` | Release the clipping mask on a middle clipped member with a full-canvas member above it. | The freed full-canvas member is in the damage refs and the region over the damage equals the full composite. | `pictura-render::releasing_a_clipping_mask_damages_the_freed_run` |
| `dd_structural_group` | Group bounded layers and ungroup a bounded group. | Only the affected before/after influence changes and the region equals a full recomposite. | `pictura-render::group_and_ungroup_are_bounded` |
| `dd_structural_duplicate` | Duplicate a bounded layer. | The source's before influence and the copy's after influence bound the damage. | `pictura-render::duplicate_and_insertion_are_bounded` |
| `dd_structural_insert` | Insert a bounded layer into a document with existing layers. | The inserted layer's after influence bounds the damage. | `pictura-render::duplicate_and_insertion_are_bounded` |
| `dd_structural_large` | Apply a structural edit whose clipped damage area is at least three quarters of the canvas. | The whole document is recomposited and no history hint is passed. | `pictura-app::large_bounded_structural_edit_rebuilds_fully` |
| `dd_structural_live` | Commit a bounded structural edit during a live session or a pending preview. | The whole document is recomposited and no history hint is passed. | `pictura-app::structural_edit_during_live_session_rebuilds` |
| `dd_structural_mask_solo` | Commit a bounded structural edit while a mask is soloed. | The whole document is recomposited so the solo source is re-resolved. | `pictura-app::structural_region_edit_with_solo_mask_rebuilds` |
| `dd_structural_unbounded` | Bound an unmasked adjustment, an unexpectedly missing path, or a knockout base. | The influence is unbounded and the full recomposite runs. | `pictura-render::unmasked_adjustment_is_unbounded` |
| `dd_neutral_move_hidden` | Move a hidden layer into and out of a clipping run. | The compositor does not run and the composite is byte-identical. | `pictura-render::hidden_content_move_is_neutral` |
| `dd_neutral_move_inert` | Move an empty layer or empty group into and out of a clipping run. | The compositor does not run and the composite is byte-identical. | `pictura-render::inert_move_into_a_run_is_neutral` |
| `dd_neutral_move_base` | Move an inert or hidden layer that is a clipping base before or after the move. | The move is not content-neutral and the full recomposite runs. | `pictura-render::inert_base_before_is_not_neutral` |
| `dd_neutral_move_knockout` | Move an inert bottom layer while a knockout layer exists. | The move is not content-neutral because it reassigns the knockout base. | `pictura-render::inert_bottom_move_under_a_knockout_is_not_neutral` |
| `dd_neutral_move_property` | Apply every move the predicate calls neutral to a set of small stacks. | Each result's composite is byte-identical to the before composite. | `pictura-render::neutral_moves_preserve_composite` |
| `dd_neutral_move_pending` | Commit a content-neutral insertion or move while an uncommitted preview is pending. | The whole document is recomposited. | `pictura-app::inserted_edit_with_pending_preview_rebuilds` |
