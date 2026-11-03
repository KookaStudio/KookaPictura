# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pbf_uniform_flood` | Flood an empty layer contiguous and global; flood inside an opaque ring at tolerance 0 | Whole surface both ways; ring inside only; all but the ring | `pictura-paint::an_empty_layer_floods_whole_and_an_enclosed_seed_does_not` |
| `pbf_antialias_exact` | Anti-alias pseudo-random masks at 1×1, 1×7, 7×1, 2×2, 37×23, plus a zero width | Equal to the per-pixel box; a short tail untouched | `pictura-paint::the_parallel_antialias_matches_the_per_pixel_box` |
| `pbf_blend_reuse` | Fill striped backdrops at changing strengths in Normal, Multiply, Behind | Each pixel equals its lone blend | `pictura-paint::a_run_of_equal_pixels_blends_like_each_pixel_alone` |
| `pbf_large_manual` | New 16507×16196 document, new layer, Paint Bucket click, undo, with `PICTURA_PAINT_TIMING=1` | The fill lands and undoes; no flood or anti-alias phase dominates | manual: the full-size document does not fit the test runners' memory; `pictura_app::bucket_fill_profile` times the stages at a divisor |
