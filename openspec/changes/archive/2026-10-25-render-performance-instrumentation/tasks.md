# Tasks

No build or test runs during implementation (AGENTS.md workflow step 4). Every
check below runs once, in the final verification group.

## 1. Tests first

- [x] 1.1 Write the `pictura-app` metrics and transaction tests named in test-plan.md (metrics state, record formatting, `an_inserted_outcome_drops_links_without_compositing`, `new_layer_keeps_composite_exact`, `new_group_keeps_composite_exact`) in sibling test files
- [x] 1.2 Write the `pictura-render` tests (`largest_canvas_covers_full_document`, `dissolve_decline_is_counted`, the five predicate tests, and the `neutral_insertions_preserve_composite` property test)
- [x] 1.3 Add `tst_control_metrics` and `tst_control_metrics_off` and register both in `crates/pictura-app/cpp/tests/CMakeLists.txt`; add the `tst_edit_refresh` and `tst_layer_commands` cases

## 2. Metrics facility (#651)

- [x] 2.1 Add `cxxqt_object/metrics.rs` (level, phase totals, edit record ring, line formatting) and make `paint_timing::record` forward to it, keeping the stroke report
- [x] 2.2 Add `pictura_render::metrics` (always-on resident, decline, and largest-canvas counters; `enabled()`; the device-time slot); replace `RESIDENT_HITS`; count declines in `composite_gpu` / `composite_gpu_region` where `check_supported` fails
- [x] 2.3 Open and close edit records in the transaction (`apply_edit`), mark `composited` in `apply_edit`, and print `pictura metrics:` lines
- [x] 2.4 Add the `record_metric_phase` invokable and `MetricsTimer` in `pictura_debug_timing.h`, and time `LayersPanel::refresh`
- [x] 2.5 Request `TIMESTAMP_QUERY` only when metrics are on and the adapter supports it; write the begin/end timestamps around the composite dispatches, resolve them with the readback, and publish the device time
- [x] 2.6 Add the control-server `metrics` method (snapshot, `reset`, `refused` when off) and the `layer_op add_group` op
- [x] 2.7 Add `--metrics` and the `new-layer` / `new-group` scenarios to `scripts/profile-tools.py`
- [x] 2.8 Document `PICTURA_METRICS`, the `metrics` method, and `profile-tools.py --metrics` in `DEVELOPING.md`

## 3. New Layer latency (#379)

- [x] 3.1 Add `pictura_render::insertion_is_content_neutral` in `document_ops/layer_ops/neutral.rs`
- [x] 3.2 Add `Edit::Inserted` (links and solo snapshot cleared, the mask-solo source re-resolved when soloed, one state, Notify, no composite) to `apply_edit` and `preview_edit`, and return it from every creation handler when the predicate holds
- [x] 3.3 Coalesce the `LayersPanel` refresh so a synchronous refresh consumes the queued one, and drop `retargetDock`'s duplicate `refresh()` after `setView` (design D6b)

## 4. Docs and roadmap

- [x] 4.1 Split the ROADMAP.md #651 line: tick "metrics" and list the remaining ARCH-013 items, each linked to its follow-up issue (needs the issues from 4.3)
- [x] 4.2 Mark XC-11 as partial in `docs/dev/doc-code-parity-gaps.md` and update `docs/dev/STATE.md`, in a separate `TASK-ALLOWS-DOCS` commit
- [x] 4.3 Open follow-up issues for frame-budget instrumentation, TileCache, scratch paging, the tile scheduler, and filter cancellation (outward-facing, so confirm with the user first)

## 5. Verification (run once, after all code is written)

- [ ] 5.1 `cargo fmt --all`, clippy, `cargo nextest run -p pictura-render -p pictura_app`, CMake build, and the new and updated Qt suites; `scripts/check-test-plan.py` and the file-size check pass
- [x] 5.2 Profile New Layer and New Group on a 24 MP document before and after (`profile-tools.py --metrics`) and quote both breakdowns in the PR body; if `transparent_layer` (suspect E) dominates, open the empty-rect follow-up change
