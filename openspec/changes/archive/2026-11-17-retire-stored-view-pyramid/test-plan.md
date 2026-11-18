# Test plan

Implementation-only pass: the listed new checks and extensions are written.
The State column records the original coverage gap; execution remains deferred
at the user's request, including the new GPU comparison's byte-exact expectation.
CI runs the profile checker for normal release and dev-app artifacts and runs
renderer tests alone before workspace feature unification.

Each delta scenario has one row. Coverage is last; `new` means a planned check,
`extend` means the existing body does not yet assert the entire THEN, and
`retain` means useful assertions must survive deleting mixed coarse/update
coverage. These are implementation tasks, not claimed completed tests. No
manual-only rows are needed: build exclusions, numerical outputs and these GUI
behaviors can all be checked automatically. The test-existence guard is expected
to name the planned checks until apply creates them.

For `vo_renderer_tests`, run renderer unit tests in a separate invocation without
`--features test-oracle`, rather than relying on workspace feature unification.
For `vo_release` and `vo_dev_app`, pass Cargo JSON output from ordinary library
builds to the planned profile checker; never substitute a test/all-targets build.

## compositing/region-composite

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `rc_level0` | a region is composited at level 0 for seeded documents covering every blend mode the oracle covers | it equals the same rectangle cropped from the whole composite, byte for byte | existing | `pictura-render::composite_region_matches_full_crop` |
| `rc_levels` | regions are composited at levels 1 through 3, including rectangles that touch odd-sized right and bottom edges | each equals the same rectangle cropped from that level of a pyramid rebuilt from the whole composite, byte for byte | existing | `pictura-render::composite_region_folds_odd_edges`, `pictura-render::composite_region_matches_full_crop` |
| `rc_outside` | a requested rectangle extends past the level's bounds | the part outside is fully transparent and the part inside matches the crop | existing | `pictura-render::composite_region_outside_is_transparent` |
| `rp_odd_edge` | a region includes a last row or column of an odd-sized level | its reduced pixels equal successive rounded averages of only the existing samples and match the independent whole-rebuild reference | existing | `pictura-render::composite_region_folds_odd_edges`, `pictura-render::composite_region_matches_full_crop` |
| `rp_soft_edge` | opaque and partially transparent colored samples are reduced together | their colors are premultiplied before averaging, and the returned channels equal the independent reference byte for byte | existing | `pictura-render::shrinking_premultiplied_keeps_a_soft_edge_its_colour`, `pictura-render::composite_region_matches_full_crop` |
| `rp_rounding` | a region is requested at levels 1 through 3 | its bytes equal a whole reference reduced one level at a time, including odd edges and transparent out-of-level pixels | existing | `pictura-render::composite_region_folds_odd_edges`, `pictura-render::composite_region_matches_full_crop` |

## compositing/render-session

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `rs_fallback_no_level` | the canvas requests a level unavailable in the document's pyramid | the exact path runs and the presented frame equals the full composite | extend | `pictura-app::no_stored_level_falls_back_to_exact`, `pictura-app::unavailable_preview_levels_present_and_commit_exact_pixels` |
| `rs_level0_only` | a canvas-sized edit is previewed at level 0 on a document with no reduced level | the level-0 preview is presented and equals the full composite | extend | `pictura-app::preview_leaves_the_composite_unchanged` |
| `rs_fallback_budget` | the backdrop plus the scratch canvas would exceed the session's memory budget | no backdrop is cached and the exact path runs | extend | `pictura-render::over_budget_backdrop_is_refused` |
| `rs_fallback_state` | a mask is soloed or a stroke is live | the exact path runs and the displayed frame is the one that state defines | extend | `pictura-app::solo_mask_and_live_stroke_fall_back`, `pictura-app::solo_mask_and_live_stroke_present_exact_fallback_pixels` |
| `rs_fallback_content` | a reduced-level preview is requested and the edited stack contains a live type layer, a proxy-less smart object, or a vector fill | the preview and the commit use the exact path and equal the full composite | extend | `pictura-render::unreducible_content_falls_back_to_full`, `pictura-app::unreducible_content_presents_and_commits_exact_fallback_pixels` |
| `rs_fallback_empty` | the visible document rectangle is empty | no preview is presented and no damage is marked | extend | `pictura-render::empty_rect_is_a_noop` |
| `rs_fallback_no_presented_level` | the canvas reports no presented level | the exact path runs and the frame equals the full composite | existing | `pictura-app::no_presented_level_falls_back_to_exact`, `pictura-app::unavailable_preview_levels_present_and_commit_exact_pixels` |

## compositing/tile-pyramid

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `tp_crop` | the canvas requests a crop at levels 0 through 3 of a document whose tiles are built on demand | each crop equals the same crop of a pyramid rebuilt from the whole display composite | existing | `pictura-app::tile_crops_match_a_whole_rebuild` |
| `tp_visible_only` | a large document is presented at 100 % in a small viewport | only the tiles under the view and its margin are composited and resident | existing | `pictura-app::only_visible_tiles_are_built` |
| `tp_no_frame` | a large layered document is opened and presented | no buffer of the document's full display size is resident afterward | existing | `pictura-app::no_full_resolution_frame_is_kept` |
| `tp_level_sizes` | levels are reported for a 1500x700 image | their sizes are 1500x700, 750x350 and 375x175, with no further level | extend | `pictura-render::level_count_matches_the_view_pyramid` |
| `tp_level_511` | levels are reported for a 511x300 image | their sizes are 511x300 and 256x150, with no further level | extend | `pictura-render::level_count_matches_the_view_pyramid` |
| `tp_level_small` | levels are reported for a 300x200 image | only level 0 is available | extend | `pictura-render::level_count_matches_the_view_pyramid` |
| `tp_level_thin` | levels are reported for a 4096x2 image or its transpose | only level 0 is available even though another ceil-halving would leave the long side above 256 | new | `pictura-render::thin_images_keep_only_level_zero` |

## tools/mask-stroke-preview

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `msp_large_mask_dab_previewed` | a mask stroke runs with GPU compute off and a dab bounding box above the raster budget on a document whose pyramid has that level | the view presents the stroke at that level, and the mask is unchanged until the commit | existing | `tst_mask_edit::largeMaskDabPreviews` |
| `msp_preview_blend` | the preview presents a level pixel whose coverage moves the mask value from `m0` to `m1` | the presented pixel is the stroke-start snapshot plus `(m1 - m0) / 255` of the difference between the reduced revealed and hidden composites, and zero coverage presents the snapshot unchanged | existing | `pictura-app::mask_preview_moves_the_snapshot_by_the_coverage_change` |
| `msp_commit_exact` | a previewed mask stroke ends | the mask holds the exact stroke's coverage, the composite shows it, and one history state is recorded | existing | `tst_mask_edit::largeMaskDabPreviews` |
| `msp_cancel` | a previewed mask stroke is cancelled | no history state is recorded and the mask and the composite are the pre-stroke ones | existing | `tst_mask_edit::largeMaskDabPreviews` |

## tools/paint-engine

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `pe_preview_below_threshold` | a stroke runs with a dab bounding box at or under the raster budget | it rasterizes and presents at full resolution, with no reduced document and no change to the pixels it produces | existing | `pictura-app::a_dab_that_fits_the_budget_is_not_previewed` |
| `pe_preview_above_threshold` | a stroke runs with a dab bounding box above the raster budget | the in-progress stroke is presented at a tile-pyramid level whose own work fits the budget, built from a snapshot taken at stroke start rather than a scan of the document, and the number of pixels rasterized per dab no longer grows with the bounding box | new + existing | `pictura-app::a_dab_over_the_budget_previews_at_a_level_that_fits`, `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `pe_preview_is_presentational` | a previewed stroke is recorded | the reduced-resolution work is discarded, the exact full-resolution stroke is computed before the history state is recorded, and every pixel of the committed document is byte-identical to the same stroke run without a preview | new | `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `pe_preview_cancel` | a previewed stroke is cancelled | the recorded samples are dropped without producing a history state and the displayed document is the pre-stroke one | new | `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `pe_pyramid_per_frame` | several dabs arrive between two presents of a live stroke | the pending tile damage is refreshed once, at the flush, rather than once per dab | new + existing | `pictura-app::present_accumulates_until_flush_and_the_stroke_lifecycle_supersedes_it`, `tst_present_cache::frameBoundedPaintMatchesCpu` |
| `pe_pyramid_flush_equal` | a frame's accumulated region is folded into the pyramid | every exact tile presented equals a rebuild from the frame's current working document | extend | `pictura-app::mid_stroke_tiles_match_a_full_recomposite_of_the_working_document` |

## ui/canvas-frame-reuse

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `cfr_no_image` | the `tst_large_document` suite paints on a document | the canvas reports the document's size, holds no image, and the image it yields on request shows the painted pixels | existing | `tst_large_document::aDocumentCanvasHoldsNoImage` |

## ui/document-canvas

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `A dab repaints through a level crop` | the canvas repaints after a dab while a stroke is in progress | it presents the level crop chosen for the zoom and does not rescale the full-resolution document | existing | `tst_present_cache::reducedLevelPresentsADab` |
| `The mid-stroke present equals a full recomposite` | a dab has been applied and the canvas is presented mid-stroke | the presented pixels are identical to a full recomposite of the stroke's working document at the same zoom and sampling filter | existing | `pictura-app::mid_stroke_tiles_match_a_full_recomposite_of_the_working_document` |
| `A large document presents through a crop mid-stroke` | a stroke is painted on a 16000²-class document | each repaint crops a level rather than drawing the full-resolution source | extend | `tst_present_cache::largeDocumentPresentsThroughALevel` |
| `cv_preview_present` | a preview stroke presents a region | the canvas crops the tile-pyramid level the preview wrote, that level is repaired in place rather than rebuilt from level 0, and level 0 is unchanged | new | `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `cv_preview_commit_restores_level` | a previewed stroke is released | the exact region repairs or invalidates the affected tiles at every cached level, and the canvas returns to the level the zoom selects | new | `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `cv_preview_cancel_restores_level` | a previewed stroke is cancelled | the pre-stroke document is presented and the canvas returns to the level the zoom selects | new | `tst_present_cache::previewStrokeUsesTilesAndRestoresZoom` |
| `A pan repaint does not resample the document` | the view is panned without changing the zoom | the paint presents the level chosen for the zoom and does not rescale the full-resolution document | existing | `tst_present_cache::panPresentsWithoutResamplingTheDocument` |
| `cvda_zero_copy_level` | the zoom selects a reduced level and the canvas paints the same current rectangle twice | the second paint reuses its assembled tile crop without reconstructing a whole level or re-copying the unchanged crop | extend | `tst_present_cache::cachedLevelCropMatchesTheUncachedDraw` |
| `cvda_level0_crop_reused` | the zoom selects level 0 and the canvas paints twice at the same document rectangle | the second paint reuses the cached premultiplied crop instead of re-cropping the document | existing | `tst_present_cache::level0CropIsReusedAcrossPaints` |
| `cvda_pan_reuses_crop` | the zoom selects level 0, the canvas paints, and then the view pans without changing the zoom | the next paint reuses the cached crop's overlap and crops only the newly exposed band, and the presented pixels are byte-identical to a fresh level-0 crop over the new rectangle | existing | `tst_present_cache::panAndZoomReuseTheLevel0Crop` |
| `cvda_zoom_reuses_crop` | the canvas paints at a zoom that selects level 0 and then zooms in without crossing the level-0 boundary | the next paint reuses the cached crop (the new rectangle is inside the cached one) and the presented pixels are byte-identical to a fresh level-0 crop over the new rectangle | existing | `tst_present_cache::panAndZoomReuseTheLevel0Crop` |
| `A zoom change invalidates the cache` | the zoom changes across a level boundary | the next paint chooses the corresponding level and presents it | existing | `tst_present_cache::zoomChangeSelectsTheNewLevel` |
| `Cached present matches the transform draw` | the canvas paints a document at zoom 100 % or above (level 0) through the present path and through a direct full-resolution transform draw, with the same sampling filter | the two rendered images are pixel-identical | existing | `tst_present_cache::level0PresentMatchesTheTransformDraw` |
| `A coarse-level present is a single reduction` | the canvas paints at a zoom that selects a level below 0 | the source is that level's exact pixels, reduced according to compositing/region-composite, and the paint does not rescale the full-resolution document | existing | `tst_present_cache::cachedLevelCropMatchesTheUncachedDraw` |
| `A document past the former cache bound still presents through a crop` | a document whose scaled present would exceed 64 megapixels is painted at a high zoom | the paint presents the level selected for the zoom rather than drawing the full-resolution source, and does not fall back to the transform draw | extend | `tst_present_cache::largeDocumentPresentsThroughALevel` |
| `cvda_gpu_zoom_level` | the canvas paints while a GPU stroke is live at a zoom that selects a reduced level | it presents that tile-pyramid level rather than forcing level 0 | new | `tst_present_cache::frameBoundedPaintMatchesCpu` |
| `cvda_gpu_dab_in_pyramid` | a GPU dab changes a rectangle of the working document | affected cached tiles are repaired or made stale, and exact tiles presented as current match the independent reference at every requested level | new | `tst_present_cache::frameBoundedPaintMatchesCpu` |
| `cvda_gpu_idle` | no stroke is live | the present level is the zoom-selected level, exactly as before | existing | `tst_present_cache::idlePresentLevelIsZoomSelected` |

## ui/info-histogram-panel

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `Sampling a large document performs no full composite` | a pixel is sampled in the Info panel on a 4000×4000 document whose display tiles are already current | the sample is read through the current composite region or tile and no full-document composite is run | new | `pictura-app::panel_reads_stay_bounded` |
| `Histogram bins a bounded image` | the Histogram panel refreshes on a 4000×4000 document | it bins an image of at most 512×512 and retains the current plotted distribution | new | `tst_present_cache::panelsUseTileCrops` |
| `Histogram refresh uses a tile level` | the Histogram panel refreshes after a region refresh | it reads the coarsest tile-pyramid level without calling `PictureView::image()` or rebuilding the full-resolution image | new | `tst_present_cache::panelsUseTileCrops` |
| `A thin document is scaled after fetching level 0` | the Histogram panel refreshes on a 4096×2 document | it fetches level 0 at 4096×2 and scales it to fit within 512×512 before binning | extend + new | `pictura-app::histogram_source_level_is_bounded_and_size_independent`, `tst_present_cache::panelsUseTileCrops` |
| `Histogram binning is bounded for larger documents` | the Histogram panel refreshes on a document at least twice as large in each dimension | the image it bins is at most 512×512 regardless of fetched source dimensions | new | `tst_present_cache::panelsUseTileCrops` |

## ui/mask-solo-view

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `msv_level` | a mask is soloed and the canvas repaints | the presented pixels come from a tile-pyramid level crop of the mask plane at the zoom-selected level, with the level count and sizes unchanged | existing | `pictura-app::solo_mask_restore_presents_visible_tiles` |
| `msv_level_restore` | the solo mode ends | subsequent crops show the document composite again, with no rebuild of the document | existing | `pictura-app::solo_mask_restore_presents_visible_tiles` |

## ui/navigator-panel

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `The thumbnail comes from a pyramid level` | the Navigator paints a thumbnail for a large document | it samples a tile-pyramid level sized to the widget and does not rescale the full-resolution composite | new | `tst_present_cache::panelsUseTileCrops` |
| `No second full-resolution copy is held` | the Navigator is shown for a document already using a tile pyramid | no additional full-resolution document buffer is allocated for the thumbnail | new | `tst_present_cache::panelsUseTileCrops` |
| `A content change refreshes the thumbnail` | the document composite changes | the Navigator thumbnail redraws from the updated composite tiles at that level | existing | `tst_edit_refresh::aStrokeCommitRefreshesPanels` |

## verification/verification-harness

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `vh_pyramid_selfcheck` | the paint present-tiling self-check runs | its counters show one pending-damage refresh at the flush rather than a refresh per queued dab | new | `tst_present_cache::frameBoundedPaintMatchesCpu` |
| `vh_pyramid_commit` | the same stroke is committed on the checked path and on the CPU path | their committed images are byte-identical | new | `tst_present_cache::frameBoundedPaintMatchesCpu` |
| `vh_commit_blit` | the unit check commits a single-region stroke and compares its blit image with the level-0 crop of the same rectangle | every pixel channel is identical | existing | `pictura-app::a_level_zero_damage_crop_matches_the_cached_crop_rect`, `tst_present_cache::level0PatchMatchesFreshCrop` |

## verification/view-pyramid-oracle

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `vo_renderer_tests` | renderer unit tests run alone without the oracle feature | they can construct the independent reference and compare level counts, sizes and pixels | existing | `pictura-render::level_count_matches_the_view_pyramid`, `pictura-render::composite_region_folds_odd_edges` |
| `vo_app_tests` | application tests build through the development dependency | the whole-rebuild reference is available to existing crop, damage and solo-restore comparisons | existing | `pictura-app::tile_crops_match_a_whole_rebuild`, `pictura-app::an_edit_repairs_the_visible_tiles`, `pictura-app::solo_mask_restore_presents_visible_tiles` |
| `vo_release` | an ordinary application library is built in release with no oracle feature or test targets | the renderer artifact has no oracle feature, an external consumer cannot import the oracle or its planar source adapter, and the shared level result and constants remain usable | new | `check-view-oracle::check_profile`, `pictura-app::view_oracle_is_test_only` |
| `vo_dev_app` | an ordinary application library is built with the dev-app profile and no oracle feature or test targets | the renderer artifact has no oracle feature, an external consumer cannot import the oracle or its planar source adapter, and the shared level result and constants remain usable | new | `check-view-oracle::check_profile`, `pictura-app::view_oracle_is_test_only` |
| `vo_independent` | regions, cached crops, repaired tiles and restored solo-mask tiles are compared against whole rebuilds | expected pixels come from the independent minimal reference and all comparisons remain byte-identical | new + existing | `pictura-app::view_oracle_is_test_only`, `pictura-render::composite_region_folds_odd_edges`, `pictura-app::tile_crops_match_a_whole_rebuild`, `pictura-app::an_edit_repairs_the_visible_tiles`, `pictura-app::solo_mask_restore_presents_visible_tiles` |
| `vo_tiled_source` | the oracle rebuilds and crops a tiled planar source and the same pixels supplied as contiguous slices | the bytes match and the tiled source is not materialized | retain + new | `pictura-render::a_tiled_level0_builds_the_same_levels`, `pictura-render::a_tiled_level0_crops_without_materializing` |

## Additional issue acceptance and checker checks

| Scenario | Steps | Expected | State | Coverage |
|---|---|---|---|---|
| `tp_damage` | Run the existing damage comparison without editing its test body or expected pixels | Repaired tiles match the independent reference; far tiles are not rebuilt | unchanged | `pictura-app::an_edit_repairs_the_visible_tiles` |
| `tp_solo_restore` | Run the existing restored-mask comparison unchanged | Mask tiles and the restored composite match independent whole references | unchanged | `pictura-app::solo_mask_restore_presents_visible_tiles` |
| Profile checker rejects false evidence | Run the checker with malformed/missing/feature-enabled artifact records and unrelated compiler failures | It fails each invalid proof and accepts only the successful positive probe followed by unresolved oracle imports | new | `check-view-oracle::self_check` |

## Assertions to preserve or extend

- Preserve the bodies and expected bytes of the issue's `rc_levels`, `tp_crop`,
  `tp_damage` and `tp_solo_restore` comparisons. The `rp_*` rows reuse that
  independent numerical coverage, rather than introducing a production helper
  into expected-value construction.
- Extend `level_count_matches_the_view_pyramid` to check dimensions as well as
  counts for its existing cases; add both orientations to the new thin-image
  check. Keep the reference's level math separate.
- Extend preview checks: an unavailable requested level falls back, a real
  level-0-only document produces an exact level-0 preview, a live stroke as well as solo mode, each unreducible content kind at a reduced level,
  backdrop-plus-scratch budget exhaustion, and unchanged damage for an empty
  rectangle. Keep exact fallback output assertions.
- Extend `cachedLevelCropMatchesTheUncachedDraw` with the unchanged reduced-crop
  reuse counter. Extend `largeDocumentPresentsThroughALevel` to cover the former
  64-megapixel scaled-present bound at high zoom and a live stroke on a sparse
  16000-class document without allocating a dense full-resolution fixture.
- Extend mid-stroke comparisons to a fixture with reduced levels and multiple
  queued dabs; keep frame-flush counts and exact tile comparisons distinct.
- The `panel_reads_stay_bounded` check asserts Info sample correctness and bounded
  tile/region work. Adapt the histogram topology check for the thin level-0-only
  case. The Qt panel check observes actual Navigator/Histogram sources and the
  histogram image after scaling, including a 4096×2 source; only binning, not
  source dimensions or generation work, is universally bounded. Use narrow test
  hooks only if existing observations cannot expose these facts.
- New `previewStrokeUsesTilesAndRestoresZoom` covers pixel-preview commit/cancel
  with exact CPU replay, cached-tile replacement, history counts and zoom-selected
  levels. Existing mask-preview coverage remains separate.
- New `frameBoundedPaintMatchesCpu` measures pending-region flushes for queued
  dabs and compares the committed CPU result; exercise GPU when available, with
  an explicit GPU-only skip if unavailable rather than silently claiming GPU
  coverage. No legacy self-test entry is added.
