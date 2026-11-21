# Spec Delta

## MODIFIED Requirements

### Requirement: An unavailable GPU reads as off

After the startup probe establishes that no GPU candidate is available, the
effective `gpuCompute` preference SHALL be false even when the preferences store
holds true. Preferences > Performance SHALL show **Use GPU Compute** unchecked
and disabled, and a subsequent save SHALL record false in preferences. A launch whose
candidate chain ended in safe mode SHALL retain the stored value, keep the toggle
enabled, and leave the off-and-on reset path intact.

#### Scenario: No candidate leaves the preference off [gb_no_gpu_unchecked]

- **WHEN** a launch has no usable adapter and no safe-mode reset path, and the Performance preferences page is opened
- **THEN** Use GPU Compute is unchecked and disabled, and saving preferences records `gpuCompute` false

#### Scenario: Safe mode keeps the stored preference [gb_safe_mode_stored]

- **WHEN** the candidate chain ended in safe mode and the stored `gpuCompute` is true
- **THEN** the toggle remains enabled and checked, and saving preferences keeps `gpuCompute` true

### Requirement: GPU compute preference persists

The requested `gpuCompute` value SHALL round-trip through the preferences
store. A profile without an override SHALL use the default of true before
applying backend availability policy.

#### Scenario: Round-trip

- **WHEN** `gpuCompute` is set to false and the preferences store is loaded again
- **THEN** the loaded `gpuCompute` is false

#### Scenario: Missing field defaults true

- **WHEN** a legacy schema-1 session without a `gpuCompute` field is migrated and no system setting overrides the default
- **THEN** `gpuCompute` is true
