# Test plan

This plan covers the requirements this change adds to
`document/long-operation-progress` for `whole-image-op-progress` (#1278, PR 2).

**Fixtures.** Engine tests use a three-layer 48² document. App Rust tests start
the job directly on a 64² two-layer view, which bypasses the size check (that
check has its own test). Qt tests use a 2400 × 1800 white fixture, the smallest
non-square size over the 4 MP threshold, and dispatch the real menu commands.
They hold the job with `document_job_set_delay` and cancel through the
`jobProgress` Cancel button.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lop_whole_image_job` | Dispatch Rotate 90° CW on the 2400 × 1800 fixture with the job held 800 ms; separately, check which views want a job | Window shown with a counted value; 1800 × 2400 with one `"Rotate"` state; only an armed large view wants a job | `tst_document_jobs::aRotateRunsAsAJob`, `pictura-app::a_document_job_lands_one_history_state`, `pictura-app::only_an_armed_command_on_a_large_document_leaves_the_gui_thread` |
| `lop_whole_image_cancel` | Dispatch Flatten on a two-layer fixture held 30 s; press Cancel; then flatten again | Layers, history count and index, and pixels unchanged; second flatten adds one state; a closed document drops its job | `tst_document_jobs::cancellingAFlattenChangesNothing`, `pictura-app::a_cancelled_document_job_changes_nothing`, `pictura-app::a_closed_document_drops_its_job` |
| `lop_merge_job` | Start a Merge Down job on a two-layer view and land it | Outcome path is the target, count 2, one layer left, one added state | `pictura-app::a_merge_job_reports_the_merged_layer` |
| `lop_small_sync` | Dispatch Rotate on a 64 × 48 fixture with the delay seam at 800 ms | No window, returns in under 800 ms, rotated, one state | `tst_document_jobs::aSmallDocumentAppliesAtOnce` |
| `lop_counted_progress` | Run each engine under `job::scope` on the 48² three-layer document | `permille()` is 999 for each | `pictura-render::image_size_counts_every_resampled_row`, `pictura-render::rotate_and_flip_count_layers`, `pictura-render::merge_and_flatten_count_composited_rows` |
| `lop_busy_bar` | Run `runCancellableJob` reporting -1 for 600 ms; dispatch Mode > 16 Bits/Channel on the large fixture | Window shows `maximum() == 0`, never 1000; the conversion lands as one state | `tst_busy_job::aNegativeProgressShowsABusyBar`, `tst_document_jobs::aDepthConversionShowsABusyBar` |
| `lop_engine_stops` | Run Image Size, Rotate and Flatten under a pre-cancelled token | `Err(Cancelled)`; progress 0 for Image Size and Flatten; layer rects unchanged for Rotate | `pictura-render::a_cancelled_engine_stops_and_reports_cancelled` |
