# Test plan

Every scenario is automated except the manual acceptance row, which needs the
external reference PSD (too large to commit and not a generated fixture). Test
names are the functions the implementation adds; `scripts/check-test-plan.py`
starts passing once they exist.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vsms_decode` | Build a layer carrying only a `vsms` block with one closed rectangle and read it through `resolve_vector_masks`. | `vector_mask` is set with the authored flags and a closed subpath spanning the rectangle in document pixels. | `pictura-codec::a_vsms_block_decodes_when_vmsk_is_absent` |
| `vsms_vmsk_precedence` | Give one layer a `vmsk` and a `vsms` block with different rectangles and resolve it. | The derived `vector_mask` matches the `vmsk` rectangle. | `pictura-codec::vmsk_wins_when_both_blocks_exist` |
| `vsms_malformed` | Resolve a layer whose only mask block is a `vsms` with version 2, an unknown selector, and a truncated record. | No error or panic, `vector_mask` unset, raw block preserved. | `pictura-codec::a_malformed_vsms_block_is_unset_and_preserved` |
| `vsms_precedence_malformed` | Give one layer a valid `vmsk` and a malformed `vsms` and resolve it. | The view comes from `vmsk`; both raw blocks are preserved. | `pictura-codec::a_valid_vmsk_wins_over_a_malformed_vsms` |
| `vsms_no_fallback` | Give one layer a malformed `vmsk` and a valid `vsms` and resolve it. | The view is unset; no fallback to `vsms`; both raw blocks are preserved. | `pictura-codec::a_malformed_vmsk_does_not_fall_back_to_vsms` |
| `vsms_roundtrip` | Write and re-read a document whose layer carries a `vsms` block. | The block bytes are preserved and the reconstructed document equals the input. | `pictura-codec::a_vsms_block_round_trips_verbatim` |
| `vsms_fixture_pictura` | Read the committed `vector_mask_vsms.psd` with `pictura-codec` and write it back. | The shape layer has `vector_mask` with one closed subpath covering the authored rectangle; the `vsms` block is byte-identical after the round trip. | `pictura-codec::vsms_fixture_decodes`, `pictura-codec::vsms_fixture_round_trips` |
| `vsms_fixture_psd_tools` | Read the same fixture with psd-tools and compare its vector-mask geometry with the Rust read. | psd-tools reports the `vsms` block as the layer vector mask with the same closed path; the test self-skips without psd-tools. | `pictura-codec::vsms_fixture_matches_psd_tools` |
| `vsms_fixture_roundtrip` | Read and write `vector_mask_vsms.psd` with the codec. | The reconstructed document equals the input and the `vsms` block is byte-identical. | `pictura-codec::vsms_fixture_round_trips` |
| `lc_vsms_shape_mask` | Composite the fixture (`vscg` shape with a `vsms` rectangle over an opaque base) on the CPU. | Inside the rectangle carries the fill; outside carries the base; the layer rect is not painted. | `pictura-render::a_vsms_shape_fill_renders_inside_its_path` |
| `lc_vsms_inverted` | Composite the same shape with the `vsms` invert flag set. | Inside the path carries the base and outside carries the fill. | `pictura-render::an_inverted_vsms_mask_hides_its_inside` |
| `clip_vsms_base` | Append a full-canvas layer clipped to the `vsms` shape base in memory and composite. | Clipped colour appears only inside the rectangle; outside shows the backdrop. | `pictura-render::a_clipped_layer_follows_a_vsms_shape_base` |
| `clip_vmsk_base` | Build the same group with the mask under `vmsk` and composite it. | The composite equals the `vsms` case byte for byte. | `pictura-render::a_clipped_layer_follows_a_vmsk_shape_base` |
| `clip_vsms_disabled` | Composite the group with the base `vsms` disable flag set. | The clipped layer is confined to the fill base's rendered alpha without the mask. | `pictura-render::a_disabled_vsms_mask_leaves_the_base_alpha` |
| `vmk_vsms_flags` | Build a 4x4 layer carrying a `vsms` block, then unlink, disable, and re-enable that mask. | Each change patches only the requested bit in the `vsms` flags word; path bytes and other bits are unchanged; reads reflect the state. | `pictura-render::a_vsms_mask_flags_and_link_edit` |
| `vmk_vsms_delete` | Build a 4x4 layer carrying a `vsms` block, delete its vector mask, write, and re-read. | No `vmsk`/`vsms` block remains, the composite is unmasked, and the mask does not return on reload. | `pictura-render::a_vsms_mask_deletes_and_round_trips` |
| `vmk_vsms_rasterize` | Build a 4x4 layer carrying a `vsms` block, rasterize its vector mask, write, and re-read. | No `vmsk`/`vsms` block remains, one linked/enabled layer mask carries the coverage, and the composite is unchanged. | `pictura-render::a_vsms_mask_rasterizes_without_a_reload_change` |
| `vmk_vsms_reads` | Build a 4x4 layer carrying a `vsms` block and query presence, link, and disabled. | Presence present, link linked, disabled enabled, matching the derived view. | `pictura-render::vsms_read_predicates_track_the_effective_block` |
| `lm_vsms_shape` | Read the fixture, inspect the shape layer, then run Rasterize Shape, write, and re-read. | The layer reports as a shape with its outline from `vsms`; rasterization succeeds, drops both vector-mask keys, and keeps the composite unchanged. | `pictura-render::a_vsms_shape_layer_is_recognized_and_rasterizes` |
| Reference-file acceptance | Open the reported reference document in a Pictura build and inspect the affected shape layer. | The shape layer renders its letterforms, not a filled rectangle, and the clipped layer is confined to those letterforms; the panel shows one shape row. | manual: the client PSD is not a committed CI fixture |

Existing coverage to keep green while implementing: `vector_mask.psd` and
`vector_fill.psd` fixture tests, `malformed_vmsk_is_unset_and_never_panics`, the
`clipping` suite, and the existing `vmsk` delete/enable/link/rasterize tests.
None of their baselines should change.

The manual row belongs in the implementation PR's reviewer checklist without
committing the reference file.
