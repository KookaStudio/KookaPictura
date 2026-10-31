# Spec Delta

## MODIFIED Requirements

### Requirement: Graceful CPU fallback when no GPU is available

The GPU path MUST NOT panic when no usable adapter exists. Adapter/device
creation failure, documents exceeding device buffer or workgroup limits, and
readback failure SHALL each produce a `GpuError` variant
(`Unavailable`, `TooLarge`, `Readback`). `composite_gpu_or_cpu(doc)` MUST return
the CPU compositor's result whenever `composite_gpu` returns any `GpuError`, and
its output for a CPU-only stack MUST be byte-identical to
`composite_rgba(doc)`.

#### Scenario: No Vulkan adapter

- **WHEN** `composite_gpu` runs on a machine with no usable adapter on the selected backend
- **THEN** it returns `Err(GpuError::Unavailable)` and does not panic, and
  `composite_gpu_or_cpu` returns the CPU composite

#### Scenario: Document exceeds device limits

- **WHEN** the document's buffers or dispatch count exceed the device limits
- **THEN** `composite_gpu` returns `Err(GpuError::TooLarge)` and does not panic

#### Scenario: Fallback for a CPU-only stack

- **WHEN** a document contains a color-luminosity or Dissolve layer and is passed
  to `composite_gpu_or_cpu`
- **THEN** the returned buffer equals `composite_rgba(doc)` byte for byte

### Requirement: The GPU compositor is validated against the CPU oracle before use

`gpu_available()` SHALL report a GPU as available only when a usable device
exists on the backend selected by `compositing/gpu-backend-selection` AND that
device reproduces the CPU oracle `composite_rgba` on a representative scene
within ±1 LSB per channel. The representative scene SHALL include partial alpha
and a separable blend, so a device that composites only a few pixels or drops
alpha is detected. The check SHALL run at most once per process and its result
SHALL be cached. When the check fails, `composite_gpu` SHALL return
`GpuError::Unavailable` for a supported document, `composite_active` and
`composite_gpu_or_cpu` SHALL keep the CPU path, `shared_device()` SHALL return
`None` so the GPU filter kernels also fall back to the CPU, and the GPU paint
stroke SHALL fall back to the CPU: `GpuStroke::available()` SHALL report false
and `GpuStroke::new` SHALL return `GpuError::Unavailable`, so a miscompiled
`STROKE_SHADER` never commits wrong paint pixels. The eligibility check SHALL
still precede the availability gate, so a `Dissolve` stack SHALL report
`GpuError::UnsupportedMode` even when the device is unusable. The public
signatures of `composite_gpu`, `composite_active`, `gpu_available`, and
`Backend` SHALL be unchanged. `GpuError` MAY gain variants that name a reason
for declining; its existing variants SHALL be unchanged.

#### Scenario: A device that matches the oracle stays enabled

- **WHEN** a usable device on the selected backend composites the representative
  scene and every channel differs from `composite_rgba` by at most 1 LSB
- **THEN** `gpu_available()` returns true, `composite_gpu` returns `Ok`, and
  `composite_active(doc, true)` reports `Backend::Gpu`

#### Scenario: A device that diverges is treated as absent

- **WHEN** a usable device on the selected backend composites the representative
  scene and any channel differs from `composite_rgba` by more than 1 LSB
- **THEN** `gpu_available()` returns false, `composite_gpu` returns
  `Err(GpuError::Unavailable)`, `composite_active(doc, true)` reports
  `Backend::Cpu` with bytes equal to `composite_rgba(doc)`, and `shared_device()`
  returns `None`

#### Scenario: The GPU paint stroke falls back on a diverging device

- **WHEN** the validation fails and the app tries to seed a GPU paint stroke
- **THEN** `GpuStroke::available()` returns false and `GpuStroke::new` returns
  `Err(GpuError::Unavailable)`, so the CPU stroke is used and no GPU dab patches
  the document

#### Scenario: The CPU-only mode check still precedes the gate

- **WHEN** `composite_gpu` is called on a document whose visible layer uses
  `Dissolve` on a device that fails the validation
- **THEN** it returns `Err(GpuError::UnsupportedMode(BlendMode::Dissolve))`, not
  `Unavailable`

#### Scenario: The check runs at most once

- **WHEN** `gpu_available()` is called repeatedly on the same process
- **THEN** the representative scene is composited at most once and the cached
  result is returned
