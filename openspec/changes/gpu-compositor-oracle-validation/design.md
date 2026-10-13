# Design: gpu-compositor-oracle-validation

## Context

`composite_gpu` mirrors `composite_rgba` through a wgpu compute shader. The
`gpu_parity` suite proves the two agree within ±1 LSB on a working adapter, and
the per-call fallback (`composite_active`, `composite_gpu_or_cpu`,
`composite_region_active`) already keeps the CPU path on any `GpuError`.

That contract assumes a driver that either works or errors. Honeykrisp (Mesa
25.3.6, Apple M1) instead accepts the pipelines and returns wrong pixels: a
64-pixel white canvas comes back with only the first 4 pixels set, and the
`gpu_parity` scenes diverge from the CPU oracle by more than 1 LSB. Because the
output is not a `GpuError`, `gpu_available()` reported the GPU as usable and the
app presented the wrong composite — the `fresh_white` self-test check caught it.

Isolated probes confirmed the shader source is correct (a trivial compute shader,
a storage-buffer upload/readback, a variable shift, and a modulo all behave),
and the same shaders pass on other adapters. Requesting the adapter's raw limits
changed the symptom but not the divergence, so the fault is in the driver's
compilation/execution of the full compositor shader, not in Kooka's WGSL.

## Goals / Non-Goals

- **Goal:** never present a GPU composite that disagrees with the CPU oracle.
- **Goal:** keep the GPU on adapters that do agree (the RTX 3090 in the
  `gpu_parity` record).
- **Non-goal:** repairing or detecting the driver bug at the shader level. The
  WGSL is correct; the driver is not.

## Decisions

### Validate once, against the CPU oracle, on a representative scene

`gpu_available()` becomes `devices().is_ok() && gpu_composite_verified()`, where
`gpu_composite_verified` composites a tiny scene through the ungated kernel and
diffs it against `composite_rgba` within ±1 LSB. The result is cached in a
`OnceLock`, so the check runs at most once per process.

The scene must exercise the paths a broken driver gets wrong: a uniform-white
stack passes on Honeykrisp (only the first pixels matter and they are white),
while the gradient backdrop plus a half-transparent gradient `Multiply` source
diverges. The scene is kept at 8×8 so the check is cheap.

### Gate at the availability boundary, not per call

`composite_gpu_region` returns `GpuError::Unavailable` when the check fails, so
the existing per-call fallback needs no change and `composite_gpu`'s callers
(including the `gpu_parity` tests) keep their skip-on-`Unavailable` behaviour.
The CPU-only-mode check stays ahead of the gate so a `Dissolve` stack still
reports `UnsupportedMode` on an unusable device.

The ungated kernel is split into `composite_gpu_region_raw`; the validation calls
it directly so it does not recurse through `gpu_available`.

### Disable the filter kernels and the GPU stroke too

`shared_device()` is the entry point the M27 GPU filter kernels share with the
compositor. A device that miscompiles the compositor miscompiles those kernels,
so `shared_device()` returns `None` when the check fails and `filter_gpu_available`
reports no GPU.

The GPU paint stroke reaches the device through `devices()` directly, not
`shared_device()`, and the app seeds it via `GpuStroke::new(...).ok()`, not
`GpuStroke::available()`. The gate therefore lives in both: `available()` returns
`gpu_available()` and `new` returns `GpuError::Unavailable` when the check fails.
This matters because a GPU stroke patches the working document in place and
commits with no CPU replay, so a miscompiled `STROKE_SHADER` would write wrong
pixels into the document.

## Risks / Trade-offs

- A device with a legitimate >1 LSB rounding difference would be disabled. The
  parity contract already requires ±1 LSB, so this does not weaken it.
- The check costs one small composite on the first `gpu_available()` call; the
  `OnceLock` bounds it to one.

## Migration Plan

None. `gpu_available`, `composite_gpu`, `composite_active`, `Backend`, and
`GpuError` keep their signatures; only `gpu_available`'s answer can change from
true to false on a device that fails the check.

## Open Questions

None.
