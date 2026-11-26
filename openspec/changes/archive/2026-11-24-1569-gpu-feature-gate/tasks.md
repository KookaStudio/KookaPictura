# Tasks

## 0. Tests first

- [x] 0.1 Add `crates/pictura-render/src/feature_gate_tests.rs` with `renderer_manifest_gates_wgpu_behind_the_gpu_feature`, `gpu_backend_is_one_type_behind_the_feature`, `gpu_enabled_request_falls_back_to_the_cpu_without_the_feature`, `app_manifest_enables_the_gpu_feature` and `software_gpu_implies_gpu`, declared from `lib.rs` as `#[cfg(test)] mod feature_gate_tests;`. Gate `gpu_backend_is_one_type_behind_the_feature` with `#[cfg(feature = "gpu")]` and `gpu_enabled_request_falls_back_to_the_cpu_without_the_feature` with `#[cfg(not(feature = "gpu"))]`; the manifest assertions read the sibling manifests through `env!("CARGO_MANIFEST_DIR")`. Verify `cargo test -p pictura-render --lib feature_gate` (no feature) and `cargo test -p pictura-render --lib --features gpu feature_gate` each report the tests they can compile (expected to fail until task 1.1).

## 1. Feature and manifest

- [x] 1.1 Add to `crates/pictura-render/Cargo.toml`: `gpu = ["dep:wgpu", "dep:pollster"]` in `[features]`, `software-gpu = ["gpu"]`, and `optional = true` on both `wgpu` entries (the plain one and the `cfg(windows)` one) and on `pollster`; verify `cargo metadata --no-deps` shows the renderer's default feature set empty and `gpu`/`software-gpu` as declared.
- [x] 1.2 Add `features = ["gpu"]` to the `pictura-render` dependency in `crates/pictura-app/Cargo.toml`; leave the dev-dependency at `features = ["test-oracle"]` (the `view_oracle` guard pins that line, and Cargo unifies the two for test builds). Verify `cargo check -p pictura_app --all-targets` passes.

## 2. Backend seam

- [x] 2.1 Create `crates/pictura-render/src/backend.rs` holding the `Backend` enum moved verbatim from `gpu/mod.rs` (doc comment included); declare `mod backend;` and `pub use backend::Backend;` in `lib.rs`, and add `pub use crate::backend::Backend;` to `gpu/mod.rs` so `pictura_render::gpu::Backend` still resolves; remove the original enum from `gpu/mod.rs`.
- [x] 2.2 Point the CPU-side users at the new path: `document_composite.rs`'s `use crate::gpu::Backend;` becomes `use crate::Backend;`, and `composite_request.rs`'s `use crate::gpu::{self, Backend};` becomes `use crate::Backend;` plus `#[cfg(feature = "gpu")] use crate::gpu;`.
- [x] 2.3 Rewrite `crates/pictura-render/src/locks.rs`'s module doc so the pointer to `pictura-core` gives the layer-model reason and makes no wgpu claim.

## 3. Cfg the GPU modules and their users

- [x] 3.1 Gate `pub mod gpu;`, `pub use gpu::{...}`, `mod gpu_filter;` and `pub use gpu_filter::{apply_filter_active, filter_gpu_available}` in `lib.rs` behind `#[cfg(feature = "gpu")]`.
- [x] 3.2 In `composite_request.rs`, wrap the GPU attempt in `composite_with_blend_time` in `#[cfg(feature = "gpu")]` so a GPU-enabled request falls through to the CPU oracle without the feature.
- [x] 3.3 In `filter.rs`, replace the direct `crate::gpu_filter::apply_filter_active` call with a local `apply_kernel_filter(filter, buf, gpu)` that delegates to `gpu_filter::apply_filter_active` under `#[cfg(feature = "gpu")]` and to `pictura_filters::apply` otherwise, and correct the doc comment that names the gated function.
- [x] 3.4 In `metrics.rs`, gate `use crate::GpuError`, `reason_index`, `count_decline` and `count_layer_decline` behind `#[cfg(feature = "gpu")]`; keep `DECLINE_REASONS`, `decline_count`, `Counters` and the rest unconditional; gate the three `GpuError`-typed tests in its `mod tests`.
- [x] 3.5 Gate the `composite_gpu` imports and every test that names them in `crates/pictura-render/src/tests/{composite,text_gamma,vector_fill}.rs` (composite.rs lines 918 and 1013, text_gamma.rs line 88, vector_fill.rs line 205); gate `assert_gpu_accepts_and_matches_cpu` in `src/tests/layer_effects.rs` together with all 32 call sites across `layer_effects.rs` and `src/tests/layer_effects/*.rs`; make the availability probes feature-aware (`cfg!(feature = "gpu") && gpu_available()`) in `src/tests/region.rs` and in `src/tests/composite.rs::proxy_less_type_layer_composites_live_coverage`, keeping those two files' CPU assertions running in both builds.
- [x] 3.6 Give `crates/pictura-render/tests/gpu_parity/main.rs`, `tests/gpu_filter_parity.rs`, `examples/gpu_text_gamma_bench.rs` and `examples/resident_upload_profile.rs` a crate-level `#![cfg(feature = "gpu")]`, and gate `tests/backend_pin.rs` and `tests/backend_cpu.rs` the same way.
- [x] 3.7 In `tests/oracle/main.rs`, gate the two `#[ignore]`d GPU tests and their `Backend` imports behind `#[cfg(feature = "gpu")]`; in `tests/cold_engine_profile.rs` and `examples/selection_move_perf.rs`, make the `gpu_available()` probe feature-aware.

## 4. CI and test profile

- [x] 4.1 Add `binary(backend_pin) | binary(backend_cpu)` to the `[profile.gpu]` `default-filter` in `.config/nextest.toml` and update its comment to state that `software-gpu` implies `gpu`.
- [x] 4.2 In `.github/workflows/ci.yml`, split the Windows renderer test step so the library tests stay feature-less and the two gated integration binaries build with `gpu` alone (not `software-gpu`, which would accept WARP's CPU-type device): `cargo test -p pictura-render --lib` then `cargo test -p pictura-render --test backend_pin --test backend_cpu --features gpu`.

## 5. Verification

- [x] 5.1 `cargo check -p pictura-render` and `cargo check -p pictura-render --features gpu` both pass.
- [x] 5.2 `cargo clippy -p pictura-render --all-targets -- -D warnings` and the same with `--features gpu` pass.
- [x] 5.3 `cargo test -p pictura-render` (no feature) passes, and `cargo test -p pictura-render --features gpu --lib` passes what it can without a device.
- [x] 5.4 `cargo build -p pictura_app` still succeeds.
- [x] 5.5 `bash scripts/verify-fast.sh` passes.
- [x] 5.6 Re-take the closure and cold-check numbers and fill in `design.md`'s after table.
