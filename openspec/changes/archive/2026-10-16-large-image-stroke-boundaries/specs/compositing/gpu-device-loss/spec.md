## ADDED Requirements

### Requirement: GPU filter passes stay under the driver timeout

A GPU filter pass SHALL be split into submissions, each holding a bounded, estimated amount of shader work, so that no single job on a very large buffer can run past the driver's hang timeout. The split SHALL NOT change the output.

#### Scenario: Chunked submissions match one submission

- **WHEN** a filter runs once as one submission and once split into the smallest chunks
- **THEN** the two outputs are byte-identical, including the read-modify-write combine pass

### Requirement: A lost GPU device falls back to the CPU

When the driver reports the shared GPU device lost, compositing and filtering SHALL switch to the CPU for the rest of the session. They SHALL NOT panic. A composite in flight when the device is lost SHALL report a readback failure, so its caller falls back to the CPU.

#### Scenario: Uploads after a loss do not panic

- **WHEN** an upload to the lost device is refused
- **THEN** the composite returns an error, and the CPU composite is used instead
