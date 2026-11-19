# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ahm_one_pass_identical` | For five tips (soft, hard, squashed and rotated at partial opacity and flow, aliased square, 1 px), every style, and three lock/transparency cases, run 12 dabs through `dab` and through tip-by-tip stamping. | The same dirty rectangle for every dab, and the same pixels after the run. | `pictura-paint::one_pass_strokes_match_tip_by_tip_stamping` |
| `ahm_budget` | On an otherwise idle machine, build optimized and run `profile-tools.py --size 8000x6000 --gpu on --moves 60 --scenarios art-history-textured,history-brush-textured` three times. | The median Art History motion p95 is at most 1.25× the median History Brush p95. | manual: a timing ratio depends on the host and its load; CI runners are shared and have no GPU, so a gate would be flaky |
| `sts_restore_tiled` | Tile a 170×150 layer covering the document, paint corner to corner with the Brush and with Dodge, and cancel each. | The layer keeps its bounds and equals its pre-stroke state. | `pictura-paint::cancelling_on_a_tiled_layer_restores_partial_edge_tiles` |
| `rpb_small_cpu` | Ask whether 520² and 4,000,000 − 1 pixel regions refresh on the GPU with GPU compute on. | Neither does. | `pictura-app::a_small_region_refresh_composites_on_the_cpu` |
| `rpb_large_gpu` | Ask for a 4,000,000-pixel region with GPU compute on, and a larger one with it off. | The first is offered to the GPU; the second is not. | `pictura-app::a_small_region_refresh_composites_on_the_cpu` |
| `pr_coarse_only` | Cache levels 1–2 only, edit a region, and repair it. | No level-0 tile appears, and every cached tile equals a rebuild. | `pictura-render::a_repair_with_only_coarse_tiles_cached_matches_a_rebuild` |
| `pr_level0` | Cache levels 0–2, edit a region, and repair it. | Every tile equals a rebuild; a far tile keeps its allocation. | `pictura-render::damage_repairs_cached_tiles` |
| `pr_interleave` | Interleave a composited region. | Equal to a serial plane-to-pixel copy. | `pictura-render::interleave_matches_a_serial_copy` |
| `ahp_scenarios` | `profile-tools.py --self-check` runs the three scenarios on 512². | Each records 60 moves and an undo time. | `profile-tools::self_check_motion` |
| `ahp_textured_paints` | In the same self-check, read the history list before each textured scenario's undo. | The newest state is "Art History Brush" or "History Brush". | `profile-tools::self_check_motion` |
| `ahp_brush_source` | Apply Posterize, then send `history` `brush_source` with the current index. | The source is that state, the index is unchanged, and the tier is Edit. | `tst_control_editing::historySetsTheBrushSource` |
| `ahp_brush_source_invalid` | Send `brush_source` with the index past the last state. | `invalid_param`; the source is unchanged. | `tst_control_editing::historySetsTheBrushSource` |
