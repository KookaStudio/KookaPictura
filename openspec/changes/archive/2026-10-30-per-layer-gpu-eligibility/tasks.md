# Tasks

## 1. Per-layer plan

- [x] 1.1 Write the plan tests in `crates/pictura-render/src/gpu/eligibility_tests.rs`: `plan_is_device_independent`, `plan_and_dispatch_agree`, `hidden_layer_is_not_planned`, `hidden_clipping_base_suppresses_run`, and `advanced_blending_still_declines`; start `plan_and_dispatch_agree` with the routes that exist and extend it as later groups land.
- [x] 1.2 Record the pre-change CPU bytes for one document per content route and write `cpu_oracle_unchanged_for_every_route` before the dispatch rewrite; verify it passes on the unchanged code.
- [x] 1.3 Add `crates/pictura-render/src/gpu/plan.rs` with the `LayerPlan { effects, body }` tree (`Pixels`, `Raster`, `Fill`, `Adjustment`, `Group`, `ClipGroup`, `Decline`) and the stack walk that carries clipping-run state; keep it under the 800-line target.
- [x] 1.4 Replace the duplicated route logic in `Gpu::composite_layer` (`gpu/backend.rs`) with a match on the plan, thread the plan tree into `composite_gpu_region_raw` and `composite_layer`, and verify `plan_and_dispatch_agree` and the existing `adjustment_layers_match_cpu` scene pass on lavapipe.
- [x] 1.5 Update the eligibility tests for the widened accepted set: `channel_less_type_layer_is_accepted`, `unrenderable_type_source_declines`, `unrenderable_smart_source_declines`, `decoded_vector_fill_is_accepted`, `effects_and_clipping_are_accepted`, and `every_declining_route_names_its_reason`; verify `cargo nextest run -p pictura-render --lib`.
- [x] 1.6 Update the existing decline assertions that the widened set makes false: the proxy-less type case in `crates/pictura-render/src/tests/composite.rs` and the smart-filter declines in `gpu/eligibility_tests.rs`; verify `cargo nextest run -p pictura-render --lib`.

## 2. Lookup-table adjustments

- [x] 2.1 Write `adjustment_lut_matches_apply` and `gradient_map_lut_carries_alpha` in `crates/pictura-adjust`; verify `cargo nextest run -p pictura-adjust` fails until the builder exists.
- [x] 2.2 Add `adjustment_lut(&Adjustment)` (per-channel tables for Levels/Curves/Exposure, a luma colour+alpha table for Gradient Map with the CPU's Rec.601 index and rounding) built from the existing `apply` path; verify the two tests from 2.1 pass.
- [x] 2.3 Write the GPU scene `adjustment_luts_match_cpu` in `crates/pictura-render/tests/gpu_parity/layers.rs` covering Levels, Curves, Exposure, Gradient Map, and a masked reduced-opacity layer.
- [x] 2.4 Add `crates/pictura-render/src/gpu/lut.rs`, the 256-word LUT binding, and the shader adjustment kinds; verify `adjustment_luts_match_cpu` on lavapipe with `cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all`.
- [x] 2.5 Update `unsupported_adjustment_falls_back_or_errors` to use a still-unsupported kind (Selective Color or Color Lookup) instead of Levels; verify the scene still asserts `UnsupportedAdjustment` and the CPU fallback.

## 3. Cached raster sources and fills

- [x] 3.1 Write the composited-source tests in `crates/pictura-render/src/layer_source/tests.rs`: `composited_smart_filter_mask_mix_is_cached`, `composited_smart_source_is_cached`, `composited_type_raster_is_cached`, `composited_source_invalidates_on_filter_mask`, and `composited_source_invalidates_on_payload_or_text`.
- [x] 3.2 Add `layer_source::composited_source` and its content key (type raster, filtered chain mixed with the unfiltered base through the filter mask, placed smart source); include the filter-mask rect, enabled flag, extend-with-white flag, and bytes in the key; verify the tests from 3.1 pass.
- [x] 3.3 Write the GPU scenes `raster_sources_match_cpu`, `fill_layers_match_cpu`, `cached_raster_source_is_reused`, `cached_raster_source_invalidates`, and `raster_key_absent_for_region_build`, plus `pattern_fill_is_not_served_stale` for the pattern key; verify they compile.
- [x] 3.4 Add `UploadKey::Raster { content, rect, doc, gray }` and `crates/pictura-render/src/gpu/raster.rs` to upload a cached raster or a rasterized Solid/Gradient/Pattern/`vscg` fill with a rect layout; verify the scenes from 3.3 on lavapipe.
- [x] 3.5 Make a pattern fill upload unretained and a sub-region raster upload unretained; verify `pattern_fill_upload_is_not_retained` and `raster_key_absent_for_region_build`.
- [x] 3.6 Update the decoded `vscg` assertion in `crates/pictura-render/src/tests/vector_fill.rs` from `UnsupportedAdjustment` to the fill route, and add the decoded-vector-fill scene to `fill_layers_match_cpu`.

## 4. Effect passes

- [x] 4.1 Write the effect scenes in `crates/pictura-render/tests/gpu_parity/layers.rs`: `effects_match_cpu` (one scene per family), `effect_draw_order_matches_cpu`, `disabled_effects_stay_eligible`, and `new_routes_region_matches_full_slice`.
- [x] 4.2 Export `EffectColor` unconditionally from `crates/pictura-render/src/layer_effects/`, then add `crates/pictura-render/src/gpu/effect.rs` to upload a cached `EffectPass` as a dispatch: the pass colour and alpha packed into the existing 8-bit RGBA word, the pass blend mode, constant coverage 255, `fill = 1`, and `text = false`.
- [x] 4.3 Wire below-content passes before the layer dispatch and above-content passes after it, with `opacity = layer.opacity / 255`; verify the scenes from 4.1 on lavapipe.
- [x] 4.4 Verify a disabled, undecodable, or decoded-but-not-rendered effect adds no pass and does not decline; run `disabled_effects_stay_eligible`.
- [x] 4.5 Update the existing layer-effect decline tests in `crates/pictura-render/src/tests/layer_effects*` to assert GPU acceptance and parity, preserving each test's disabled/undecodable/no-op cases; verify `cargo nextest run -p pictura-render --lib`.

## 5. Clipping groups

- [x] 5.1 Write the clipping scenes in `crates/pictura-render/tests/gpu_parity/layers.rs`: `clipping_matches_cpu`, `clipping_hidden_base_matches_cpu`, `clipping_no_base_matches_cpu`, `clipping_multiple_layers_match_cpu`, and `clipping_base_fill_and_effects_match_cpu`.
- [x] 5.2 Add a pure-refactor `build_sibling_group` seam in `crates/pictura-render/src/composite_clipping.rs` that returns the raw group canvas; make `composite_siblings_range` call it; verify the existing clipping oracle tests are byte-identical.
- [x] 5.3 Add `crates/pictura-render/src/gpu/clip.rs` to build the clipped group over the base/clipped bounds with `build_sibling_group`, upload it as a packed source, and dispatch it with the base's opacity and blend mode, `fill = 1`, and no layer mask or vector mask.
- [x] 5.4 Verify all five clipping scenes and the unit test on lavapipe, including the hidden-base and no-base cases.

## 6. Metrics and documentation

- [x] 6.1 Write `layer_decline_is_counted` in `crates/pictura-render/src/metrics.rs`; verify it fails until the counter exists.
- [x] 6.2 Add the per-layer decline counter, the additive `Counters::layer_declines` field, and the app metrics JSON `layer_declines` object next to `declines`; verify `cargo nextest run -p pictura-render -p pictura-app`.
- [x] 6.3 Update the GPU eligibility bullet in `docs/dev/STATE.md` and the perf notes it references; verify `bash scripts/guard.sh` (the docs commit carries `TASK-ALLOWS-DOCS`).
- [x] 6.4 Re-run `python3 scripts/check-test-plan.py` and update any renamed test reference in `test-plan.md`.

## 7. Integration and archive

- [ ] 7.1 Run `bash scripts/verify-fast.sh` and fix every failure.
- [ ] 7.2 Run the GPU parity profile on lavapipe (`cargo nextest run -p pictura-render --features software-vulkan --profile gpu --run-ignored all`) and confirm every scene passes; a missing device must fail the run, not skip it.
- [x] 7.3 Run the full and region benchmarks on a fixture with an effect, a clipping mask, and Curves (`PICTURA_GPU_BENCH=1`); record the before/after numbers in `benchmark.md` in the change folder.
- [x] 7.4 Run `openspec validate --all --strict`, then `openspec archive per-layer-gpu-eligibility` and confirm `scripts/check-openspec-archived.sh` passes.
