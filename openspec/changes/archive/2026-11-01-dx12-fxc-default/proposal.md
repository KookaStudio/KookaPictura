# Proposal

## Why

On Windows, v0.3.0 falls back to the CPU on an Intel HD 530 even though DX12 is
compiled in. wgpu 30's `Dx12Compiler::Auto` loads the `dxcompiler.dll` that
windeployqt ships (DXC ~1.5), and only a *load* failure falls back to FXC; wgpu
30 requires DXC >= 1.8.2502, so the DX12 instance or its pipelines fail and the
launch stays on the CPU. Because both device-creation sites use
`InstanceDescriptor::new_without_display_handle()`, `WGPU_DX12_COMPILER` is never
read and a user cannot force FXC.

The same bug is invisible to CI: the Windows runner has only WARP, which wgpu
reports as a CPU-type device, and `accelerates()` rejects CPU devices unless the
test feature is on. A Windows build that cannot compile a single compositor
shader would stay green.

## What Changes

- **DX12 uses FXC by default.** One helper in `pictura-render` returns
  `Dx12Compiler::Fxc`, or `Dx12Compiler::from_env()` when
  `WGPU_DX12_COMPILER` is set. Both device-creation sites use it. The system
  requests no shader-model-6-only feature, so SM 5.1 is sufficient.
- **The probe records a diagnostic detail.** A probe returns a one-line detail
  next to its outcome: on success the adapter name, driver, and DX12 compiler;
  on failure the `RequestAdapterError` / `RequestDeviceError` text. The parent
  writes it into the existing `detail` field of `gpu-backend.json`, bounded in
  length.
- **Windows CI proves the DX12 path on WARP.** The `software-vulkan` test
  feature already accepts any CPU-type device, whatever the backend; rename it
  `software-gpu`, and run the GPU parity profile on the Windows runner. A
  missing device or a parity mismatch fails the job instead of skipping.
- **No cache-policy change.** `WGPU_BACKEND` continues to honor a cached
  `crashed`; manual diagnosis uses the existing reset path (re-enabling GPU
  Compute, or deleting `gpu-backend.json`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/gpu-backend-selection`: DX12 gains a default shader compiler
  (FXC) with an environment override, and a probe outcome gains a persisted
  diagnostic detail. The candidate order, pin, probe isolation, cache, and
  marker requirements are unchanged.
- `verification/gpu-parity-ci`: the test feature admits a software GPU on any
  backend (lavapipe or WARP), and a Windows CI job runs the GPU parity suites on
  WARP with the FXC-backed DX12 path.

## Impact

- `crates/pictura-render/src/gpu/{select,backend,mod}.rs`, `src/lib.rs`.
- `crates/pictura-app/src/gpu.rs`, `src/gpu_probe_bridge.rs`,
  `src/gpu_probe_bridge/imp.rs`, `cpp/gpu_probe.{h,cpp}`,
  `cpp/tests/tst_gpu_backend.cpp`.
- `.github/workflows/ci.yml` (`windows-render`; the `lint` job's clippy flag),
  `.config/nextest.toml` comment.
- Test feature rename in `pictura-render` and the `pictura-app` guard test.
- Docs: `docs/01-architecture/gpu-rendering-pipeline.md`, `docs/GLOSSARY.md`,
  `docs/dev/STATE.md`, `docs/dev/testing-conventions.md`, `DEVELOPING.md`.
- No new dependencies; no cache schema bump (`detail` is already optional).
- Windows packaging is unchanged: the Qt-deployed DXC DLLs stay in place but
  are not loaded by default.
