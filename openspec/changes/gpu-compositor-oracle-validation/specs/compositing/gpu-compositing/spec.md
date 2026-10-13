# Spec Delta

## ADDED Requirements

### Requirement: The GPU compositor is validated against the CPU oracle before use

`gpu_available()` SHALL report a GPU as available only when a usable Vulkan
device exists AND that device reproduces the CPU oracle `composite_rgba` on a
representative scene within ±1 LSB per channel. The representative scene SHALL
include partial alpha and a separable blend, so a device that composites only a
few pixels or drops alpha is detected. The check SHALL run at most once per
process and its result SHALL be cached. When the check fails, `composite_gpu`
SHALL return `GpuError::Unavailable` for a supported document, `composite_active`
and `composite_gpu_or_cpu` SHALL keep the CPU path, and `shared_device()` SHALL
return `None` so the GPU filter kernels also fall back to the CPU. The CPU-only
blend-mode check SHALL still precede the availability gate, so a `Dissolve` stack
SHALL report `GpuError::UnsupportedMode` even when the device is unusable. The
public signatures of `composite_gpu`, `composite_active`, `gpu_available`,
`Backend`, and `GpuError` SHALL be unchanged.

#### Scenario: A device that matches the oracle stays enabled

- **WHEN** a usable Vulkan device composites the representative scene and every
  channel differs from `composite_rgba` by at most 1 LSB
- **THEN** `gpu_available()` returns true, `composite_gpu` returns `Ok`, and
  `composite_active(doc, true)` reports `Backend::Gpu`

#### Scenario: A device that diverges is treated as absent

- **WHEN** a usable Vulkan device composites the representative scene and any
  channel differs from `composite_rgba` by more than 1 LSB
- **THEN** `gpu_available()` returns false, `composite_gpu` returns
  `Err(GpuError::Unavailable)`, `composite_active(doc, true)` reports
  `Backend::Cpu` with bytes equal to `composite_rgba(doc)`, and `shared_device()`
  returns `None`

#### Scenario: The CPU-only mode check still precedes the gate

- **WHEN** `composite_gpu` is called on a document whose visible layer uses
  `Dissolve` on a device that fails the validation
- **THEN** it returns `Err(GpuError::UnsupportedMode(BlendMode::Dissolve))`, not
  `Unavailable`

#### Scenario: The check runs at most once

- **WHEN** `gpu_available()` is called repeatedly on the same process
- **THEN** the representative scene is composited at most once and the cached
  result is returned
