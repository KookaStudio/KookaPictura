# Tasks

## 1. Tests first

- [x] 1.1 Add the four std-only level-handle tests to `view_pyramid.rs` (`a_stored_level_view_shares_the_level_bytes`, `an_update_copies_a_level_a_live_view_holds`, `an_update_without_a_view_writes_in_place`, `level_zero_has_no_borrowed_view`); verify the four names match `test-plan.md` and exist in `view_pyramid.rs` (grep); compilation is verified by task 2.1
- [x] 1.2 Add the app-level tests `a_level_zero_damage_crop_matches_the_cached_crop_rect` and `sample_argb_after_a_region_refresh_reads_the_composite` to `canvas_view_test.rs`; verify the two names match `test-plan.md` and exist in `canvas_view_test.rs` (grep); compilation is verified by task 2.2
- [x] 1.3 Create `crates/pictura-app/cpp/tests/tst_present_cache.cpp` with the cases `documentRefreshArrivesAsDamage`, `nonDocumentCanvasBlitsPixels`, `documentRefreshMatchesFullRecomposite`, `presentDoesNotBuildTheFullImage`, `storedLevelIsDrawnFromALevelView`, `storedLevelViewMatchesTheCropDraw`, `level0CropIsReusedAcrossPaints`, `zoomChangeSelectsTheNewLevel`, `level0PresentMatchesTheTransformDraw`, `largeDocumentPresentsThroughALevel`, `panPresentsWithoutResamplingTheDocument`, `idlePresentLevelIsZoomSelected`, `level0PatchMatchesFreshCrop`, `damageOutsideCacheChangesNothing`, `damageForAnotherLevelDiscardsCrop`, `filterCommitPresentsThroughDamage`; register the file in `crates/pictura-app/cpp/tests/CMakeLists.txt` and its shard entries; verify `python3 scripts/check-test-plan.py` resolves every reference

## 2. Borrowed level storage

- [x] 2.1 Change `PyramidLevel` storage to copy-on-write and add the std-only shared level handle (shared bytes plus width/height) to `ViewPyramid`; verify `cargo nextest run -p pictura-render view_pyramid` (the four group-1 tests green)
- [x] 2.2 Wrap the shared level handle as a premultiplied RGBA8888 `QImage` in `pictura-app` (`QImage::from_raw_parts` with a cleanup that keeps the handle alive), expose it through the app bridge, choosing the solo-mask pyramid while a mask is soloed and the composite pyramid otherwise; verify `cargo nextest run -p pictura-app canvas_view` and `cargo clippy -p pictura-app -p pictura-render --all-targets -- -D warnings`

## 3. Damage notification

- [x] 3.1 Add the `regionDamaged(x, y, w, h)` signal and switch every document region emitter (`refresh_region`, `refresh_regions`, preview present, edit/preview, history restore, cancel paint) to it, skipping the region display image for document views; `regionBlitted` is no longer emitted for a document; verify `cargo nextest run -p pictura-app` and that no `premultiplied_display_image` remains on the document refresh path
- [x] 3.2 Wire the damage rectangle through `frame_canvas.cpp` to the canvas (including the filter-commit damage signal) and keep the pixel blit path for a canvas that holds its own image; verify `tst_present_cache::documentRefreshArrivesAsDamage` and `tst_present_cache::nonDocumentCanvasBlitsPixels`
- [x] 3.3 Route the filter-commit path through the damage notification: carry the `clipped` rectangle out of `Refresh::Region` in `Landed::Region`, emit `regionDamaged` with it, and delete the 1×1 marker pixel image from `filter_tools/jobs.rs`; verify `tst_present_cache::filterCommitPresentsThroughDamage`
- [x] 3.4 Migrate existing Qt spies from `regionBlitted` to `regionDamaged` where they observe a document refresh, and adapt the affected self-test checks without adding new ones: `selftest.cpp` (check 76), `selftest_paint_perf.cpp` (344/345/514/544/545), `selftest_canvas_view.cpp` (541), `selftest_canvas.cpp`, `selftest_large_doc.cpp`, and `selftest_visibility.cpp`. Once levels below 0 have no pixel cache, `presentCacheImageSize` reports the presented level rectangle and `presentCacheRebuiltOnLastPaint` means the presented level/rectangle changed or a level-0 crop was produced. Verify `ctest --test-dir build -R '^tst_(document_session|edit_refresh|mask_edit|layers_preview_coalescing)$' --output-on-failure` and `bash scripts/test-report.sh`

## 4. Zero-copy present

- [x] 4.1 Extend the level provider with the app-side QImage and make the paint draw a stored level through a source rectangle instead of a crop, keeping the level-0 crop path and the single-channel crop path. The borrowed view is created and dropped inside one present step and MUST NOT be retained in a member across paints, or the next dab pays a copy-on-write level copy. `tst_present_cache::storedLevelIsDrawnFromALevelView` asserts the canvas pixels across an update; verify also `tst_present_cache::panPresentsWithoutResamplingTheDocument`, `tst_present_cache::largeDocumentPresentsThroughALevel`, and `tst_present_cache::zoomChangeSelectsTheNewLevel`
- [x] 4.2 Remove the `-1` GPU present level so a live GPU stroke uses the zoom-selected level; verify `tst_present_cache::idlePresentLevelIsZoomSelected`, `pictura-app::level0_region_update_matches_a_rebuild`, and the GPU paint-perf self-test check
- [x] 4.3 Verify the zero-copy draw is pixel-identical to the crop draw at several zooms and offsets in `tst_present_cache`; if it is not, keep the crop cache for levels below 0 and patch it from damage instead (this is a design fallback, not a new task)

## 5. Level-0 patch

- [x] 5.1 Add the canvas damage entry point: map the damage rectangle to the cached crop's level, re-crop only that rectangle, row-copy it into the cache, advance the cache revision, and discard the cache when the present level differs; verify `pictura-app::a_level_zero_damage_crop_matches_the_cached_crop_rect` (positional premise), `tst_present_cache::level0PatchMatchesFreshCrop` (in-place byte identity), `tst_present_cache::damageOutsideCacheChangesNothing`, `tst_present_cache::damageForAnotherLevelDiscardsCrop`
- [x] 5.2 Limit the pixel cache to level 0 and shrink the present test hooks accordingly. `setPresentCacheEnabledForTest(false)` and `setPresentLevelCropForTest(false)` bypass BOTH the level-0 crop cache and the borrowed level view, so the direct-draw comparisons stay direct at any level; verify `tst_present_cache::level0CropIsReusedAcrossPaints`, `tst_present_cache::level0PresentMatchesTheTransformDraw`, `tst_present_cache::presentDoesNotBuildTheFullImage`
- [x] 5.3 Keep the existing region-equality coverage green after the cache change; verify `pictura-app::paint_commit_region_patch_equals_a_full_recomposite`, `pictura-app::image_after_region_refresh_matches_a_full_recomposite`, `pictura-app::region_refresh_reuses_the_level0_frame_when_it_matches`, `tst_large_document::aDocumentCanvasHoldsNoImage`

## 6. Integration

- [x] 6.1 Run the full test layers; verify `cargo nextest run --workspace` and `cargo test --workspace --doc`
- [x] 6.2 Run the acceptance profile: 500 px brush on 6000×4000 at fit zoom, GPU on and off, `PICTURA_METRICS=1`; verify per-frame `cxx_paintEvent` stays under 5 ms and no per-blit region image is counted
- [ ] 6.3 Run the repository gates; verify `bash scripts/verify-fast.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`
