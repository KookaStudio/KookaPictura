# Test plan

One row per spec scenario. Coverage names an automated test, or `manual:` with
the reason it cannot be automated.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lcb_new_empty | Add a new pixel layer to a document; read its rect and channel lengths | rect is 0×0, every plane empty, composite unchanged | `pictura-render::new_layer_is_empty_rect` |
| lcb_neutral | Add a new layer with `neutral_fill` on a Multiply blend | rect is the canvas and every pixel is white | `pictura-render::neutral_fill_layer_is_canvas_sized` |
| lcb_paint_grow | Begin a brush stroke on an empty layer, dab away from the origin | rect covers the dab and the pixels equal a canvas-sized stroke | `pictura-paint::paint_outside_rect_grows_layer` |
| lcb_fill_grow | Fill a box on an empty-rect layer, then fill outside the grown rect | rect covers the box and the second fill lands | `pictura-paint::fill_outside_rect_grows_layer`, `tst_fill_tools::fillGrowsAnEmptyLayer` |
| lcb_filter_grow | Commit a Gaussian blur on a layer whose content reaches its edge | rect grows by the apron and the blurred edge is complete | `pictura-app::blur_grows_the_layer` |
| lcb_locked | Grow a transparency-locked layer, a group, and an adjustment layer | each rect is unchanged | `pictura-core::refuses_a_transparency_lock` |
| lcb_refused | Pin the governor's room below the grow's bytes and grow | refusal, rect and pixels unchanged | `pictura-core::refuses_when_the_governor_is_full` |
| lcb_move_trim | Move a content-sized layer, then fill outside its moved rect | the composite follows the move and the later fill lands | `tst_move_feedback::moveThenFillOutsideTheLayer`, `pictura-render::trim_to_content_hugs_the_opaque_pixels` |
| lcb_trim_composite | Trim a padded layer; compare every pixel before and after | every non-transparent pixel is identical | `pictura-render::trim_keeps_the_composite` |
| lcb_goldens | Run the paint, fill and filter goldens | all unchanged | `pictura-render::imagemagick_fixtures_reproduce` |
| lcb_fork_grown | Ask `fork_bytes` for a box outside a small layer | estimate covers the union rect's bands | `pictura-app::a_fill_that_grows_the_layer_forks_the_grown_bands` |
| elr_empty | Write and read a document with a 0×0 layer | rect 0×0, channel ids preserved, planes empty | `pictura-codec::empty_layer_round_trips` |
| elr_grown_oracle | Write a grown layer, read with psd-tools and Pictura | same rect and pixels both ways | `pictura-codec::psd_tools_reads_a_grown_layers_rect_and_pixels` |
| elr_identical | Read and write an existing PSD unchanged | byte-identical output | `pictura-codec::existing_files_stay_byte_identical` |
| fmb_grown_bands | Fill a box outside a small layer with the gate pinned | estimate covers the grown bands | `pictura-app::a_fill_that_grows_the_layer_forks_the_grown_bands` |
| fmb_gate | Existing refusal suite | unchanged | `tst_fill_tools::fillsRefuseWhatMemoryCannotHold` |
| fmb_large_manual | 17000² marquee bucket on a 7 GB host | no OOM kill; refusal message when it cannot fit | manual: needs a 17000² document and a memory-limited host; covered by `scripts/profile-tools.py --scenarios memory` |
