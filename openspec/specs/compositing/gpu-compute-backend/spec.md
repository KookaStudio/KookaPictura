# gpu-compute-backend Specification

## Purpose
GPU compute as the default compositing backend with a capability probe, a disable switch, and persisted preference.

## Requirements

### Requirement: GPU compute is the default compositing backend

The system SHALL composite documents through
`pictura_render::gpu::composite_active(doc, gpu_enabled) -> (PixelBuffer, Backend)`.
When `gpu_enabled` is true and `pictura_render::gpu::gpu_available()` reports a
usable adapter on the backend selected by `compositing/gpu-backend-selection`,
`composite_active` SHALL return the GPU composite and `Backend::Gpu`; otherwise
it SHALL return the CPU composite and `Backend::Cpu`. The returned buffer MUST
match `pictura_render::composite_rgba(doc)` within ±1 LSB per channel regardless
of the chosen backend.

#### Scenario: Adapter present and GPU enabled

- **WHEN** a document is composited with `gpu_enabled` true on a machine with a usable adapter on the selected backend
- **THEN** `composite_active` returns `Backend::Gpu` and a buffer within ±1 LSB of `composite_rgba(doc)`

#### Scenario: GPU disabled or unavailable

- **WHEN** `gpu_enabled` is false, or no usable adapter exists on the selected backend
- **THEN** `composite_active` returns `Backend::Cpu` and a buffer byte-equal to `composite_rgba(doc)`

### Requirement: Capability probe

`pictura_render::gpu::gpu_available()` SHALL report whether a usable adapter and
device exist on the backend selected by `compositing/gpu-backend-selection`. The
probe MUST be cheap to call repeatedly (its result is cached) and MUST NOT panic.
It SHALL return false when no candidate is selected or no usable adapter exists.

#### Scenario: Probe reports a usable adapter

- **WHEN** `gpu_available()` is called on a machine with a working adapter for the selected backend
- **THEN** it returns true, and repeated calls return the same cached value without re-creating the device

#### Scenario: No candidate is selected

- **WHEN** the selection is empty, including `WGPU_BACKEND=none`
- **THEN** `gpu_available()` returns false and does not panic

#### Scenario: Probe reports no adapter

- **WHEN** `gpu_available()` is called on a machine with no usable adapter for the selected backend
- **THEN** it returns false and does not panic

### Requirement: User can disable GPU compute

A boolean session preference `gpuCompute` SHALL control GPU compositing. Its
default SHALL be true, and a missing value SHALL be read as true. When `gpuCompute`
is false the system MUST force the CPU backend for every composite while keeping
results within the parity tolerance. A checkable, implemented command SHALL toggle
the preference and reflect the persisted value.

#### Scenario: Disabling forces the CPU backend

- **WHEN** `gpuCompute` is false and a document is composited
- **THEN** every call returns `Backend::Cpu` with a buffer within ±1 LSB of `composite_rgba(doc)`

#### Scenario: Toggle command reflects the value

- **WHEN** the GPU-compute command is invoked
- **THEN** the preference flips, the document recomposites on the newly selected backend, and the command's checked state matches the persisted `gpuCompute` value

### Requirement: GPU compute preference persists

The `gpuCompute` value SHALL round-trip through the XDG session store
(`crates/pictura-app/cpp/session.{h,cpp}`). The session store's schema version
SHALL be bumped to 2. A store without the field MUST load `gpuCompute` as true.

#### Scenario: Round-trip

- **WHEN** a session with `gpuCompute` false is saved and then loaded
- **THEN** the loaded `gpuCompute` is false, with the store reporting schema version 2

#### Scenario: Missing field defaults true

- **WHEN** a schema-1 store without a `gpuCompute` field is loaded
- **THEN** `gpuCompute` is true

### Requirement: Status bar reports the active backend

The status bar SHALL show the active compositing backend with exactly one of
five labels: `GPU (DX12)` when the Direct3D 12 backend is active, `GPU (Vulkan)`
when the Vulkan backend is active, `CPU` when the user disabled GPU compute,
`CPU (no GPU)` when no usable adapter exists and no backend is blocked, and
`CPU (safe mode)` when an adapter was found but the GPU path is blocked by a
failed or crashed candidate. The label SHALL name the API of the adapter actually
selected, not a compiled-in default. Help > About SHALL show the same label.

#### Scenario: Backend indicator values

- **WHEN** the status bar is updated with GPU compute active on DX12, active on Vulkan, user-disabled, with no adapter available, and with a blocked backend
- **THEN** it shows `GPU (DX12)`, `GPU (Vulkan)`, `CPU`, `CPU (no GPU)`, and `CPU (safe mode)` respectively

#### Scenario: A DX12 adapter is active [gb_label_dx12]

- **WHEN** GPU compute is enabled and the selected adapter uses the Direct3D 12 backend
- **THEN** the status bar shows `GPU (DX12)`

#### Scenario: A Vulkan adapter is active [gb_label_vulkan]

- **WHEN** GPU compute is enabled and the selected adapter uses the Vulkan backend
- **THEN** the status bar shows `GPU (Vulkan)`

#### Scenario: The user disables GPU compute [gb_label_cpu]

- **WHEN** a usable adapter exists and `gpuCompute` is false
- **THEN** the status bar shows `CPU`

#### Scenario: No usable adapter [gb_label_no_gpu]

- **WHEN** no usable adapter exists and no GPU backend is blocked
- **THEN** the status bar shows `CPU (no GPU)`

#### Scenario: The GPU path is blocked [gb_label_safe_mode]

- **WHEN** an adapter was found but the device was lost, failed validation, or a cached crash fallback blocks the backend
- **THEN** the status bar shows `CPU (safe mode)`

### Requirement: No GPU never blocks a document

When no candidate is usable, or the candidate chain ends in safe mode, the system
SHALL fall back to the CPU backend, compositing SHALL continue to work without a
panic or an error dialog, consistent with the existing `Graceful CPU fallback`
requirement. The GPU-compute toggle SHALL be disabled only when no GPU backend is
compiled or present. In safe mode the toggle SHALL remain enabled so that turning
it off and on clears the cache and lets a later launch retry.

#### Scenario: Document opens without a GPU

- **WHEN** a document is opened on a machine where no candidate is usable
- **THEN** it composites on the CPU without a panic or error dialog

#### Scenario: Toggle stays disabled without an adapter

- **WHEN** the GPU-compute command is offered while no GPU backend is compiled or present
- **THEN** it is disabled/greyed and invoking it does not enable the GPU backend

#### Scenario: Safe mode keeps the reset path [gb_safe_mode_toggle]

- **WHEN** the chain ended in safe mode and the user turns GPU compute off and back on
- **THEN** the toggle is available, the cache is cleared, and the next launch retries the candidates
