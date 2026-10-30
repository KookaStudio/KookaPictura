# verification/gpu-parity-ci Specification

## Purpose
Run the GPU parity tests in CI on a software Vulkan device, and report a GPU
test that did not run as skipped, never as passed. The CPU oracle stays the
only reference.

## Requirements

### Requirement: A software Vulkan device is accepted only when the test feature is on

`pictura-render` SHALL provide a `software-vulkan` cargo feature, off by
default. With the feature on, the GPU probe SHALL accept a software (CPU-type)
Vulkan device. With it off, the probe MUST keep rejecting software devices. No
workspace crate SHALL enable the feature in its manifest, so the application
binary and every default workspace build never treat a software device as a
GPU.

#### Scenario: Production rejects a software device

- **WHEN** `pictura-render` is built without `software-vulkan` and the probe
  sees a CPU-type device
- **THEN** the device is rejected

#### Scenario: The test feature accepts a software device

- **WHEN** `pictura-render` is built with `software-vulkan` and the probe sees a
  CPU-type device
- **THEN** the device is accepted

#### Scenario: The application build leaves the feature off

- **WHEN** the `pictura_app` crate's tests are built with default features
- **THEN** its `pictura-render` dependency reports `software-vulkan` disabled

### Requirement: GPU-dependent tests are skipped, never silently passed

A correctness test whose assertion needs a usable GPU device SHALL be ignored by
default, with a reason string, so that when it does not run the unified test
report counts it as skipped. When such a test is run, it MUST fail if no usable
device exists. It MUST NOT return early with success. Tests that check GPU
behavior without a device SHALL NOT be ignored: eligibility declines, the
disabled-GPU path, and availability consistency are examples. Opt-in benchmark
and profile tests gated by `PICTURA_GPU_BENCH`, or that only print timings, are
outside this requirement.

#### Scenario: A default run reports GPU tests as skipped

- **WHEN** `scripts/test-report.sh` runs with ignored tests disabled
- **THEN** the GPU-dependent parity tests appear in the skipped count, not the
  passed count

#### Scenario: A run without a device fails

- **WHEN** the device-requirement helper runs with no usable device
- **THEN** it panics with a message naming the missing Vulkan device

#### Scenario: Device-free GPU checks always run

- **WHEN** the default test run executes the eligibility decline tests
- **THEN** they run and pass without a GPU device

### Requirement: CI runs the GPU parity tests on a software device

The CI workflow SHALL include a Linux job that installs a software Vulkan
driver. The job SHALL run every GPU-dependent correctness test in `pictura-render`, in-crate and
integration, with the feature on and ignored tests enabled. Benchmark and
profile tests SHALL be excluded by name. The job SHALL fail when any of those
tests fails, including a test that finds no usable device.
The `software-vulkan` clippy run SHALL happen in the `lint` job, not in this
job: `lint` SHALL run `cargo clippy -p pictura-render --all-targets --features
software-vulkan -- -D warnings` after the workspace clippy, so the feature's
code is linted once per run.

#### Scenario: The job exercises the GPU

- **WHEN** the `gpu-parity` job runs on a hosted runner
- **THEN** the availability test asserts `gpu_available()` is true and the GPU
  parity tests report passed results

#### Scenario: A broken driver setup fails the job

- **WHEN** the software Vulkan driver is missing from the job's runner
- **THEN** the job fails instead of passing on skipped tests

#### Scenario: The feature is linted in the lint job [gpu_clippy_in_lint]

- **WHEN** the CI workflow runs on a change that is not skipped
- **THEN** the `lint` job runs clippy on `pictura-render` with `software-vulkan` and fails on a warning, and the `gpu-parity` job runs no clippy step

### Requirement: Fixture scenes run through the active backend

The ImageMagick fixture scenes, which cover the blend modes they map and the
partial-alpha scene, SHALL be composited through `composite_active(doc, true)`
in a device-dependent test. Each result SHALL either report `Backend::Gpu` and
lie within ±1 LSB per channel of `composite_rgba(doc)`, or report `Backend::Cpu`
with bytes equal to `composite_rgba(doc)`. The reference SHALL be the CPU
oracle, never the ImageMagick output.

#### Scenario: A GPU-eligible scene matches the oracle

- **WHEN** a Multiply fixture scene is composited through the active backend on
  the software device
- **THEN** it reports `Backend::Gpu` and every channel is within 1 LSB of
  `composite_rgba`

#### Scenario: A declined scene equals the oracle

- **WHEN** a fixture scene changed to `Dissolve` is composited through the active
  backend
- **THEN** it reports `Backend::Cpu` and its bytes equal `composite_rgba`
