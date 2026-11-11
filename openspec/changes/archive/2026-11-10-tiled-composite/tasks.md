# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `composite_region_matches_full_crop`, `composite_region_folds_odd_edges`, `composite_region_outside_is_transparent`, `streamed_region_equals_unbanded`, `flatten_composites_by_band`.
- [x] 1.2 `pictura-render` tile cache: `damage_repairs_cached_tiles`, `a_stale_tile_is_not_installed`, `the_cache_keeps_to_its_budget`, `pressure_drops_unpinned_tiles`, `hits_and_misses_are_counted`.
- [x] 1.3 `pictura-core`: `the_display_budget_is_a_quarter_of_the_ceiling`.
- [x] 1.4 `pictura-codec`: `streamed_merged_image_is_byte_identical`.
- [x] 1.5 `pictura-app`: `tile_crops_match_a_whole_rebuild`, `only_visible_tiles_are_built`, `no_full_resolution_frame_is_kept`, `an_edit_repairs_the_visible_tiles`, `history_presents_current_tiles`, `a_missing_large_tile_does_not_block`, `small_documents_build_tiles_synchronously`, `solo_mask_restore_presents_visible_tiles`, `sample_all_reads_regions`, `save_streams_the_merged_image`, `save_allocates_no_whole_composite`; adapt the `picture.rs` tests to regions.
- [x] 1.6 Qt: `tst_eyedropper::sampleAllLayersReadsTheComposite`, `tst_canvas_pan::panShowsTheDocumentPixels`, `tst_canvas_mask_domain::soloMaskRestoreShowsTheMask`.

## 2. Render

- [x] 2.1 `composite_region(doc, rect, level)` with banded streaming and odd-edge folding.
- [x] 2.2 `TilePyramid`: keys, damage generations, epoch, pins, LRU, budget, counters, damage folding.
- [x] 2.3 `DocumentComposite` authority rule; flatten by band; remove `LevelView`.

## 3. Core and codec

- [x] 3.1 `Governor::display_budget`; `Document::composite_is_authoritative`.
- [x] 3.2 PSD/PSB save: the merged image composited band by band into tiles (`save_view`, `composite_tiled`); the writer is unchanged, so its bytes are too. Open skips the merged image (`skip_layered_merged`).

## 4. App

- [x] 4.1 Display state: one `TilePyramid` replaces `level0`, `pyramid`, `mask_level0`, `mask_pyramid` and the deferred pyramid.
- [x] 4.2 Present path, damage folding, worker tile builds; C++ level provider without `levelImage`. (No open pre-build: the first paint starts the worker for the visible tiles; see design §3.)
- [x] 4.3 Solo mask as a tile source.
- [x] 4.4 `picture()` as regions and bands; Sample All, eyedropper, Info, histogram.
- [x] 4.5 Save band source; flatten.
- [x] 4.6 Stop maintaining `doc.composite` for layered documents; history; `materialize_composite` on `ponytail:` paths; memory gate.

## 5. Verification and close-out

- [x] 5.1 fmt, clippy, nextest, CMake build, Qt suites (254/255; `tst_open_save::rasterSaveKeepsFileInfo` needs the Qt TIFF plugin, absent locally).
- [x] 5.2 4250² heap guards; 17000² M1/M2/M5/M7/M8/M11/M12/M13 and solo-mask profiles; results in `design.md`. M12 not met (#1273); M13 frame time and the GPU comparison are manual.
- [x] 5.3 `gpu-rendering-pipeline.md` (`TASK-ALLOWS-DOCS` commit); `ROADMAP.md`; `STATE.md`.
- [x] 5.4 Follow-up issues: #1274 (`ponytail:` composite readers), #1273 (M12 PSD decode).
- [x] 5.5 `openspec validate --all --strict`; archive.
