# Proposal

## Why

`pictura-render` compiles the whole wgpu/Vulkan stack unconditionally. Its
manifest depends on `wgpu` and `pollster` with no feature gate, `src/lib.rs`
declares `pub mod gpu;` and re-exports the GPU surface at the crate root, and
`src/gpu_filter/` adds a second wgpu module on top.

The CPU renderer does not need any of it. `src/document_ops` (97 files) names no
wgpu symbol, the document composite cache and the CPU compositor are wgpu-free,
and `src/locks.rs` still carries a comment explaining that another crate cannot
depend on `pictura-render` *because* the crate pulls in wgpu. Every CPU-only
build of the crate, every one of its test binaries, and the `pictura-codec`
dev-dependency pay for a package closure of 103 package entries (101 names) in
`Cargo.lock`, and for a normal dependency graph of 138 packages where the
renderer's own code is the last thing to compile.

## What Changes

- Add an optional `gpu` feature to `crates/pictura-render/Cargo.toml`, off by
  default. `wgpu` (both the Linux and the Windows entry) and `pollster` become
  optional and are enabled by `gpu`; `software-gpu` becomes `["gpu"]`, so the
  existing CI spelling keeps working.
- Gate `pub mod gpu;`, the `pub use gpu::{...}` surface, `mod gpu_filter;` and
  `pub use gpu_filter::{apply_filter_active, filter_gpu_available}` behind
  `#[cfg(feature = "gpu")]`.
- Move the wgpu-free `Backend` enum into a small unconditional module so the CPU
  composite API (`CompositeRequest`, `composite`, `composite_with_blend_time`,
  `DocumentComposite::Refreshed`) keeps compiling and returning the same type
  with the feature off. `Backend` stays reachable at `pictura_render::Backend`
  always and at `pictura_render::gpu::Backend` when the feature is on.
- Make the modules that referenced GPU internals feature-aware without changing
  their public signatures: `composite_request` and `document_composite` (a
  GPU-enabled request composites on the CPU when the feature is off), `filter`
  (the GPU filter dispatch falls back to `pictura_filters::apply`), and `metrics`
  (the decline counters are GPU-only; the shape of `counters()` is unchanged).
- Enable `gpu` explicitly from `pictura-app`'s renderer dependency, so the
  application and its GPU paths build exactly as before, and from the GPU test
  binaries through the `software-gpu` spelling that the `gpu` nextest profile
  and the `gpu-parity` CI job already pass.
- Replace the `locks.rs` wgpu caveat with a feature-agnostic sentence.

No behavioural change for any build that enables `gpu` (the application, the GPU
CI job, the GPU test binaries). A CPU-only build compiles the same public
compositing API and runs the CPU oracle.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `meta/build-performance`: add a requirement that the renderer's GPU stack is an
  optional feature. A default-feature build of `pictura-render` SHALL compile and
  test without `wgpu`; enabling `gpu` SHALL restore the full GPU surface
  unchanged; `pictura-app` and the GPU CI job SHALL enable it; `software-gpu`
  SHALL imply `gpu`.

## Impact

- `crates/pictura-render/Cargo.toml`: optional `wgpu`/`pollster`, the `gpu`
  feature, `software-gpu = ["gpu"]`.
- `crates/pictura-render/src/`: new `backend.rs`; cfg gates in `lib.rs`, cfg-aware
  `composite_request.rs`, `document_composite.rs`, `filter.rs` and `metrics.rs`;
  test modules gated or made feature-aware.
- `crates/pictura-render/tests/`: the GPU-only integration binaries gain a
  crate-level `#![cfg(feature = "gpu")]`; the two device-free backend tests are
  gated the same way and added to the `gpu` nextest profile, so they still run in
  the `gpu-parity` job.
- `crates/pictura-app/Cargo.toml`: `features = ["gpu"]` on the renderer
  dependency. The dev-dependency keeps `test-oracle` alone, because the
  `view_oracle` guard pins that line; Cargo unifies the two for test builds.
- `.config/nextest.toml`: record that `software-gpu` implies `gpu`, and select the
  two device-free backend test binaries.
- `.github/workflows/ci.yml`: the Windows renderer step splits its library run
  from the two integration binaries, so the library tests keep running without
  the feature while the binaries are compiled with `gpu` alone (not
  `software-gpu`, which would accept WARP's CPU device).
- No new dependency. No `.psd`, docs or user-visible change.
