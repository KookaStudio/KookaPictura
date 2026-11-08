# Test Plan

This plan covers `tools/tiled-flood` and `tools/tiled-selection-masks` for
`tiled-fills-and-masks` (#1214).

**Fixtures.** Parity tests compare each tiled operation with the dense code it
replaces, kept as a `#[cfg(test)]` reference. The sources are seeded random
masks and images at sizes that are not multiples of 256, built with blocks of
one value so that some tiles are uniform. Checks that assert "no
materialization" run under nextest, one process per test.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `tf_parity` | Flood seeded 300×700, 513×257 and 600² sources at tolerances 0, 8, 40 and 255, contiguous and global, with and without a wall | The tiled mask equals the dense flood, byte for byte | `pictura-core::tiled_flood_matches_dense` |
| `tf_reached` | Flood a closed 50² room inside one tile of a 2048² source | One tile has a visited bitmap; every other output tile is `Empty` | `pictura-core::visited_state_covers_only_reached_tiles` |
| `tf_uniform` | Flood a uniform 4250² source from its centre | Every output tile is `Uniform(255)`; no dense tile | `pictura-core::a_uniform_source_floods_whole_tiles` |
| `tf_empty_layer` | Bucket an empty layer with no selection, anti-alias on; read the pixels and the tile states (engine), and the pixels across tile edges (GUI) | Every pixel is the foreground; every plane tile is `Uniform`; no dense tile | `pictura-paint::a_whole_canvas_bucket_writes_uniform_tiles`, `tst_fill_tools::paintBucketFillsAWholeEmptyLayer` |
| `tf_fill_parity` | Run the existing bucket, fill and stroke unit tests | All pass unchanged | `pictura-paint::a_boxed_fill_matches_the_document_mask_and_forks_only_its_tiles`, `pictura-paint::a_windowed_stroke_equals_the_whole_document_stroke`, `pictura-paint::the_selection_is_a_wall_for_the_contiguous_flood`, `pictura-paint::the_selection_edge_is_not_softened` |
| `tf_uniform_write` | Fill solid red at 60 % Normal through a marquee whose interior tiles are `Uniform(255)` on a uniform layer | Interior target tiles are `Uniform`; every pixel equals the per-pixel blend | `pictura-paint::uniform_tiles_are_filled_without_allocating` |
| `tf_wand` | Run the Magic Wand (contiguous and global), Grow and Similar on a seeded composite | Each selection equals the dense reference | `pictura-select::tiled_wand_matches_dense`, `pictura-select::tiled_grow_and_similar_match_dense` |
| `tf_magic_eraser` | Magic-erase a seeded layer with anti-alias on and off | The layer equals the dense reference | `pictura-paint::magic_erase_tiled_matches_dense` |
| `tf_heap` | Run the M4 and M5 heap guards at 4250² under nextest; count the layer's planes after M5 | Peak ≤ 1.1 × lowered baseline; 0 materializations; the layer holds ≤ 4 MiB | `pictura-app::bucket_in_selection_heap_peak`, `pictura-app::bucket_whole_layer_heap_peak`, `pictura-app::bucket_layer_plane_cost`, `pictura-app::estimates_cover_the_measured_peaks` |
| `tf_wand_heap` | Run the Magic Wand on one disc of the M12 fixture at 4250² | Peak ≤ 1.1 × baseline, far below one plane the size of the document | `pictura-app::magic_wand_heap_peak` |
| `tf_large_manual` | Run `python3 scripts/profile-tools.py --scenarios M4,M5 --peak --size 17000x17000` on master `daeed06f` and on this branch | Peak ≤ 64 MiB above steady; the layer after M5 ≤ 4 MiB; time ≤ 1.1 × | manual: a 17000² run needs about 4 GB and minutes per scenario, and wall-clock timing on a quiet machine is not deterministic enough for CI |
| `tsm_marquee` | Select a 3000×2000 rectangle and an ellipse on a 4250² document | Interior tiles `Uniform(255)`, exterior `Empty`, dense tiles only on the edge; bytes equal the per-pixel shape | `pictura-select::tiled_mask_marquee_interior_is_uniform` |
| `tsm_polygon` | Select a seeded lasso polygon | Equals the dense even-odd fill; tiles outside its bounds are `Empty` | `pictura-select::tiled_polygon_matches_dense` |
| `tsm_boolean` | Combine seeded soft masks (each operation); invert; translate | Equal to the dense reference; uniform input tiles stay uniform | `pictura-select::tiled_boolean_ops_match_dense` |
| `tsm_margin_ops` | Feather at radii 1, 3, 9 and 60; expand, contract, border and smooth at radii 1, 4 and 40 | Equal to the dense reference, byte for byte | `pictura-select::tiled_margin_ops_match_dense` |
| `tsm_oracle` | Run the select oracle tests | They pass unchanged | `pictura-select::expand_matches_imagemagick_dilate`, `pictura-select::contract_matches_imagemagick_erode`, `pictura-select::feather_matches_imagemagick_gaussian` |
| `tsm_no_materialize` | Run every selection operation on a tiled mask inside `assert_no_materialization` | No materialization | `pictura-select::selection_ops_do_not_materialize` |
| `tsm_quick_mask` | Enter and leave Quick Mask over a marquee with no edit | The selection is tiled, its interior is `Uniform(255)`, and its bytes are unchanged | `pictura-app::a_quick_mask_round_trip_keeps_the_tiles` |
