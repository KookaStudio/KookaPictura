# Tasks

Tests are written before their corresponding production edits. Compilation is
deferred until all code is finished, following the compile-last rule.

## 1. Layer model and codec round trip

- [x] 1.1 Add `crates/pictura-codec/tests/layer_record_flags.rs` with `codec_reads_and_round_trips_the_pixel_data_irrelevant_flag` (read `vector_fill.psd`; assert the read flags; make the PSD round-trip equal the input; make the PSB round-trip equal the input after setting the expected document's `is_psb` to `true`), `codec_preserves_a_clear_flag_on_vector_masked_fill_layers` (round-trip `vector_mask.psd` as both PSD and PSB), and `codec_does_not_add_the_flag_to_ordinary_fill_or_vector_masked_pixel_layers` (create an ordinary `add_solid_fill` layer and a pixel layer with a vector mask and no fill adjustment, write and read them back); verify the assertions cover `slf_read`, `slf_round_trip`, `slf_clear_stays_clear`, and `slf_no_gain` before changing production behavior.
- [x] 1.2 Add `pixel_data_irrelevant` to `Layer` with a doc comment and `false` default, and update every fully spelled `Layer { ... }` literal with the field or `..Default::default()`; verify after the final build that the tests from 1.1 and the existing golden byte tests compile and pass.
- [x] 1.3 Read `flags & 0x10` into the field in `read.rs` and, in `write_layers.rs`, OR `0x08 | 0x10` into `record_flags` when the field is set while leaving bit 3 clear otherwise; verify after the final build that the 1.1 tests pass and `default_document_matches_rle_golden` plus `existing_files_stay_byte_identical` still produce identical bytes.

## 2. Authored shapes and independent decoder

- [x] 2.1 Add `psd_tools_keeps_a_round_tripped_shape_layer_a_shape` (assert `Shape` kind `shape`, `Base` kind `pixel`, and raw record flags exactly `0x18`) and `psd_tools_keeps_a_round_tripped_fill_layer_a_fill_layer` (assert the exact `solidcolorfill` kind) to `layer_record_flags.rs` with the existing dependency-availability self-skip. In `vector_mask_oracle.rs`, add a separate script constant for `psd_tools_reads_an_authored_shape_layer` that reports the layer kind and raw record flags; leave the shared `SCRIPT` and the expected rows of `psd_tools_reads_vector_mask_fixture` unchanged. Verify the expected kinds and raw flag values match `slf_oracle_shape`, `slf_oracle_fill`, and `slf_authored` before changing the renderer.
- [x] 2.2 Set the new field in `add_shape_layer` when it writes the shape's paths; verify after the final build that the authored-shape oracle reports `shape` with raw flags `0x18` and that `add_solid_fill` and a vector-masked pixel layer still write a clear flag (`slf_no_gain`).

## 3. Rasterization lifecycle

- [x] 3.1 Add `rasterize_shape_clears_the_pixel_data_irrelevant_flag` and `rasterize_fill_content_clears_the_pixel_data_irrelevant_flag` to `crates/pictura-render/src/tests/rasterize.rs` (create a flagged authored shape with `add_shape_layer`, assert the field is true, rasterize, assert it is false). Add `psd_tools_reports_a_rasterized_shape_as_pixels` to `layer_record_flags.rs`: read `vector_fill.psd` and rasterize its `Shape` with `rasterize_shape`; create an authored shape with `add_shape_layer` and rasterize it with `rasterize_fill_content` (the imported `vscg` shape has no `adjustment`, which that function requires); save both and assert the field is false and the saved record has bit 4 clear, self-skipping only the psd-tools non-shape assertion when psd-tools is unavailable. Verify the assertions cover `lm_rasterize_shape_flag` and `lm_rasterize_fill_flag` before changing the renderer.
- [x] 3.2 After baking, set `pixel_data_irrelevant = false` in `rasterize_shape` and `rasterize_fill_content`; leave `drop_type` and `rasterize_smart_object` unchanged as out of scope. Verify after the final build that the 3.1 tests pass and the saved records have bit 4 clear.

## 4. Integrated verification and delivery

- [x] 4.1 After all source and test edits are complete, format and perform the production build using at most 80% of available CPUs; verify the build succeeds and rebuild only when a compiler fix changes code.
- [x] 4.2 Run `cargo test -p pictura-codec --test layer_record_flags`, the render rasterize tests, the codec golden tests, and the named oracle checks; then run the applicable repository verification gates and confirm the new checks execute rather than silently skip.
- [x] 4.3 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify every scenario row has real coverage and all artifacts validate. Archive `preserve-shape-layer-flag` in the implementation PR and verify the merged main specs contain the new requirements and no open change remains.
