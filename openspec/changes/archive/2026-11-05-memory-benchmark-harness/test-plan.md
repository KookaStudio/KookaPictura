# Test plan

Script references (`profile-tools::fn`, `gen-large-fixtures::fn`) are `def`s in
`scripts/<name>.py`, run by that script's `--self-check`. `verify-full.sh` runs
both after the build.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mb_peak` | Run M1 and M2 at 512² with `--peak` against the built app | Each row has `peak_mb`, `peak_added_mb`, `rss_mb` and `total_ms`; `peak_mb` ≥ the RSS at the reset, `peak_added_mb` ≥ 0 | `profile-tools::check_peak_row`, `profile-tools::self_check_catalogue` |
| `mb_limit_command` | Build the launch command with and without `--memory-max 4G` | `systemd-run --user --scope … MemoryMax=4G … MemorySwapMax=0 -- app …`; without it, the app alone | `profile-tools::check_limit_command` |
| `mb_limit_guard` | Ask whether 4G fits with 6000 and 5000 MiB available, and 2G with 3600; run the profiler with `--memory-max 6G` on a machine with less free | 4G fits 6000, not 5000; 2G fits 3600; the run exits before launching, naming both amounts; with 5000 MiB free an unlimited memory run gets 3464M | `profile-tools::check_limit_guard` |
| `mb_oom_outcome` | Classify return codes −9 and 137 in setup and in the scenario, 1, no answer, and an error answer from a live app | `oom` with the phase; `crash`; `refused`; the loop goes on to the next scenario | `profile-tools::check_outcomes` |
| `mb_catalogue` | `profile-tools.py --self-check` with a built app: every M1–M13 at 512², M12 from a generated 512² fixture | Every row `outcome: ok` and valid | `profile-tools::self_check_catalogue` |
| `mb_compare` | Compare two in-memory runs: one peak 400 → 200 MB, one outcome ok → oom | The table shows `400 -> 200 (2.0x)` and `ok -> oom` | `profile-tools::check_compare` |
| `mb_peak_alloc` | Allocate, grow and shrink by realloc, free, and reset a `PeakAlloc` over `System` | `current` follows each change; `peak` is the max since the reset; reset sets peak = current | `pictura-testkit::peak_alloc_tracks_alloc_and_free`, `pictura-testkit::peak_alloc_tracks_realloc`, `pictura-testkit::peak_alloc_reset_sets_peak_to_current` |
| `mb_heap_guard` | `cargo nextest run -p pictura_app heap_peak` | M1–M12 each ≤ baseline × 1.10 at 4250² | `pictura-app::new_document_heap_peak`, `pictura-app::add_layer_heap_peak`, `pictura-app::edit_fill_heap_peak`, `pictura-app::bucket_in_selection_heap_peak`, `pictura-app::bucket_whole_layer_heap_peak`, `pictura-app::brush_stroke_heap_peak`, `pictura-app::undo_redo_heap_peak`, `pictura-app::snapshot_restore_heap_peak`, `pictura-app::move_layer_heap_peak`, `pictura-app::free_transform_heap_peak`, `pictura-app::save_psd_heap_peak`, `pictura-app::open_psd_heap_peak` |
| `mb_fixture` | Generate 64² twice with one seed; parse the file back | Identical SHA-256; header 8-bit RGB, 5 layers, RLE row tables whose counts match the data | `gen-large-fixtures::self_check` |
| `mb_baseline` | Read `docs/dev/benchmarks/memory-baseline.json` | `meta.machine` is filled; the `4G` run has an outcome and, if ok, a peak for every M1–M13 | manual: the 17000² run needs the reference machine with its other programs closed, so no CI runner can repeat it |
| `mb_free_transform` | `layer_op free_transform` on a painted square of a 32² layer at 150 %, 15° | The layer's bounds grow and one history state is added | `tst_control_editing::freeTransformScalesAndRotates` |
| `mb_free_transform_busy` | Open Free Transform on the layer, then call `free_transform` | `refused`; the session is still open | `tst_control_editing::freeTransformIsRefusedDuringASession` |
| `mb_new_background` | `document new` 24×16 with `background_layer`; again with a transparent background | One layer named Background, white, one history state; the second call is `invalid_param` | `tst_control_editing::newDocumentCanHaveABackgroundLayer` |
| Full-size heap report | `cargo nextest run -p pictura_app heap_peak_full --run-ignored only` on the reference machine | Each M1–M12 prints its heap peak at 17000² | manual: 17000² does not fit a CI runner's memory |
