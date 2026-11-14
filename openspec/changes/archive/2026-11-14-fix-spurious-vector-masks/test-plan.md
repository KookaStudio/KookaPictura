# Test plan

Every spec scenario is automated. The psd-tools checks self-skip with a clear
message when `python3` plus `psd-tools` is unavailable, following the existing
oracle convention; CI's oracle job runs them.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The psd-tools rectangle fixture decodes to a closed subpath | Read `vector_mask.psd` and inspect the `Shape` layer. | The derived mask has clear invert/disabled flags and one closed subpath spanning the authored rectangle. | `pictura-codec::psd_tools_reads_vector_mask_fixture`, `pictura-codec::rectangle_decodes_to_a_closed_subpath` |
| The invert flag is read from flags bit 0 | Decode a `vmsk` header with flags `0x01`. | The derived mask's `invert` is true. | `pictura-codec::flags_set_invert_and_disable` |
| The disable flag is read from flags bit 2 | Decode a `vmsk` header with flags `0x04`. | The derived mask's `disabled` is true. | `pictura-codec::flags_set_invert_and_disable` |
| The fill-rule field defaults to even-odd | Decode a closed subpath whose fill-rule field is `0` or `1`. | The subpath's fill rule is even-odd. | `pictura-codec::fill_rule_field_zero_is_even_odd` |
| The ag-psd non-zero marker promotes the fill rule | Decode a closed subpath whose fill-rule field is `2`. | The subpath's fill rule is non-zero. | `pictura-codec::fill_rule_marker_two_is_non_zero` |
| A malformed vector mask is unset and never panics | Decode a bad version, unknown selector, knot without subpath, and truncated block. | `read_psd` succeeds, the view is unset, and the raw block stays preserved. | `pictura-codec::malformed_blocks_are_unset` |
| The derived view does not change the serialized block | Read `vector_mask.psd`, write and reread it. | The reconstructed document equals the input, so the `vmsk` bytes are unchanged. | `pictura-codec::codec_preserves_vector_mask_blocks` |
| `vmk_derived_source` | Read the derived-source fixture: pixel layer, record bit 4 clear, `-2` mask with bit 3 set, `vmsk`, no `-3` channel. | The raster mask is present, `vector_mask` is unset, and the raw `vmsk` block remains in `extra_blocks`. | `pictura-codec::a_rendered_source_path_does_not_become_a_vector_mask`, `pictura-codec::codec_suppresses_a_rasterized_source_path` |
| `vmk_derived_shape` | Build the same layer with record bit 4 set. | The derived `vector_mask` is present. | `pictura-codec::a_pixel_data_irrelevant_layer_keeps_its_outline` |
| `vmk_derived_composite` | Build the same layer with a `-3` real-user-mask channel. | The derived `vector_mask` is present. | `pictura-codec::a_real_user_mask_channel_keeps_the_vector_view` |
| `vmk_derived_raster` | Build the same layer with mask flag bit 3 clear. | The derived `vector_mask` is present. | `pictura-codec::an_ordinary_raster_mask_keeps_the_vector_view` |
| `vmk_derived_round_trip` | Read the derived-source fixture, write and reread it. | The layer again has no vector mask, its raster mask is unchanged, and the raw `vmsk` block is preserved. | `pictura-codec::codec_suppresses_a_rasterized_source_path` |
| `vmk_derived_fixture` | Open the derived-source fixture with psd-tools. | The layer carries a `vmsk` block, its mask reports `user_mask_from_render`, and its parameters hold user density 128 plus vector density 200 and feather 4.0. | `pictura-codec::psd_tools_reads_the_derived_raster_mask_fixture` |
| `vmk_derived_authored` | Read the fixture, add a live vector mask, write and reread; also build the post-transform state with mask bit 3 and no `-3`. | The live vector mask survives and the saved raster mask's render flag is clear. | `pictura-codec::an_authored_vector_mask_survives_save_and_reopen`, `pictura-codec::a_live_vector_mask_without_a_real_user_channel_survives_save` |

## Execution during apply

- Add `derived_raster_mask.psd` to `scripts/generate-fixtures.py` and regenerate;
  confirm every existing fixture stays byte-identical.
- Put the predicate unit tests in `crates/pictura-codec/src/vector_mask.rs` and
  the fixture/round-trip checks in
  `crates/pictura-codec/tests/vector_mask_oracle.rs`.
- The psd-tools fixture check asserts only the source-path facts; psd-tools does
  not model Photoshop's display rule, so it is not the behavior oracle.
- The fixture's user density (128) also exercises the raster-mask composite;
  `pictura-render` asserts the density lift outside the mask. The saved record
  normalizes the vector parameters, checked by re-running the psd-tools script
  on a written copy.
- The verified scope is OpenTS's `Triangle`/`Chevron` pixel layers. Layers
  whose record bit 4 is set (fill, adjustment, type, smart object, or shape)
  keep their derived view by design; the issue's Solid Color adjustment layer
  is not in that file and is out of scope.
- Finish all source and test edits before building; follow the repository's
  compile-last rule and the 80% CPU budget.
- After the build, run the named tests and the repository gates, then archive
  the change in the implementation PR.
