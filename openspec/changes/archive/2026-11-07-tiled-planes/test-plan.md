# Test Plan

This plan covers `document/tiled-planes` and the modified
`document/plane-access` for `tiled-planes` (#1213, with the rest of #1212).

Fixtures:
- **Core.** Tests build planes at sizes that are not multiples of 256, tile
  them, and fork a tile from a held clone. The reference is a `Vec` that
  received the same writes, or a flat plane.
- **Integration.** Every check that asserts "no materialization" runs under
  nextest, one process per test, so the process-wide counter is exact.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `tp_oracle` | 10 000 seeded ops (rect, row, tile writes; reads; clone-then-write; settle) at 1×1, 255×257, 256², 300×700, and 3×300²; for u8, u16 and f32 | Every read equals the `Vec`, bit for bit | `pictura-core::matches_vec_reference` |
| `tp_channels` | Tile three stacked 300² channels; write the last row of channel 0 | 2×2 tiles per channel; channel 1–2 tiles stay shared | `pictura-core::tiles_restart_at_each_channel` |
| `tp_fork` | Clone a varied 1024² plane, write one sample | One tile no longer shared, 65 536 B new; the original keeps its bytes | `pictura-core::one_pixel_write_forks_one_tile` |
| `tp_collapse` | Three channels: all 7, all 7 but one sample, all 0; then fix the odd sample and settle | `Uniform(7)`, one `Dense`, `Empty`; the fixed tile collapses | `pictura-core::uniform_collapse_per_channel`, `pictura-core::a_write_of_the_uniform_value_forks_nothing` |
| `tp_empty` | `filled` 17000² at 255 and at 0; tile a flat zero plane | No allocation; zero tiles are `Empty`; samples read back | `pictura-core::default_value_tiles_are_empty`, `pictura-core::build_tiled_fills_strips` |
| `tp_identity` | Write a stamped clone through `tiles_mut`, `write_rect_from`, `write_row_from` and `share_tile_from` | Stamp and ident cleared; unwritten tiles keep their generations; the original keeps all | `pictura-core::stamp_and_ident_cleared_by_tile_writes` |
| `tp_generation` | Write a tile to one value through `tiles_mut`, stamp it, settle | Collapses to `Uniform`; generation, stamp and ident unchanged | `pictura-core::tile_generation_survives_clone_and_settle` |
| `tp_background` | New white RGB document at 4250² (17000² ignored) | Background planes ≤ 4 MiB (measured 0 B) | `pictura-app::background_plane_cost`, `pictura-app::background_plane_cost_full` |
| `tp_new_layer` | Add a layer; count layer planes plus history-retained planes before and after | ≤ 1 MiB added (measured 0 B) | `pictura-app::new_layer_plane_cost`, `pictura-app::new_layer_plane_cost_full` |
| `tp_fill` | Edit › Fill of the centre square inside a marquee; Paint Bucket in the selection | Layer planes added ≤ 1.2 × tile-rounded square × 4; bucket heap guard holds | `pictura-app::fill_plane_cost`, `pictura-app::fill_plane_cost_full`, `pictura-app::bucket_in_selection_heap_peak` |
| `tp_stroke` | The same stroke on a 1024² and a 4096² full-canvas layer held by a clone | Forked tile count is equal and nonzero | `pictura-paint::a_stroke_costs_the_tiles_it_covers_not_the_document_width`, `pictura-app::brush_stroke_heap_peak` |
| `tp_history` | Capture a 600² layer, write one sample, capture again | Every other tile is the same allocation in the old state, the new state and live | `pictura-app::untouched_tile_keeps_its_arc`, `pictura-app::a_hinted_capture_shares_its_fully_hinted_tiles` |
| `tp_gate` | `fork_bytes` for a 10×10 write in one tile at depths 8, 16 and 32 | Planes × 65 536 × 1, 2 and 4 | `pictura-app::fork_bytes_counts_tiles`, `pictura-app::a_write_forks_the_tiles_it_crosses` |
| `tp_time` | `python3 scripts/profile-tools.py --scenarios memory --compare docs/dev/benchmarks/memory-baseline.json` and the brush scenarios, before and after | No scenario +10 %; dab latency within 5 % | manual: wall-clock timing on a quiet machine; not deterministic enough for CI |
| `tp_psd_write` | Write a layered document flat, then tiled with every plane diverged, under the guard | Byte-identical; no materialization | `pictura-codec::psd_write_is_byte_identical_tiled` |
| `tp_psd_read` | Write and read a white 600×520 document with a small red patch | Layer planes tiled; white tiles `Uniform`; the patch tile `Dense`; samples read back | `pictura-codec::reading_fills_tiles` |
| `pa_rows` | Copy rows `254..515` of a diverged tiled plane; ask for a row past the end | Rows equal the slice; the past-the-end buffer is untouched; no materialization | `pictura-core::rows_copy_out_of_a_diverged_plane_across_tile_edges` |
| `pa_read_rect` | Read a rect across tile edges and one past the last column and row | In-plane samples match; the outside stays `0xAA`; empty rects copy nothing | `pictura-core::read_rect_into_matches_the_slice_and_clips_to_the_plane`, `pictura-core::read_rect_into_an_empty_rect_copies_nothing` |
| `pa_read_into` | Read a range across a tile-row edge; ranges past the end | Samples match; the part past the end is untouched | `pictura-core::read_into_copies_a_logical_range_across_tiles` |
| `pa_bands` | Visit every tile of the diverged plane; visit a flat plane | Six tiles with their rects, which reassemble the slice; a flat plane gives 256-row strips | `pictura-core::for_each_tile_visits_every_tile_with_its_position`, `pictura-core::a_flat_plane_presents_full_width_strips` |
| `pa_mut` | `write_row_from`, `for_each_tile_mut`, `write_rect_from` on the diverged plane; the same on a flat plane | Written samples land; untouched tiles stay shared; the sharer keeps its bytes | `pictura-core::a_mutable_access_forks_only_the_tiles_it_touches`, `pictura-core::write_rect_from_round_trips_and_clips`, `pictura-core::flat_planes_serve_the_same_access` |
| `pa_guard` | `as_slice` on a tiled plane inside the guard | The guard panics under nextest | `pictura-core::assert_no_materialization_catches_a_whole_plane_copy` |
| `pa_composite` | Composite a document whose planes are all diverged tiles | No materialization; equals the flat composite | `pictura-render::composite_reads_a_diverged_mask_and_layer_in_place`, `pictura-render::banded_composite_does_not_materialize_a_fractured_plane` |
| `pa_move` | Trim/untrim and move selected content on diverged tiles | No materialization; equals the flat run | `pictura-render::trim_and_untrim_read_diverged_rows_in_place`, `pictura-render::moving_selected_content_merges_into_diverged_rows_in_place`, `pictura-render::content_rect_does_not_materialize_a_diverged_plane` |
| `pa_transform` | 150 %, 15°, bicubic on a diverged tiled layer and mask | No materialization; equals the flat run | `pictura-render::free_transform_resamples_a_diverged_layer_in_place` |
| `pa_scenarios` | Run each heap guard at 4250² under nextest | M1–M12 report 0 materializations | `pictura-app::new_document_heap_peak`, `pictura-app::add_layer_heap_peak`, `pictura-app::edit_fill_heap_peak`, `pictura-app::bucket_whole_layer_heap_peak`, `pictura-app::undo_redo_heap_peak`, `pictura-app::snapshot_restore_heap_peak`, `pictura-app::move_layer_heap_peak`, `pictura-app::free_transform_heap_peak`, `pictura-app::save_psd_heap_peak`, `pictura-app::open_psd_heap_peak` |
| `pa_view` | Run `python3 scripts/profile-tools.py --scenarios M13 --peak --size 4250x4250` and compare with the baseline | Peak added memory stays 0 MB | manual: M13 is Qt-side view navigation with no engine path to repeat in a Rust test |
