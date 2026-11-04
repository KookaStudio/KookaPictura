# Spec Delta

## MODIFIED Requirements

### Requirement: GPU counters

The render crate SHALL count resident-upload cache hits and misses, and SHALL
count every `GpuError` a GPU composite returns, by kind: eligibility declines
before the availability gate, `Unavailable` at the gate, and device errors
after it. Counting an eligibility decline SHALL NOT require a GPU device. When
metrics are on, the device supports timestamp queries, and a GPU composite runs,
the composite SHALL report its device-side execution time. Otherwise the device
time SHALL be absent, never reported as zero. The application SHALL request
timestamp-query support from the device only when metrics are on.

#### Scenario: A decline is attributed

- **WHEN** the GPU path declines a document because a visible layer uses
  `Dissolve`
- **THEN** the decline count for the unsupported-mode reason increases

#### Scenario: No timestamps, no fake zero

- **WHEN** a GPU composite timing has no device timestamps
- **THEN** its report line carries no device-time field
