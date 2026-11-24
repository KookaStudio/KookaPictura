# Spec Delta

## ADDED Requirements

### Requirement: The renderer's GPU stack is an optional feature

`crates/pictura-render` SHALL carry its wgpu/Vulkan stack behind an optional
`gpu` feature that is off by default. With default features — no `gpu` — the
crate SHALL compile, test, and expose the CPU compositing API without any
`wgpu` or `pollster` dependency in the build. The crate's default feature set
SHALL be empty; `wgpu` and `pollster` SHALL be optional dependencies enabled
only by `gpu`, and the test-only `software-gpu` feature SHALL imply `gpu`.

With the feature disabled, `pictura_render::Backend` and the CPU composite entry
points (`CompositeRequest`, `composite`, `composite_with_blend_time`,
`DocumentComposite`) SHALL remain available with unchanged names and signatures,
and a request made with `gpu_enabled` set SHALL composite on the CPU and report
`Backend::Cpu`.

With the feature enabled, the crate SHALL expose the same GPU surface the
application consumes today: `pub mod gpu`, and the root re-exports of
`probe_backend`, `backend_label`, `set_backend`, `selected_backend`,
`gpu_available`, `device_available`, `gpu_adapter_info`, `gpu_backend_status`,
`compiled_backends`, `env_candidates`, `resolve_candidates`, `platform`,
`dx12_compiler`, `dx12_compiler_label`, `probe_backend_outcome`,
`set_safe_mode`, `Backend`, `GpuAdapterInfo`, `GpuBackend`, `GpuBackendStatus`,
`GpuError`, `Platform`, `ProbeOutcome`, `ProbeReport`, together with
`apply_filter_active` and `filter_gpu_available`. The GPU gate SHALL NOT remove
or rename any of these items.

`pictura-app` SHALL enable the `gpu` feature on its `pictura-render` dependency,
so the application build and its GPU code paths are unchanged, and its
`pictura-render` dev-dependency SHALL keep enabling only the test oracle, which
Cargo unifies with the dependency for test builds. The `gpu` nextest profile and
the `gpu-parity` CI job SHALL continue to be invoked with a feature that enables
`gpu`.

#### Scenario: A default build compiles without wgpu [bp_gpu_default_off]

- **WHEN** `pictura-render` is built or tested with its default features
- **THEN** the manifest declares `wgpu` and `pollster` as optional, lists no
  default features, and the crate compiles and its tests pass without either
  dependency in the graph

#### Scenario: The GPU surface is unchanged when the feature is on [bp_gpu_surface]

- **WHEN** `pictura-render` is built with `--features gpu`
- **THEN** `pictura_render::gpu` and every root re-export listed above resolve,
  and `pictura_render::gpu::Backend` is the same type as
  `pictura_render::Backend`

#### Scenario: A CPU-only build answers a GPU-enabled request on the CPU [bp_gpu_cpu_composite]

- **WHEN** a `CompositeRequest` with `gpu_enabled` set is composited in a build
  without the `gpu` feature
- **THEN** the result is byte-identical to `composite_rgba` and reports
  `Backend::Cpu`

#### Scenario: The application enables the GPU stack [bp_gpu_app_enables]

- **WHEN** the `pictura-app` manifest is inspected
- **THEN** its `pictura-render` dependency enables the `gpu` feature, and its
  dev-dependency enables only `test-oracle` (unified with the dependency for
  test builds)

#### Scenario: The software device spelling implies the feature [bp_gpu_software]

- **WHEN** the manifest's `software-gpu` feature definition is inspected
- **THEN** it contains `gpu`, so the `gpu-parity` job's
  `--features software-gpu` enables the GPU stack

#### Scenario: GPU tests still require a device [bp_gpu_tests]

- **WHEN** the `gpu` nextest profile runs a device-dependent test in a build with
  the feature enabled but no usable adapter
- **THEN** the test fails on its `require_gpu` assertion rather than passing or
  silently skipping
