# Proposal

## Why

On Windows the app crashes at startup on Intel Gen9 integrated graphics
(i5-6500 / HD Graphics 530) with an access violation inside `igvk64.dll`, the
Intel Vulkan ICD. The fault is native driver code, not a Rust panic, so no
in-process error handling can catch it. Both device-creation paths hardcode
Vulkan (`crates/pictura-render/src/gpu/backend.rs:237-299`,
`crates/pictura-app/src/gpu.rs:58-93`), the Windows build compiles only the
Vulkan wgpu feature, and `WGPU_BACKEND` is never read. The startup probe in
`crates/pictura-app/cpp/frame.cpp:151-155` therefore loads the crashing ICD
before any window is shown.

`Instance::new` initializes every backend named in its bitmask, so asking for
`DX12 | VULKAN` still loads the Vulkan ICD and still crashes. The process must
pin exactly one backend, which means the fallback chain cannot live inside one
process.

## What Changes

- Compile the existing wgpu `dx12` feature on Windows only. No new direct
  dependency; Windows-only transitive crates.
- Select one backend explicitly: Windows defaults to DX12 -> Vulkan -> CPU;
  Linux keeps the direct Vulkan path.
- Probe a candidate in an isolated child process (`--gpu-probe dx12|vulkan`).
  A driver crash or a 10 s hang kills only the child; the parent continues.
- Persist one outcome per backend in `kooka-pictura/gpu-backend.json` next to
  `state.json`: `ok`, `failed`, `crashed`, `unknown`, invalidated by app version.
- Write `gpu-init.marker` around the parent's own device init. A marker left by
  a crash marks that backend `crashed` on the next launch and moves to the next
  candidate.
- `WGPU_BACKEND` replaces the platform candidate order but still runs the child
  probe and still honors a `crashed` cache entry; `WGPU_BACKEND=none` forces CPU.
- Status bar and About show `GPU (DX12)`, `GPU (Vulkan)`, `CPU`,
  `CPU (no GPU)`, or `CPU (safe mode)`.
- `--interop-probe` and the zero-copy interop state remain Vulkan-only.
- Linux keeps the existing direct Vulkan path: no child probe, no cache, no
  marker.

## Capabilities

### New Capabilities

- `compositing/gpu-backend-selection`: the platform candidate order, the
  `WGPU_BACKEND` override, the isolated child probe, the per-backend cache, the
  crash marker, the Linux no-op path, and the Vulkan-only interop rule.

### Modified Capabilities

- `compositing/gpu-compute-backend`: the capability probe reports a usable
  adapter on the selected backend rather than Vulkan; the status bar reports
  the backend name and safe mode; the no-GPU fallback covers chain exhaustion
  while keeping a reset path.
- `compositing/gpu-compositing`: the CPU-oracle validation requirement applies
  to the selected GPU device, not to Vulkan specifically.
- `compositing/composite-view`: the offscreen render and self-test path use the
  selected backend and keep the CPU fallback.

## Impact

- `crates/pictura-render/Cargo.toml`, `crates/pictura-app/Cargo.toml` — add the
  `dx12` wgpu feature under `cfg(windows)` only.
- `crates/pictura-render/src/gpu/select.rs` (new) — backend enum, candidate
  resolution, compiled-backend intersection, process pin, status, labels.
- `crates/pictura-render/src/gpu/backend.rs` — parameterize `request_device`;
  `devices()` resolves the pin.
- `crates/pictura-render/src/gpu/mod.rs` — re-export the selection API; update
  `GpuError::Unavailable` wording.
- `crates/pictura-app/src/gpu.rs` — parameterize `create_device`; route the
  offscreen gradient through the selected backend; keep interop Vulkan-only.
- `crates/pictura-app/src/gpu_probe_bridge.rs` (new) and `src/gpu_probe_bridge/`
  — child probe, spawn/timeout/kill, pin, safe-mode flag; register in `build.rs`.
- `crates/pictura-app/cpp/gpu_probe.{h,cpp}` (new) — cache, marker,
  `prepareGpuBackend`, and the child spawn call into Rust; add to
  `CMakeLists.txt`.
- `crates/pictura-app/cpp/main.cpp` — handle `--gpu-probe` before
  `QApplication`; call `prepareGpuBackend()` for normal launches.
- `crates/pictura-app/cpp/frame.cpp`, `frame.h`, `frame_preferences.cpp`,
  `frame_session.cpp` — marker lifecycle, candidate availability, label; the
  cache reset lives in `PicturaMainWindow::applyGpuComputePreference`
  (`frame_session.cpp:190-200`), connected from `frame_preferences.cpp:107`.
- `crates/pictura-app/src/cxxqt_object/impl_core.rs` — `active_backend_label`;
  keep `active_backend` as the legacy token for the control protocol.
- `.github/workflows/ci.yml` — a Windows `pictura-render` compile/test job.
- `docs/` — document the chain, override, and interop rule; needs
  `TASK-ALLOWS-DOCS`.
- No new direct dependency; no data migration.
