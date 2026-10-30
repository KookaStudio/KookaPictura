# Tasks

## 1. Measurement protocol

- [x] 1.1 Add `crates/pictura-render/tests/composite_profile.rs` with an ignored `cpu_composite_profile_4000` test (the GPU profile already uses `composite_profile_4000`) that builds a 4000² document (edge overridable with `PICTURA_PROFILE_SIZE`) and prints the wall time for Normal without a mask, Multiply, a feathered raster mask, a vector mask, and a full-canvas type layer (the sparse case the early-out targets); verify `cargo test -p pictura-render --test composite_profile -- --ignored --nocapture` prints one timing per scenario.
- [x] 1.2 Run the profile before any implementation change and record the five baseline timings in the task result and the PR body; verify the recorded numbers match the printed output.

## 2. Tests (written before the implementation they cover)

Group 2's tests are written against the APIs in groups 4-5; per the repo workflow, intermediate commits need not build, and the tree compiles once tasks 5.1-5.2 land.

- [x] 2.1 Add `ctx_kernel_matches_legacy` in `crates/pictura-render/src/tests/composite_ctx.rs`: every layer blend mode over seeded random source/backdrop samples, including a `fill != 255` case, context kernel versus the test-only legacy switch; compare `Px` channel `f32` `to_bits()` as well as the final `u8`, and verify it passes once group 5 lands.
- [x] 2.2 Add `zero_alpha_skips_mask` in `crates/pictura-render/src/tests/composite_ctx.rs`: a sparse layer over a backdrop plus the test-only mask-sample counter; verify output matches the legacy switch and the counter records no zero-alpha sample.
- [x] 2.3 Add `zero_alpha_does_not_mark_cover`: a knockout layer with a zero-alpha pixel leaves its coverage flag false and the backdrop unchanged; verify it passes once group 3 lands.
- [x] 2.4 Add `ctx_high_depth_matches_legacy`: 16-bit and 32-bit documents with a retained native plane, including a store that omits one channel; verify byte-identity against the test-only legacy source path (`FORCE_LEGACY_SOURCE`) with the legacy blend body.
- [x] 2.5 Add `ctx_masks_match_legacy`: feathered raster, feathered vector, plain vector, and native raster plus vector; verify byte-identity against the legacy switch.
- [x] 2.6 Add `ctx_constants_match_legacy`: opacity, fill, non-default Blend If, and a type-tool curve; verify byte-identity against the legacy switch.
- [x] 2.7 Add `ctx_route_callers_match_legacy`: pixel, isolated group, adjustment, solid/gradient/pattern fill, smart source, smart filter, and type source, each with the legacy switch on and off; verify byte-identity for every route.
- [x] 2.8 Extend the existing corpus in `crates/pictura-render/src/tests/composite_ctx.rs` with a zero-alpha sparse layer and a native-plus-vector mask so the route tests reach both cases; verify each new test passes once its implementation group lands.

## 3. Early-out cascade

- [x] 3.1 In `composite_pixels` (`crates/pictura-render/src/composite_rows.rs`), read the source alpha before the colour planes and skip the pixel when it is zero; verify the existing composite tests pass (`zero_alpha_skips_mask` is verified at 5.2).
- [x] 3.2 Reorder `blend_into_row` to `content = (src_a * opacity) * fill`, return on `content <= 0`, apply the curve, sample the mask, return on `content * mask <= 0`, then evaluate Blend If and blend; verify `zero_alpha_does_not_mark_cover` passes; the legacy comparison is verified at 5.2.
- [x] 3.3 Verify the region and serial equality tests (`a_region_no_effect_reaches_composites_alone_and_exactly`, `short_region_composite_matches_serial`, `parallel_row_equals_sequential_row`) still pass.

## 4. Core sampling accessors

- [x] 4.1 Add `Samples::row(y, width)` and a `SampleRow` enum with the same unit conversion as `sample_unit` in `crates/pictura-core/src/samples.rs`; `row` returns the available slice (`None` only when the row start is out of range) and `unit` bounds-checks per pixel; verify a unit test compares the row values with the per-index conversion, including a fractured banded plane and a truncated plane.
- [x] 4.2 Add `VectorMask::coverage_sampler(clip)` returning an opaque `VectorCoverage<'_>` in `crates/pictura-core/src/vector.rs`, with `coverage()` delegating to it; verify the existing vector-mask tests (`coverage` inside/outside/feather) pass unchanged.
- [x] 4.3 Add `mask_refine::resolve_raster` (eager `shared_cell` + `get_or_init`, returning a `RasterGate<'_>` that owns the `Arc`) and `mask_refine::resolve_vector` (a `VectorGate<'_>` wrapping a core `VectorCoverage<'_>` or a resolved feathered plane); verify `a_feathered_vector_mask_softens_and_inverts`, `painting_the_mask_retires_its_feathered_plane`, and a new sampler-equivalence test pass.

## 5. Blend context and kernel

- [x] 5.1 Add `crates/pictura-render/src/composite_blend.rs` with `BlendCtx`, `MaskGate`, `SourcePlanes`, and the context-based blend kernel, and declare `mod composite_blend;` in `crates/pictura-render/src/lib.rs`; verify the crate compiles with the new module and no callers yet.
- [x] 5.2 Add the `cfg(test)` `FORCE_LEGACY_BLEND` and `FORCE_LEGACY_SOURCE` switches and the `MASK_SAMPLES` counter, serialized with `serial_test_lock()` and reset before the measured composite; verify `ctx_kernel_matches_legacy` and `zero_alpha_skips_mask` pass (`ctx_high_depth_matches_legacy` is verified at 6.1).
- [x] 5.3 Make `blend_into_row_ctx` the shared kernel; `blend_into_ctx` takes a prebuilt context and one sample, while `blend_parts` keeps calling `blend_parts_row` with `alpha * effect_scale`; keep the per-pixel entry points `#[cfg(test)]` for the legacy oracle; verify `ctx_kernel_matches_legacy`, `zero_alpha_skips_mask`, `ctx_masks_match_legacy`, and `ctx_constants_match_legacy` pass (route tests are verified in group 6).

## 6. Migrate content routes

- [x] 6.1 Migrate `composite_pixels` to `SourcePlanes`: resolve the four native planes once per layer and take `.row(ly, lw)` per row, with the per-channel 8-bit fallback; verify `ctx_high_depth_matches_legacy` and the native-depth round-trip suite pass.
- [x] 6.2 Migrate `composite_canvas`, `composite_adjustment`, and `gate_adjusted` to build the context once per layer pass; verify the group, adjustment, and knockout tests pass.
- [x] 6.3 Migrate the fill, smart-source, smart-filter, and type-source rect loops to build the context once; verify `ctx_route_callers_match_legacy` passes.
- [x] 6.4 Add `#[inline]` to `Document::authoritative_plane` and verify the workspace builds with no new clippy warning.

## 7. Conditional specialisation

- [x] 7.1 Re-run `cpu_composite_profile_4000` after groups 3-6 and record the timings; verify the numbers are captured in the task result.
- [x] 7.2 If the profile attributes a dominant share of the no-mask Normal pass to the blend arithmetic, add a monomorphized Normal kernel and per-depth row loops; verify `ctx_kernel_matches_legacy` still passes and re-run the profile.
- [x] 7.3 If the hoist alone meets the target, record the decision to skip specialisation in the task result; verify no unused specialisation code remains.

## 8. Integration verification

- [x] 8.1 Run `cargo nextest run --workspace` and `cargo test --workspace --doc`; verify both pass.
- [x] 8.2 Run the oracle suites with `magick` and `psd-tools` available (`document_oracle`, `clipping_oracle`, `knockout_oracle`, `adjustment_oracle`, `merge_oracle`, `native_depth_roundtrip`); verify they pass.
- [x] 8.3 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-file-size.sh`, and `openspec validate --all --strict`; verify each passes.
- [x] 8.4 Run the profile on master and on the branch with the same build profile and record the before/after table in the PR body; verify the table matches the captured output.

## 9. Archive and final gate

- [x] 9.1 Run `openspec archive composite-hoist-layer-context`; verify `openspec/changes/` holds only `archive/`.
- [x] 9.2 Run `bash scripts/verify-fast.sh` after archiving; verify it passes.

## Measurements

4000² CPU composite, release build, 16 threads, same machine; median of three runs.

| Scenario | origin/master `c384db85` | branch | Speedup |
|---|---:|---:|---:|
| Normal, no mask | 169.3 ms | 126.0 ms | 1.34x |
| Multiply | 167.3 ms | 127.9 ms | 1.31x |
| Feathered raster mask | 192.9 ms | 138.5 ms | 1.39x |
| Vector mask | 189.3 ms | 141.6 ms | 1.34x |
| Full-canvas type layer | 165.9 ms | 125.4 ms | 1.32x |

The hoist plus the zero-alpha cascade gives 1.31-1.39x across the measured paths.
`perf` is not installed on this machine, so the remaining time could not be
attributed to the blend arithmetic. Specialisation (a monomorphised Normal
kernel and per-depth row loops) was therefore not added; the measured gain is
recorded here as the ceiling for the hoist alone, and the specialisation remains
a follow-up if the blend arithmetic is later shown to dominate.
