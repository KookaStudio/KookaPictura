# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pbf_uniform_flood` | Flood an empty layer contiguous and global; flood inside an opaque ring at tolerance 0 | Whole surface both ways; ring inside only; all but the ring | `pictura-paint::an_empty_layer_floods_whole_and_an_enclosed_seed_does_not` |
| `pbf_antialias_exact` | Anti-alias pseudo-random masks at 1×1, 1×7, 7×1, 2×2, 37×23, plus a zero width | Equal to the per-pixel box; a short tail untouched | `pictura-paint::the_parallel_antialias_matches_the_per_pixel_box` |
| `pbf_blend_reuse` | Fill striped backdrops at changing strengths in Normal, Multiply, Behind | Each pixel equals its lone blend | `pictura-paint::a_run_of_equal_pixels_blends_like_each_pixel_alone` |
| `pbf_large_manual` | New 16507×16196 document, new layer, Paint Bucket click, undo, with `PICTURA_PAINT_TIMING=1` | The fill lands and undoes; no flood, anti-alias or composite phase dominates | manual: the full-size document does not fit the test runners' memory; `pictura_app::bucket_fill_profile` times the stages at a divisor |
| `obs_start` | Ask each band of the five-layer stack, and of the clear-bottom stack, where it starts | Covered bands start at the opaque layer; the rest, and every clear-bottom band, at the bottom | `pictura-render::a_band_starts_at_the_opaque_layer_over_it` |
| `obs_exact` | Composite the stacks, and the hidden-above stacks, in bands and band by band; convert every byte | Byte-identical; the hidden-above stack's bands are copies; `to_u8(k/255) == k` | `pictura-render::a_skipped_band_composites_as_the_whole_stack_does`, `pictura-render::an_opaque_top_layer_alone_is_copied` |
| `iph_shared_bands` | Open a document, commit a stroke, undo; compare band allocations and bytes | Every restored state is byte-identical and every untouched band is shared | `pictura-app::opening_a_document_shares_bands_with_history` |
| `iph_capture_skips_shared_bands` | Dab inside one band, capture, inspect the scanned bands | Only the touched band is tile-scanned | `pictura-app::a_capture_scans_only_changed_bands` |
| `iph_fork_on_write` | Write the live document after a capture | Only the written bands are copied | `pictura-app::a_live_write_forks_only_touched_bands` |
| `iph_capture_shares_plane` | Change a whole layer; capture with no hint; repeat with a canvas-sized hint | Unhinted shares the live planes; hinted holds equal bytes of its own | `pictura-app::a_whole_layer_change_is_shared_and_undoes_in_place`, `pictura-app::a_hinted_capture_still_writes_only_its_tiles` |
| `iph_undo_damage` | Stroke, undo | Bytes match, rectangle covers the tiles | `pictura-app::crossing_a_segment_reports_the_changed_tile` |
| `iph_geometry_damage` | Resize, undo | Whole document | `pictura-app::undoing_a_resize_reports_the_whole_document` |
| `iph_structure_damage` | Structure change with a changed tile, undo | Rectangle covers it | `pictura-app::crossing_a_segment_reports_the_changed_tile` |
| `iph_undo_shares_plane` | Whole-layer change over 200 rows; undo, redo, undo | Each restore exact; the live bands are the current state's after the undo | `pictura-app::a_whole_layer_change_is_shared_and_undoes_in_place` |
