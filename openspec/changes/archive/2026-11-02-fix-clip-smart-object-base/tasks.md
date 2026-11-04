# Tasks

## 1. Tests

- [x] 1.1 Add a transparent-payload helper and `source_preserves_merged_alpha` in the new module `crates/pictura-render/src/tests/smart_object_alpha.rs`, and add `mod smart_object_alpha;` to `crates/pictura-render/src/tests/mod.rs`; assert the rendered source alpha matches the payload's merged transparency plane; verify with `cargo nextest run -p pictura-render source_preserves_merged_alpha`.
- [x] 1.2 Add `source_preserves_grayscale_merged_alpha` and `source_without_extra_plane_is_opaque` to the same module; verify with `cargo nextest run -p pictura-render merged_alpha`, `cargo nextest run -p pictura-render source_without_extra_plane_is_opaque`, and `cargo nextest run -p pictura-render embedded_source_renders_without_proxy`.
- [x] 1.3 Add `source_uses_first_extra_plane_as_alpha` to the same module for a payload with two extra merged planes, transparency first; verify with `cargo nextest run -p pictura-render source_uses_first_extra_plane_as_alpha`.
- [x] 1.4 Add `a_clipped_layer_respects_a_smart_object_bases_alpha` and `a_clipped_layer_respects_a_proxy_bases_alpha` in `crates/pictura-render/src/tests/clipping.rs`; verify with `cargo nextest run -p pictura-render clipped_layer_respects`.

## 2. Source alpha

- [x] 2.1 In `crates/pictura-render/src/layer_source/mod.rs` `decode_embedded`, append the decoded document's first full-size extra merged plane as alpha when the stored composite has 1 or 3 channels and `channel.data.len() >= plane`; otherwise leave the source opaque. Verify tasks 1.1 and 1.3 pass.
- [x] 2.2 In `crates/pictura-render/src/smart_source.rs` `scale_smart_source`, read alpha from plane 1 when the source has 2 channels; verify task 1.2 passes.
- [x] 2.3 Put a `ponytail:` comment on the merged-alpha heuristic naming the saved-selection ambiguity and the upgrade path (alpha-channel metadata or a background-layer discriminator), per AGENTS.md rule 7.

## 3. Clipping integration

- [x] 3.1 Confirm the proxy-base path is unchanged and task 1.4's proxy case passes with the existing `composite_pixels` coverage.
- [x] 3.2 Add `clipping_smart_source_matches_cpu` in `crates/pictura-render/tests/gpu_parity/layers.rs` for the smart-source clipping group; the CPU test is the oracle. GPU tests call `require_gpu()` and fail without a device per `docs/dev/testing-conventions.md`; run with `cargo nextest run --profile gpu --run-ignored all` on a Vulkan device.

## 4. Verification

- [x] 4.1 Run `cargo nextest run -p pictura-render`, `cargo clippy -p pictura-render --all-targets -- -D warnings`, and `cargo fmt --all --check`; verify all pass.
- [x] 4.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`; verify both pass.
- [x] 4.3 Run `bash scripts/check-file-size.sh`; verify the new module keeps every file under its cap.

## 5. Archive

- [x] 5.1 Archive the change with `openspec archive fix-clip-smart-object-base`, then verify `bash scripts/check-openspec-archived.sh` passes.
