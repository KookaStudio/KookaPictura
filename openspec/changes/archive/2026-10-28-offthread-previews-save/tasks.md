# Tasks

Tests-first tasks below are written against the API skeleton the implementation
tasks introduce; the workspace build, doctests, and Qt suites run only in group 6.

## 1. Test plan and harness

- [x] 1.1 Write `test-plan.md` with one row per scenario in the five spec deltas, including the scenarios copied by the `MODIFIED` requirements, and confirm every Coverage cell names a test that this task list creates or an existing test that already covers it
- [x] 1.2 Create the `tst_offthread_jobs.cpp` suite shell, register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`, and verify CMake configures and builds the empty suite

## 2. Codec encode cache

- [x] 2.1 Add `pictura-codec` tests named `a_cache_hit_equals_a_fresh_encode`, `an_edited_plane_is_re_encoded`, `parallel_encoding_matches_serial`, `a_channel_error_fails_the_encode` (a direct `encode_scanlines` call with a synthetic oversized row, or a channel-length error), `unused_entries_are_pruned`, `only_the_edited_layer_is_encoded`, `unchanged_cmyk_layer_reuses_retained_planes`, and `edited_cmyk_layer_is_converted`; use the conversion counter in the CMYK reuse test and verify each named test exists and fails against the uncached writer
- [x] 2.2 Move the layer-section writer (`OutRecord`, `flatten`, `record_name`, `lab_layer_color_planes`, `write_layer_info`, `write_record`, `write_extra`, `write_pascal`, `empty_layer`, `luni_data`) into `write_layers.rs` with no behavior change; declare `mod write_layers;` in `lib.rs`; make `native_plane`, `channel_stream`, `layer_retained`, and `write_tag` reachable as needed (`pub(crate)`); verify the existing PSD round-trip and psd-tools tests still cover the moved code
- [x] 2.3 Add `write_cache.rs` with `EncodeCache`, the layer-channel key, the whole-composite block key, `EncodeStats`, get/put/prune, and the `write_psd_cached` / `write_psb_cached` entry points; declare `mod write_cache;` in `lib.rs`; verify `pictura-codec` cache tests compile against the new API
- [x] 2.4 Plan per-layer channels and cache direct 8-bit color channels and raster masks; cache the whole composite image-data block as one entry keyed by the composite identity, pixel dimensions, output mode, compression, depth, PSB flag, and channel count, skipping synthesized Lab/CMYK/Indexed composite blocks; prune to the keys each successful encode used; verify the hit-bytes and prune tests
- [x] 2.5 Add `rayon` to `crates/pictura-codec/Cargo.toml` with the reason in the commit, and verify the crate still builds with the workspace dependency
- [x] 2.6 Encode layer channel tasks with `rayon` in plan order, reuse one PackBits row buffer, and scan errors in task order; verify the parallel-equality and error tests
- [x] 2.7 Add `SourceChannels.working_idents` with a manual `PartialEq` that ignores it; update the five existing struct literals (`pictura-core/src/tests.rs`, `pictura-render` crop/resize/transform tests); capture the working-plane idents after mode normalization; use them in `write_cmyk` and `lab_layer_color_planes` for the unchanged/edited branches; leave the composite forward checks out of scope; add the test-only CMYK conversion counter (or poisoning-profile seam) and use it in `unchanged_cmyk_layer_reuses_retained_planes`; verify the CMYK and Lab identity tests and the existing CMYK fixture oracle
- [x] 2.8 Add an ignored 24 MP encode benchmark named `profile_24mp_cold_and_warm_encode` that builds a 20-layer 6000×4000 RGB document, encodes cold and warm, edits one layer, and reports both timings plus `layers_encoded`; verify it runs with `--ignored --nocapture` and prints the counters

## 3. Save job and session cache

- [x] 3.1 Add Rust tests for the save job (start while running, finish writes and clears, stale snapshot stays modified, failure leaves path and dirty, `a_replaced_document_keeps_its_path_and_dirty_state`, and `a_synchronous_save_writes_the_worker_bytes`) and Qt cases in `tst_offthread_jobs.cpp` named `saveKeepsTheEventLoopLive`, `aCompletedSaveWritesAndClearsModified`, `aFailedSaveLeavesTheDocumentUntouched`, and `anEditDuringSaveLeavesTheDocumentModified`; verify the named tests exist
- [x] 3.2 Add `Arc<EncodeCache>` to `DocumentSession`, replace it on install, and route `recovery_write` through `write_psd_cached`; the snapshot's encode prunes like any successful encode but does not clear the cache; verify `a_snapshot_reuses_the_encode_cache`
- [x] 3.3 Add `impl_core/save_jobs.rs` with start/running/finish/set-delay, add the save job handle and delay seam to `state.rs`, declare `mod save_jobs;` in `impl_core.rs`, and register the bridge in `crates/pictura-app/build.rs`; verify the Rust save-job tests
- [x] 3.4 Split the native path in `impl_core/save.rs` into start plus join, keeping raster and PDF inline and the synchronous `save` result unchanged; verify `tst_document_api::saveAndReopenRoundTrip` and the save round-trip tests
- [x] 3.5 Add `busy_job.{h,cpp}` to `CMakeLists.txt`, move the filter-commit loop onto the helper, and verify the shell builds with the new source before the save cases run
- [x] 3.6 Add the `document_api` save start/running/finish wrappers and route native paths in `frame.cpp` through the busy loop, including the smart-object temp `.psd` save at `frame.cpp:506`; record the frame's document path and title only when the save applied to the current or stale same document, so a replaced document keeps its own path and title; drop the recovery snapshot only when the save leaves the document clean; keep the control server on the frame path so its save requests are serialized and block until the save lands; verify the save Qt cases

## 4. Adjustment jobs

- [x] 4.1 Add the `waitForPreview` helper to `tst_image_adjustments.cpp` and wrap every existing synchronous preview assertion (`dialogPreviewsCancelsAndApplies`, `colorLookupPresetRebuildsTheLook`, `brightnessContrastPreviewsAndApplies`, the Levels/Curves/Exposure/Vibrance/Hue-Saturation/Color-Balance/Channel-Mixer/Black-White/Gradient-Map/Photo-Filter cases) with it; verify the existing tests pass under the async preview
- [x] 4.2 Add the adjustment cases to `tst_offthread_jobs.cpp`, named `adjustmentPreviewKeepsTheEventLoopLive`, `latestAdjustmentPreviewWins`, `adjustmentApplyKeepsTheEventLoopLive`, and `maskAdjustmentPreviewStaysSynchronous`; verify the named tests exist
- [x] 4.3 Generalize the `filter_tools/jobs.rs` commit to `ActiveOp` and expose the shared running/finish helpers; verify the existing filter commit tests still name the same behavior
- [x] 4.4 Add `image_adjust/jobs.rs` with preview start/poll/busy/cancel, apply start/running/finish, preview and apply set-delay seams, add the preview/apply handles and delay fields to `state.rs`, declare `mod jobs;` in `image_adjust.rs`, and register the bridge in `build.rs`; verify the Rust preview and commit tests
- [x] 4.5 Expose `PreviewBaseline::save` for a whole-layer baseline; have the worker return the composited changed region (`Refresh::Region`-style) or refresh only the visible section, so the GUI never composites the full layer rect; verify the landing, cancel bit-identity, and rapid-tick tests
- [x] 4.6 Add the 0 ms coalescing timer, the 16 ms poll timer, and the async accept busy loop to `adjustment_dialog.cpp`; verify the Qt adjustment cases, including the existing preview/cancel/apply test
- [x] 4.7 Keep the synchronous mask-sized path for a layer-mask target and verify `maskAdjustmentPreviewStaysSynchronous`; if `impl_filters.rs` needs more than a visibility change, move the `PreviewBaseline` impl to `document_session/previews.rs` first

## 5. Whole-layer filter preview

- [x] 5.1 Add the `filter_exact_preview_set_delay` seam and the Qt cases `largeWholeLayerPreviewKeepsTheEventLoopLive` (including the assertion that the landed picture replaces the canvas), `largeWholeLayerPreviewCancelLeavesTheDocument`, `largeWholeLayerPreviewCommitReusesTheResult`, and `maskFullZoomPreviewStaysSynchronous`; verify the named tests exist
- [x] 5.2 Add `prepareExactPreview` and the over-4-MP whole-layer branch at 100% zoom or above to `filter_preview_dialog.cpp`: set `proxyArea_`, `proxyScale_`, and `proxyShown_` without drawing a draft, cap the region and scale to `kCanvasProxyPixels`, let `pollExactPreview` install the picture, and fall back to the synchronous path when `filter_exact_preview_start` returns false for a layer-mask target; leave the small-document and below-100% paths unchanged; verify the new cases plus the existing lens-blur, shear, and filter-menu preview tests

## 6. Integration

- [x] 6.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`
- [x] 6.2 Configure and build CMake, then run `ctest --test-dir build -R '^tst_' --output-on-failure`
- [x] 6.3 Run `bash scripts/verify-fast.sh` (or `verify-full.sh` when the CMake build is stale) and confirm `scripts/check-file-size.sh`, the selftest budget, and `openspec validate --all --strict` pass
- [x] 6.4 Run the ignored 24 MP encode benchmark (`cargo test -p pictura-codec -- profile_24mp_cold_and_warm_encode --ignored --nocapture`) and record the cold/warm timings and `layers_encoded` in the PR body; do not claim app metrics
- [x] 6.5 Archive the change with `openspec archive offthread-previews-save` and confirm only `openspec/changes/archive/` remains
