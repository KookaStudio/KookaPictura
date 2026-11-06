# Test Plan

This plan covers `document/plane-access` for `plane-streaming-access` (#1212,
first PR).

Fixtures:
- **Core tests** build a plane `7 × (3·64 − 5)`, fracture it, and fork band 1
  from a held clone. The reference is the materialized slice of a separate
  clone.
- **Render tests** use the same divergence on every layer plane. The
  reference is the same operation on a copy whose planes are flat.

Every check runs under nextest, one process per test, so the process-wide
counter is exact.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `pa_rows` | Read rows `62..131` of the diverged plane with `rows`; read rows past the end; read a flat plane with `rows` | Rows equal the slice's; rows past the end are not yielded; no materialization or mirror | `pictura-core::rows_match_the_materialized_slice_across_band_edges`, `pictura-core::rows_stop_at_the_last_row_and_read_a_flat_plane` |
| `pa_read_rect` | Read a `4 × 66` rect across bands 0–2; read a rect past the last column and row into a buffer filled with `0xAA`; read three empty or outside rects | In-plane samples match; the outside part stays `0xAA`; empty rects copy nothing | `pictura-core::read_rect_into_matches_the_slice_and_clips_to_the_plane`, `pictura-core::read_rect_into_an_empty_rect_copies_nothing` |
| `pa_read_into` | Read a range across the band-0/band-1 edge; read ranges past and beyond the end | Samples match; the part past the end stays untouched | `pictura-core::read_into_copies_a_logical_range_across_bands` |
| `pa_bands` | Visit every band of the diverged plane | Starts are `0, 64, 128`; the concatenation equals the slice | `pictura-core::for_each_band_reports_every_band_and_its_first_row` |
| `pa_mut` | Write two rows of band 2 with `rows_mut`; write rows through `rows_mut` on a uniquely held fractured plane and on a flat plane; write a rect with `write_rect_from` and read it back; add 1 to every sample with `for_each_band_mut` | Written samples land; untouched bands stay shared; the clone keeps its bytes; a unique base keeps its allocation; no materialization | `pictura-core::rows_mut_forks_only_the_bands_it_touches`, `pictura-core::rows_mut_writes_a_unique_base_in_place_and_a_flat_plane`, `pictura-core::write_rect_from_round_trips_and_clips`, `pictura-core::for_each_band_mut_reaches_every_sample_without_materializing` |
| `pa_guard` | Wrap `plane.as_slice()` on the diverged plane in `assert_no_materialization` | The guard panics | `pictura-core::assert_no_materialization_catches_a_whole_plane_copy` |
| `pa_composite` | Composite a base plus a varied layer with a raster mask, all planes diverged, under the guard | No materialization; the composite equals the flat document's | `pictura-render::composite_reads_a_diverged_mask_and_layer_in_place`, `pictura-render::banded_composite_does_not_materialize_a_fractured_plane` |
| `pa_move` | `trim_to_content`, then `untrim_to`, on a diverged layer; `move_selection_content` (cut and merge back) of a `16 × 90` selection by `(2, 70)` | No materialization; layers equal the flat run | `pictura-render::trim_and_untrim_read_diverged_rows_in_place`, `pictura-render::moving_selected_content_merges_into_diverged_rows_in_place`, `pictura-render::content_rect_does_not_materialize_a_diverged_plane` |
| `pa_transform` | `transform_layer_with` at 150 %, 15°, bicubic on a diverged layer with a mask | No materialization; the layer equals the flat run | `pictura-render::free_transform_resamples_a_diverged_layer_in_place` |
| `pa_scenarios` | Run each heap guard at 4250² under nextest | Scenarios in `STREAMED` report 0 materializations | `pictura-app::add_layer_heap_peak`, `pictura-app::edit_fill_heap_peak`, `pictura-app::bucket_in_selection_heap_peak`, `pictura-app::bucket_whole_layer_heap_peak`, `pictura-app::brush_stroke_heap_peak`, `pictura-app::undo_redo_heap_peak`, `pictura-app::snapshot_restore_heap_peak`, `pictura-app::move_layer_heap_peak`, `pictura-app::free_transform_heap_peak` |
| `pa_view` | Run `python3 scripts/profile-tools.py --scenarios M13 --peak --size 4250x4250` and compare with the baseline | Peak added memory stays 0 MB | manual: M13 is Qt-side view navigation with no engine path to repeat in a Rust test; the pyramid code it reads moves in the app follow-up change |
