# Tasks

## 1. Request Seam

- [x] 1.1 Add `crates/pictura-render/src/composite_request.rs` with `CompositeRequest { doc, region, gpu_enabled }`, the `whole`/`region` constructors, clamping, the GPU attempt, and the CPU fallback; declare `mod composite_request;` in `lib.rs`. Verify `cargo check -p pictura-render --lib`.
- [x] 1.2 Make `gpu::composite_gpu_region` `pub(crate)` and route the request through it; keep the whole-document CPU branch on `composite_rgba` and the sub-region branch on `composite_rgba_region`. Verify `cargo check -p pictura-render --lib`.
- [x] 1.3 Add `crates/pictura-render/src/gpu/request_tests.rs` with `request_matrix_matches_the_oracles` (`pictura-render::request_matrix_matches_the_oracles`): composite a fixture with a group, a mask, an adjustment, and a separable blend through whole/region × CPU/GPU; assert CPU bytes equal the matching oracle, GPU bytes stay within ±1 LSB or report `Backend::Cpu`, and a 32-bit region request reports `Backend::Cpu`. Verify with no device and with `--features software-gpu` on a software device.
- [x] 1.4 Re-export `composite` and `CompositeRequest` from `lib.rs`. Verify `cargo check -p pictura-render --lib`.

## 2. Error and Coverage Tests

- [x] 2.1 Add `crates/pictura-render/src/gpu/oracle_tests.rs` and move the `GpuError` assertions from `tests/gpu_parity` into it: `dissolve_still_cpu_only`, `cpu_only_modes_error_without_panicking`, `unsupported_adjustment_falls_back_or_errors`, and `document_past_2d_product_limit_returns_too_large`. The `GpuError::Unavailable` behavior is covered by `gpu::tests::the_gpu_gate_is_consistent_with_the_validation_scene` when no device is available, and `tests/backend_cpu.rs::cpu_pin_composites_on_cpu` keeps the request-level CPU assertion; no separate process-global backend test is added. Verify the device-free cases run in the default nextest job and `document_past_2d_product_limit_returns_too_large` keeps its `#[ignore]` reason.
- [x] 2.2 Add the CPU whole-document rect to `cpu_region_matches_full_slice` (`tests/gpu_parity/region.rs`) so the CPU full-canvas case is covered alongside the GPU `full_region_equals_full_composite`. Verify both region tests compile.

## 3. Rewire Callers

- [x] 3.1 Rewire render production: `crates/pictura-render/src/document_ops/crop.rs` `translate_layer_active` (line 222) to `composite(CompositeRequest::whole(doc, gpu_enabled))`. Verify `cargo check -p pictura-render --lib`.
- [x] 3.2 Rewire `pictura-app` production call sites in `helpers_composite.rs`, `web_export`, `svg_export`, `impl_paint.rs`, `impl_transform/**`, `image_adjust/jobs`, and `filter_tools/**`. Verify `cargo check -p pictura-app`.
- [x] 3.3 Rewire the in-crate render tests that call removed functions: `src/gpu/eligibility_tests.rs`, the `gpu/mod.rs` test module, `src/tests/region.rs`, `src/tests/composite.rs`, `src/tests/layer_effects.rs`, `src/tests/vector_mask.rs`, `src/tests/adjustment/part_composite.rs`, and the `document_ops/crop.rs` test module. Verify `cargo check -p pictura-render --all-targets` (or `cargo nextest run -p pictura-render --no-run`); plain `cargo check -p` does not compile `#[cfg(test)]`.
- [x] 3.4 Rewire integration tests: `tests/gpu_parity/{common,blend,layers,large,region,text,fresh_white}.rs`, `tests/backend_cpu.rs`, `tests/oracle.rs`, `tests/source_cache_profile.rs`, `tests/gpu_text_gamma_bench.rs`, `tests/per_layer_bench.rs`, `tests/resident_upload_profile.rs`, and `tests/selection_move_perf.rs`. The rewritten `fallback_matches_cpu_for_cpu_only_mode` must assert `Backend::Cpu`. Verify the device-free tests pass and the ignored tests compile.
- [x] 3.5 Rewire the `pictura-app` tests and profiling call sites: `tests_impl.rs`, `tests_profiles/**`, `tests_profiles.rs`, `tests.rs`, `canvas_view_test.rs`, `damage_policy_tests.rs`, and `metrics_tests.rs`. Verify `cargo nextest run -p pictura-app` passes.

## 4. Retire the Superseded Entry Points

- [x] 4.1 Demote `composite_rgba_region` and `composite_native` to `pub(crate)`, and make `composite_gpu` test-only (`#[cfg(test)] pub(crate)`); change the `composite.rs` `pub use` for `composite_rgba_region` to `pub(crate) use`; remove all three from the crate-root re-exports. Verify `cargo check --workspace --all-targets` and that no external crate still calls them.
- [x] 4.2 Remove `composite_active`, `composite_region_active`, and `composite_gpu_or_cpu`, plus the now-unused `composite_cpu_region` helper. Verify `cargo check --workspace --all-targets`.

## 5. Documentation

- [x] 5.1 Update the `docs/GLOSSARY.md` entries for the removed and demoted functions in a separate commit carrying `TASK-ALLOWS-DOCS`. Verify `scripts/guard.sh` accepts the commit.
- [x] 5.2 Confirm the delta specs match the final API names. Verify `openspec validate composite-request --strict` and `openspec validate --all --strict` pass.

## 6. Integration Checks

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`. Verify all pass.
- [x] 6.2 Run the GPU profile where a device exists: `cargo nextest run -p pictura-render --features software-gpu --profile gpu --run-ignored all`. Verify the parity and moved error tests pass or report skipped.
- [x] 6.3 Search the tree for `composite_active`, `composite_region_active`, `composite_gpu_or_cpu`, and external `composite_gpu` / `composite_native` / `composite_rgba_region` references. Verify only in-crate definitions, in-crate tests, and archived OpenSpec text remain, and clean stale doc comments such as `src/gpu/mod.rs:12,26,237` and `tests/gpu_parity/main.rs:13`.
- [ ] 6.4 Run `bash scripts/verify-fast.sh`; run `bash scripts/verify-full.sh` (or refresh the CMake build first) when the build tree is stale.
- [x] 6.5 Archive the change: `openspec archive composite-request`, then run `scripts/check-openspec-archived.sh`.
