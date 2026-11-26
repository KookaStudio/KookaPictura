# Tasks

Test tasks are written before their implementation task, following the repo's
test-first rule. Their verify command is expected to fail until the named
implementation task lands; it must pass at close-out.

## 1. Shared unit conversion

- [x] 1.1 Add `pictura-render::unit_table_matches_division` in a new
  `crates/pictura-render/src/tests/composite_kernel.rs` (registered in
  `src/tests/mod.rs`), asserting every table entry has the same bits as
  `(i as f32) / 255.0` for both a channel-plane sample and a mask-gate sample;
  verify with `cargo nextest run -p pictura-render unit_table_matches_division`.
- [x] 1.2 Add the 256-entry const unit table and use it in `source_unit`
  (`composite_rows.rs`) and `MaskGate::alpha` (`composite_blend.rs`); leave the
  native `SampleRow` conversions unchanged because the fast set excludes native
  stores; verify 1.1 and `cargo nextest run -p pictura-render` byte-identity
  suites pass.

## 2. Kernel selection, override, and shapes

- [x] 2.1 Add the test-only `FORCE_GENERAL` switch beside `FORCE_SERIAL` and
  per-shape `KERNEL_RUNS` counters, plus reset helpers. Write
  `pictura-render::common_layer_uses_the_row_span_kernel`,
  `pictura-render::advanced_attributes_keep_the_general_kernel`, and
  `pictura-render::kernel_selection_follows_synthetic_overrides` covering the
  alpha row, Blend If, curve, native plane, non-Normal mode, knockout, a
  clipping base override that changes mode/fill/mask, and a knockout cover.
  Hold `serial_test_lock` and reset `FORCE_GENERAL` and `KERNEL_RUNS` with an
  RAII guard. Expected to fail until 2.3 and 2.4; verify with
  `cargo nextest run -p pictura-render common_layer_uses_the_row_span_kernel`,
  `cargo nextest run -p pictura-render advanced_attributes_keep_the_general_kernel`,
  and
  `cargo nextest run -p pictura-render kernel_selection_follows_synthetic_overrides`.
- [x] 2.2 Write `pictura-render::opaque_kernel_matches_the_general_path` over a
  transparent and a partially transparent backdrop, asserting the opaque
  `KERNEL_RUNS` shape counter with a reset guard; hold `serial_test_lock`.
  Expected to fail until 2.4; verify with
  `cargo nextest run -p pictura-render opaque_kernel_matches_the_general_path`.
- [x] 2.3 Select the kernel per layer pass from the effective `BlendCtx`, the
  resolved `SourcePlanes::any_native()`, and the row's cover flag, including
  plain content and smart-object/type layers composited from a raster proxy;
  disable selection under the test-only `legacy_blend` and `legacy_source`
  switches and assert `KERNEL_RUNS` stays zero under each; verify 2.1 passes.
- [x] 2.4 Add the alpha and opaque row loops in `composite_pixels`, preserving
  the general formula's operation order and the zero-alpha colour-read skip;
  verify 2.2, `ctx_kernel_matches_legacy`, and `zero_alpha_skips_mask` pass.

## 3. Per-row mask coverage

- [x] 3.1 Write `pictura-render::kernel_masks_match_the_general_path` covering a
  plain raster mask at density 255 and below, a feathered raster mask, a vector
  mask with invert on and off, a native mask combined with a vector mask, a
  masked grayscale layer, a zero-opacity masked layer, and a zero-fill masked
  layer, asserting the masked `KERNEL_RUNS` shape counter; hold
  `serial_test_lock` and reset the counter and switches with an RAII guard.
  Expected to fail until 3.2; verify with
  `cargo nextest run -p pictura-render kernel_masks_match_the_general_path`.
- [x] 3.2 Add the thread-local `f32` mask-row builder behind the existing gate
  evaluation and the masked kernel shape. For each row, compute
  `content = (src_a * opacity) * fill` per pixel, call the gate only where
  `content > 0.0`, leave skipped entries at `0.0`, and let the kernel's
  `as_ <= 0` test skip them; verify 3.1, `ctx_masks_match_legacy`, and
  `zero_alpha_skips_mask` pass.
- [x] 3.3 Confirm the mask-sample counter still records samples through the
  masked kernel path, and that the zero-alpha, zero-opacity, and zero-fill cases
  record none; verify
  `cargo nextest run -p pictura-render zero_alpha_skips_mask` and the
  zero-content cases in 3.1.

## 4. Cross-path and matrix parity

- [x] 4.1 Write `pictura-render::kernel_matrix_matches_the_general_path`
  covering alpha/no-alpha, every mask kind, grayscale, non-default opacity and
  fill, clipping, knockout, a smart-object layer composited from its raster
  proxy, and a type layer composited from its raster proxy, asserting the
  per-shape `KERNEL_RUNS` counters and `PixelBuffer::data` equality between the
  selected kernel and the forced-general path; hold `serial_test_lock` and use
  reset guards. Verify with
  `cargo nextest run -p pictura-render kernel_matrix_matches_the_general_path`.
- [x] 4.2 Write
  `pictura-render::kernel_switches_compose_with_legacy_and_serial`: assert
  `KERNEL_RUNS` stays zero while `legacy_blend` or `legacy_source` is set, and
  that `FORCE_GENERAL` composes with `FORCE_SERIAL` on the same document; hold
  `serial_test_lock` and reset every switch with an RAII guard. Verify with
  `cargo nextest run -p pictura-render kernel_switches_compose_with_legacy_and_serial`.
- [x] 4.3 Write `pictura-render::kernel_parity_across_banded_region_and_serial`
  on a stack taller than two bands, comparing banded, sub-region, and
  `FORCE_SERIAL` output with the whole-canvas parallel composite; hold
  `serial_test_lock` and reset `FORCE_SERIAL` with an RAII guard. Verify with
  `cargo nextest run -p pictura-render kernel_parity_across_banded_region_and_serial`.
- [ ] 4.4 Run the existing oracle suites and fix any byte mismatch; verify
  `cargo nextest run -p pictura-render` and
  `cargo test -p pictura-render --test document document_oracle::` pass.

## 5. Profile evidence and close-out

- [x] 5.1 Extend `crates/pictura-render/examples/composite_profile.rs` with an
  alpha-less Normal case and a plain-raster-mask Normal case. Both must be
  kernel-eligible: Normal mode, opacity 255, fill 255, no Blend If, no
  text-gamma curve, no native stores, full-canvas 8-bit layer. Add
  `pictura-render::profile_cases_use_the_intended_shapes` in the unit tests
  asserting the opaque and masked `KERNEL_RUNS` for the same layer states,
  because the ignored integration profile cannot read the crate-private
  counters. Verify with
  `cargo run -p pictura-render --release --example composite_profile`
  prints both timings and the unit test passes.
- [ ] 5.2 Record release before/after 4000² timings for the profile cases
  (branch vs `origin/master`, same machine and build profile) in the PR body,
  using
  `cargo run -p pictura-render --release --example composite_profile`;
  verify the table is present and names the command used.
- [x] 5.3 Run `cargo fmt --all`,
  `cargo clippy -p pictura-render --all-targets -- -D warnings`,
  `python3 scripts/check-test-plan.py`,
  `openspec validate row-span-kernels --strict`, and
  `openspec validate --all --strict`; verify all exit 0. The test-plan gate is
  expected to fail only while tests named in the plan do not yet exist; by this
  point every planned test exists.
- [ ] 5.4 Close out: run `openspec archive row-span-kernels`, then
  `bash scripts/verify-fast.sh` (covers file-size, guard, archive, test-plan,
  and openspec) and `openspec validate --all --strict`; verify exit 0 and that
  only `openspec/changes/archive/` remains under `openspec/changes/`.
