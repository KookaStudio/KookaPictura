# Test plan

Every spec scenario is automated. The psd-tools checks self-skip with a clear
message when `python3` plus `psd-tools` is unavailable, following the existing
oracle convention; CI's oracle job runs them.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `slf_read` | Read `crates/pictura-codec/tests/fixtures/vector_fill.psd` and inspect both layers. | `Shape` has the pixel-data-irrelevant flag set; `Base` has it clear. | `pictura-codec::codec_reads_and_round_trips_the_pixel_data_irrelevant_flag` |
| `slf_round_trip` | From that document, write and reread PSD; for PSB, write and reread, then compare with the input after setting the expected document's `is_psb` to `true`. | The PSD reconstruction equals the input; the PSB reconstruction equals the normalized input; `Shape` still has the flag set. | `pictura-codec::codec_reads_and_round_trips_the_pixel_data_irrelevant_flag` |
| `slf_clear_stays_clear` | Read `vector_mask.psd` (vector-masked fill layers with bit 4 clear), write and reread as both PSD and PSB. | Every layer keeps the flag clear after both round trips. | `pictura-codec::codec_preserves_a_clear_flag_on_vector_masked_fill_layers` |
| `slf_oracle_shape` | Write the `vector_fill.psd` document to a temp file and open it with psd-tools. | `Shape` reports kind `shape`, `Base` reports kind `pixel`, and the shape record's raw flags are exactly `0x18` (bits 3 and 4). | `pictura-codec::psd_tools_keeps_a_round_tripped_shape_layer_a_shape` |
| `slf_oracle_fill` | Write the `vector_mask.psd` document to a temp file and open it with psd-tools. | Its vector-masked solid-color layers report the exact kind `solidcolorfill`, not `shape`. | `pictura-codec::psd_tools_keeps_a_round_tripped_fill_layer_a_fill_layer` |
| `slf_default_bytes` | Build the default document and write it; reread `default_before.psd` and write it. | Bytes equal the existing goldens, proving bits 3 and 4 are omitted at their default. | `pictura-codec::default_document_matches_rle_golden`, `pictura-codec::existing_files_stay_byte_identical` |
| `slf_authored` | Draw a rectangle through `pictura_render::add_shape_layer`, write the document, and read it with psd-tools. | The layer reports kind `shape` and its raw record flags are exactly `0x18`. | `pictura-codec::psd_tools_reads_an_authored_shape_layer` |
| `slf_no_gain` | Create an ordinary `add_solid_fill` layer and a pixel layer carrying a vector mask with no fill adjustment; write and read both back. | Both layers keep the pixel-data-irrelevant flag clear. | `pictura-codec::codec_does_not_add_the_flag_to_ordinary_fill_or_vector_masked_pixel_layers` |
| `lm_rasterize_shape_flag` | Read `vector_fill.psd`, rasterize a copy with `rasterize_shape`, and save it; also assert the in-memory field. | The field is false, the saved record has bit 4 clear, and psd-tools reports a non-shape kind. | `pictura-render::rasterize_shape_clears_the_pixel_data_irrelevant_flag`, `pictura-codec::psd_tools_reports_a_rasterized_shape_as_pixels` |
| `lm_rasterize_fill_flag` | Create a flagged authored shape with `add_shape_layer`, rasterize a copy with `rasterize_fill_content`, and save it; also assert the in-memory field. | The field is false and the saved record has bit 4 clear. | `pictura-render::rasterize_fill_content_clears_the_pixel_data_irrelevant_flag`, `pictura-codec::psd_tools_reports_a_rasterized_shape_as_pixels` |

## Execution during apply

- Put the new codec checks in `crates/pictura-codec/tests/layer_record_flags.rs`
  and the two render checks in `crates/pictura-render/src/tests/rasterize.rs`.
- Give the authored-shape oracle check its own script constant in
  `vector_mask_oracle.rs`; keep the shared `SCRIPT` and the expected rows of
  `psd_tools_reads_vector_mask_fixture` unchanged.
- For the rasterized save/oracle check, use `vector_fill.psd` for the Rasterize
  Shape leg and an authored `add_shape_layer` document for the Rasterize Fill
  Content leg; the imported `vscg` shape has no `adjustment`, which
  `rasterize_fill_content` requires.
- Reuse the committed `vector_fill.psd` and `vector_mask.psd` fixtures; do not
  add a fixture or change `scripts/generate-fixtures.py`.
- Finish all source and test edits before building; follow the repository's
  compile-last rule and 80% CPU limit.
- After the build, run the named tests and the repository gates, then archive
  the change in the implementation PR.
