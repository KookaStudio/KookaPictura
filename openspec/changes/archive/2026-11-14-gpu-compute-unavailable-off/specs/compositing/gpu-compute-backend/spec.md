# Spec Delta

## ADDED Requirements

### Requirement: An unavailable GPU reads as off

After the startup probe establishes that no GPU candidate is available, the
effective `gpuCompute` preference SHALL be false even when the session store
holds true. Preferences > Performance SHALL show **Use GPU Compute** unchecked
and disabled, and a subsequent session save SHALL record false. A launch whose
candidate chain ended in safe mode SHALL retain the stored value, keep the toggle
enabled, and leave the off-and-on reset path intact.

#### Scenario: No candidate leaves the preference off [gb_no_gpu_unchecked]

- **WHEN** a launch has no usable adapter and no safe-mode reset path, and the Performance preferences page is opened
- **THEN** Use GPU Compute is unchecked and disabled, and saving the session records `gpuCompute` false

#### Scenario: Safe mode keeps the stored preference [gb_safe_mode_stored]

- **WHEN** the candidate chain ended in safe mode and the stored `gpuCompute` is true
- **THEN** the toggle remains enabled and checked, and saving the session keeps `gpuCompute` true

## MODIFIED Requirements

### Requirement: User can disable GPU compute

A boolean session preference `gpuCompute` SHALL control GPU compositing. Its
default SHALL be true, and a missing value SHALL be read as true. When `gpuCompute`
is false the system MUST force the CPU backend for every composite while keeping
results within the parity tolerance. The Preferences > Performance **Use GPU
Compute** checkbox SHALL toggle the preference and reflect the effective value.

#### Scenario: Disabling forces the CPU backend

- **WHEN** `gpuCompute` is false and a document is composited
- **THEN** every call returns `Backend::Cpu` with a buffer within ±1 LSB of `composite_rgba(doc)`

#### Scenario: Toggle command reflects the value

- **WHEN** the removed `view.gpuCompute` command is requested
- **THEN** it is not offered, and the Preferences > Performance **Use GPU Compute** checkbox is the toggle

#### Scenario: Toggle reflects the value

- **WHEN** the Preferences > Performance **Use GPU Compute** checkbox is toggled
- **THEN** the preference flips, the active view and checkbox reflect the new value, and a session save records it

### Requirement: No GPU never blocks a document

When no candidate is usable, or the candidate chain ends in safe mode, the system
SHALL fall back to the CPU backend, compositing SHALL continue to work without a
panic or an error dialog, consistent with the existing `Graceful CPU fallback`
requirement. The GPU-compute toggle SHALL be disabled only when no GPU candidate
is available: no usable adapter and no safe-mode reset path. In safe mode the
toggle SHALL remain enabled so that turning it off and on clears the cache and
lets a later launch retry.

#### Scenario: Document opens without a GPU

- **WHEN** a document is opened on a machine where no candidate is usable
- **THEN** it composites on the CPU without a panic or an error dialog

#### Scenario: Toggle stays disabled without an adapter

- **WHEN** the Preferences > Performance **Use GPU Compute** toggle is offered while no GPU candidate is available (no usable adapter and no safe-mode reset path)
- **THEN** it is disabled and unchecked, and the document does not request the GPU backend

#### Scenario: Safe mode keeps the reset path [gb_safe_mode_toggle]

- **WHEN** the chain ended in safe mode and the user turns GPU compute off and back on
- **THEN** the toggle is available, the cache is cleared, and the next launch retries the candidates
