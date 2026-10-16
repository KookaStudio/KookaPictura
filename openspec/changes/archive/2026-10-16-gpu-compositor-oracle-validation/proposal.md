# Proposal: gpu-compositor-oracle-validation

## Why

Issue #418: `./build/pictura --headless --self-test` fails with
`FAIL core fresh_white 5 fresh document not white`. The cause is a Vulkan driver
that accepts the compositor's compute pipelines but returns wrong pixels
(observed on Mesa's Honeykrisp for the Apple M1, Mesa 25.3.6): only the first
few canvas pixels are composited, so the fresh white scratch document presents
non-white. The existing fallback could not catch this — `gpu_available()` only
checked that a device existed, and a plausible-but-wrong composite is not a
`GpuError`, so every compositing call kept the broken GPU.

## What Changes

- `pictura_render::gpu` adds a one-time compositor validation.
  `gpu_available()` now also requires that the device reproduce
  `composite_rgba` on a small representative scene (an opaque gradient backdrop
  plus a half-transparent gradient `Multiply` source) within ±1 LSB. On a
  mismatch the GPU is treated as absent for the process.
- `composite_gpu` returns `GpuError::Unavailable` when the device fails the
  check. The CPU-only-mode check still runs first, so a `Dissolve` stack still
  reports `UnsupportedMode`.
- `shared_device()` returns `None` when the check fails, so the M27 GPU filter
  kernels also fall back to the CPU on a device that miscompiles the compositor.
- `GpuStroke::available()` and `GpuStroke::new` consult the same check, so a
  device that miscompiles `STROKE_SHADER` does not commit wrong paint pixels
  into the document (a GPU stroke commits with no CPU replay).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/gpu-compositing`: adds a device-validation requirement so the GPU
  is used only when it matches the CPU oracle.

## Impact

- `pictura-render` (`gpu/mod.rs`, `gpu/backend.rs`).
- No new dependency. `gpu_available()` gains a one-time composite cost on its
  first call.

## Provenance

No external source. The failure was reproduced locally against
`composite_rgba` as the oracle; the validation scene mirrors the existing
`gpu_parity` scene so a device that passes the check is the same device the
parity tests cover.
