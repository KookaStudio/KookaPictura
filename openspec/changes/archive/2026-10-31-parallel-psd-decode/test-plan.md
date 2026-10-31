# Test Plan

One row per scenario in `specs/codec/psd-decode-parallelism/spec.md`. The serial
oracle is the same decode installed on a one-thread rayon pool
(`rayon::ThreadPoolBuilder::new().num_threads(1)`). The parallel/serial
comparisons are unit tests in `crates/pictura-codec/src/read/tests.rs`, except
`pd_budget`, which lives in `tests/hostile_inputs.rs`. The one-thread pool runs
the same algorithm as the parallel path, so the comparison pins order
independence rather than the old bytes; the existing depth and oracle suites pin
the bytes the pre-change decoder produced. Nothing in the production path
selects the serial branch.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pd_layers` | Write fixtures with `write_psd`/`write_psb` for RLE, ZIP, ZIP-with-prediction, 16-bit, 32-bit, PSB, a user mask (-2), a real mask (-3), an extra color channel, and group dividers; build the depth-1 layered fixture by hand in `src/read/tests.rs`; divider records come from `group.psd` and the placeholder-channel branch is not separately fixtured; read each through the parallel path and the one-thread pool | The two `Document` values are equal in every field, including layer channels, mask data, retained source samples, preserved raw channels, and `layer_compression` | `pictura-codec::parallel_layer_decode_matches_serial`, `pictura-codec::a_mask_channel_is_sized_by_the_mask_rect` |
| `pd_layers_slice` | Build the channel task list for a two-layer fixture with unequal channel lengths; the declared lengths come from the layer record, while each channel's compression word and payload come from its own slice; compare each task's recorded offset with the running sum of the preceding declared lengths | Every offset matches the cumulative length; no task shares a cursor with another | `pictura-codec::layer_channel_tasks_record_their_own_offsets` |
| `pd_rle` | Read RLE image data at depth 8, 16, and 32 (multi-channel) and depth-1 Bitmap (one channel), through the parallel path and the one-thread pool | The decoded composite planes are byte-identical | `pictura-codec::parallel_rle_composite_matches_serial` |
| `pd_zip_serial` | Read ZIP and ZIP-with-prediction image data through both paths | The decoded planes are byte-identical; the deflate stream is inflated as one stream | `pictura-codec::merged_zip_composite_matches_serial` |
| `pd_prediction` | Read a depth-8, depth-16, and depth-32 ZIP-with-prediction composite and layer channel through both paths | The recovered planes and the narrowed 8-bit samples are byte-identical | `pictura-codec::parallel_prediction_matches_serial` |
| `pd_errors` | Declare a channel length that exceeds the remaining bytes while the layer-section length still fits (a literal file truncation is caught earlier by the section-length check), truncate inside the RLE count table, truncate inside a scanline, and store a short and an over-long ZIP payload; read each through both paths | Both paths return the same `PsdError` variant | `pictura-codec::parallel_decode_reports_the_same_error` |
| `pd_errors_order` | Build a file whose first channel is malformed with one variant and whose second channel is malformed with a distinguishable variant, and read it through both paths | The earlier channel's error is the one returned; the test asserts the returned error and does not force out-of-order scheduling | `pictura-codec::parallel_decode_reports_the_first_error_in_order`, `pictura-codec::merged_rle_reports_the_first_row_error` |
| `pd_budget` | Build a file whose earlier channel is malformed and whose later channel exceeds the per-allocation budget; read it under an explicit `ImageBudget` | The read fails with the budget error and the malformed channel is not decoded | `pictura-codec::over_budget_channel_is_refused_before_decoding` |
| `pd_budget_existing` | Run the existing untrusted-input budget scenarios after the change | Every one reports its recorded result | `pictura-codec::a_psb_header_past_the_budget_is_refused_before_allocating`, `pictura-codec::a_composite_over_the_document_total_is_refused`, `pictura-codec::a_document_over_its_limit_is_refused_and_the_default_keeps_a_plane`, `pictura-codec::a_huge_rle_layer_rect_is_refused`, `pictura-codec::a_huge_zip_layer_rect_is_refused`, `pictura-codec::a_huge_bitmap_layer_rect_is_refused`, `pictura-codec::a_huge_mask_rect_is_refused`, `pictura-codec::a_rect_spanning_the_whole_i32_range_is_refused`, `pictura-codec::layer_planes_count_against_the_document_total`, `pictura-codec::a_canvas_class_document_header_still_opens`, `pictura-codec::mutated_psds_never_panic_or_overallocate` |
| `pd_serial` | Decode the layer, merged RLE, and prediction fixture sets with the parallel path and with the decode confined to one thread | Every output byte and the returned error are identical | `pictura-codec::parallel_layer_decode_matches_serial`, `pictura-codec::parallel_rle_composite_matches_serial`, `pictura-codec::parallel_prediction_matches_serial`, `pictura-codec::parallel_decode_reports_the_same_error` |

## Manual checks

None. Every scenario is automated; the change has no visual or interactive
surface. The ignored `profile_psd_decode_stages` timing test is a measurement,
not an acceptance check: it records the per-stage numbers in the PR body so the
issue's estimates are replaced with measured values.
