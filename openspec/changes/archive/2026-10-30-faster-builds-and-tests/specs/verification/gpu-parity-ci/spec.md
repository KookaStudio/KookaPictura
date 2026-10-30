## MODIFIED Requirements

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
