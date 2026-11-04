# Proposal

## Why

`pictura-render` encodes the `(document, region, backend)` composite as eight public entry points. Callers must know which one to call, and each repeats rectangle clamping and the GPU-to-CPU fallback, so backend policy leaks into `pictura-app`. The duplication makes the composite seam harder to test and to change.

## What Changes

- Add `CompositeRequest { doc, region: Option<PsdRect>, gpu_enabled: bool }` and `composite(request) -> (PixelBuffer, Backend)`. The request clamps the region, composites the whole document when `region` is `None`, tries the GPU when `gpu_enabled` and `gpu_available()`, and falls back to the CPU oracle inside.
- Keep `composite_rgba` public as the CPU oracle. Keep `refresh_native_composite` and `RenderSession::preview` as they are.
- **BREAKING** Demote `composite_rgba_region` and `composite_native` to `pub(crate)`, and make `composite_gpu` test-only (`#[cfg(test)] pub(crate)`).
- **BREAKING** Remove `composite_active`, `composite_region_active`, and `composite_gpu_or_cpu`.
- Rewire every application call site to `composite(CompositeRequest::whole(...))` or `composite(CompositeRequest::region(...))`.
- Rewire the render and application tests. GPU parity tests use the request; assertions on specific `GpuError` variants move to `gpu::oracle_tests`, and the request matrix lives in `gpu::request_tests`.
- Behavior is unchanged: CPU bytes are identical to `composite_rgba`, GPU bytes stay within ±1 LSB, a region equals the corresponding slice of the whole composite, and empty or out-of-bounds regions clamp without panicking.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/gpu-compositing`: region compositing, fallback, the public API, oracle validation, and high-depth fallback now go through `composite(CompositeRequest)`; the superseded entry points are internal or removed.
- `compositing/gpu-compute-backend`: the default backend entry point is `composite`, not `composite_active`.
- `compositing/native-depth-composite`: `composite_native` becomes an internal helper; `refresh_native_composite` remains the exposed application-facing operation.
- `compositing/native-depth-save`: the high-depth save refresh names `refresh_native_composite` rather than the now-internal `composite_native`.
- `verification/gpu-parity-ci`: fixture scenes composite through the request.
- `verification/performance-metrics`: decline attribution names the GPU path rather than the internal `composite_gpu`.

## Impact

- `crates/pictura-render/src/{composite,composite_region,composite_native,gpu/mod,render_session,lib}.rs`, `document_ops/crop.rs`, `gpu/{eligibility_tests,oracle_tests,request_tests}.rs`, and `src/tests/**`
- `crates/pictura-app/src/cxxqt_object/**`: about 12 production call sites plus the render-crate `document_ops::translate_layer_active` caller, and about 40 test call sites
- `crates/pictura-render/tests/**`: about 50 call sites, including `gpu_parity`, `backend_cpu`, `oracle`, `source_cache_profile.rs`, `gpu_text_gamma_bench.rs`, `per_layer_bench.rs`, `resident_upload_profile.rs`, and `selection_move_perf.rs`
- Public API of `pictura-render`; no new dependencies
- The specs listed above. The `docs/GLOSSARY.md` entries for the removed and demoted functions are updated in task 5.1 in a separate commit carrying `TASK-ALLOWS-DOCS`.
