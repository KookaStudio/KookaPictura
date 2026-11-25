# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ctdb_codec_graph` | Read the codec manifest dev-dependency table and resolve the development graph. | No render or wgpu package appears in the codec development graph. | `pictura-render::renderer_manifest_gates_wgpu_behind_the_gpu_feature`; verified with `cargo tree -p pictura-codec --edges dev` (0 `pictura-render`, 0 `wgpu`). |
| `ctdb_output_ownership` | Run the moved render content round-trip tests and inspect their locations. | Render owns compositor-output assertions; codec round-trip assertions still execute. | `pictura-render::source_free_adjustment_constructors_preserve_supported_parameters`, `pictura-render::source_free_text_constructor_preserves_its_model`, `pictura-render::opaque_text_preserves_identity_and_bytes`, `pictura-render::admitted_content_preserves_parameters_and_bytes` |
| `ctdb_mutation_ownership` | Run the moved render layer and native-depth tests, then the codec mask fixture tests. | Render mutations are tested in render; codec-only PSD/PSB fixtures use codec/core APIs. | `pictura-render::psd_tools_reads_an_authored_shape_layer`, `pictura-render::psd_tools_reads_an_authored_live_shape`, `pictura-render::psd_tools_reads_a_shape_stroke_and_no_fill`, `pictura-render::an_authored_vector_mask_survives_save_and_reopen`, `pictura-render::codec_does_not_add_the_flag_to_ordinary_fill_or_vector_masked_pixel_layers`, `pictura-render::psd_tools_reports_a_rasterized_shape_as_pixels`, `pictura-render::depth32_converted_to_16_writes_a_16bit_file`, `pictura-render::convert_flatten_leaves_one_background`, `pictura-codec::density_and_feather_round_trip`, `pictura-codec::mask_link_toggles_survive_psd_and_psb`, `pictura-codec::raster_save_heap_is_bounded` |
| `ctdb_core_ownership` | Inspect the final diff under `crates/pictura-core`. | No document mutation helper is added or moved. | manual: `git diff origin/master -- crates/pictura-core` is empty. |
| `ctdb_coverage_preserved` | Compare the original and final test-function inventories and inspect each moved body. | Twelve tests move, none are added or deleted, and assertions plus ignore reasons stay unchanged. | manual: `#[test]` inventory moves 12 tests from codec (1061 to 1049) to render (1559 to 1571), combined unchanged; all 12 pass under `cargo nextest run -p pictura-render`. |

## Source review record

The seven affected codec files drop from 102 to 90 direct `#[test]` functions. The two new render suites add ten tests, `document_oracle.rs` adds one, and `native_depth_roundtrip.rs` adds one. The combined total across these files is unchanged at 112. The twelve moved tests retain their assertions; none had an `#[ignore]` attribute. The existing feature-gate manifest test gains an assertion but no test function.

## Verification record

- `cargo fmt --all` — clean.
- `cmake -S . -B build -G Ninja && cmake --build build --parallel` — full build passes (1585 targets).
- `cargo tree -p pictura-codec --edges dev` — `lz4_flex` and `pictura-testkit` only; no `pictura-render`, no `wgpu`.
- `cargo nextest run -p pictura-codec` — 1456 passed, 4 skipped (profiling/bench, `#[ignore]`).
- `cargo nextest run -p pictura-render` — 1926 passed, 16 skipped; the moved suites (`codec_content_roundtrip` 4, `codec_layer_oracles` 6, `document_oracle` 16, `native_depth_roundtrip` 7) all pass.
- `cargo clippy -p pictura-codec -p pictura-render --all-targets -- -D warnings` — clean.
- Test inventory: codec 1061 → 1049 direct `#[test]` functions, render 1559 → 1571; 12 moved, none added or deleted.
- `bash scripts/with-build-lock.sh -- env LC_ALL=C bash scripts/verify-fast.sh` — fmt, clippy, guard, OpenSpec, and 11,921 Qt cases pass apart from one failure, `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`. That failure and `check-milestone-names.py --self-test` reproduce on the clean `origin/master` worktree at `b9ff5febd`; the test source is byte-identical to `origin/master` and the branch does not touch it. `OffthreadJobsTest::largeWholeLayerPreviewKeepsTheEventLoopLive` failed once under load and passed 3/3 standalone reruns.
- The workspace clippy first failed inside the cxx-qt build script on dangling cross-worktree symlinks in the shared `.target`; after removing the poisoned `pictura_app`/`cxx-qt` build dirs, `cargo clippy --workspace --all-targets -- -D warnings` is clean.
