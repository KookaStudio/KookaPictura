# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: A software Vulkan device is accepted only when the test feature is on`
- TO: `### Requirement: A software GPU device is accepted only when the test feature is on`

## MODIFIED Requirements

### Requirement: A software GPU device is accepted only when the test feature is on

`pictura-render` SHALL provide a `software-gpu` cargo feature, off by default.
With the feature on, the GPU probe SHALL accept a software (CPU-type) device on
any compiled backend, including a software Vulkan device and WARP on DX12. With
it off, the probe MUST keep rejecting software devices. No workspace crate SHALL
enable the feature in its manifest, so the application binary and every default
workspace build never treat a software device as a GPU.

#### Scenario: Production rejects a software device

- **WHEN** `pictura-render` is built without `software-gpu` and the probe sees a
  CPU-type device
- **THEN** the device is rejected

#### Scenario: The test feature accepts a software device

- **WHEN** `pictura-render` is built with `software-gpu` and the probe sees a
  CPU-type device
- **THEN** the device is accepted

#### Scenario: The application build leaves the feature off

- **WHEN** the `pictura_app` crate's tests are built with default features
- **THEN** its `pictura-render` dependency reports `software-gpu` disabled

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
- **THEN** it panics with a message naming the missing GPU device

#### Scenario: Device-free GPU checks always run

- **WHEN** the default test run executes the eligibility decline tests
- **THEN** they run and pass without a GPU device

### Requirement: CI runs the GPU parity tests on a software device

The CI workflow SHALL include a Linux job that installs a software Vulkan
driver. The job SHALL run every GPU-dependent correctness test in
`pictura-render`, in-crate and integration, with the feature on and ignored
tests enabled. Benchmark and profile tests SHALL be excluded by name. The job
SHALL fail when any of those tests fails, including a test that finds no usable
device. The `software-gpu` clippy run SHALL happen in the `lint` job, not in
this job: `lint` SHALL run `cargo clippy -p pictura-render --all-targets
--features software-gpu -- -D warnings` after the workspace clippy, so the
feature's code is linted once per run.

#### Scenario: The job exercises the GPU

- **WHEN** the `gpu-parity` job runs on a hosted runner
- **THEN** the availability test asserts `gpu_available()` is true and the GPU
  parity tests report passed results

#### Scenario: A broken driver setup fails the job

- **WHEN** the software Vulkan driver is missing from the job's runner
- **THEN** the job fails instead of passing on skipped tests

#### Scenario: The feature is linted in the lint job [gpu_clippy_in_lint]

- **WHEN** the CI workflow runs on a change that is not skipped
- **THEN** the `lint` job runs clippy on `pictura-render` with `software-gpu` and
  fails on a warning, and the `gpu-parity` job runs no clippy step

## ADDED Requirements

### Requirement: Windows CI runs the GPU parity tests on WARP with FXC

The Windows render CI job SHALL build `pictura-render` with the `software-gpu`
feature and run the GPU parity profile with ignored tests enabled. On a runner
without a hardware adapter, the DX12 probe SHALL accept WARP, the DX12 compiler
SHALL be FXC, and the parity suites SHALL report passed results. A run that
finds no usable device, or a parity mismatch, SHALL fail the job rather than
pass on skipped tests. The Vulkan-only software-device probe test SHALL NOT run
on Windows.

#### Scenario: The Windows job exercises WARP [gpu_windows_warp_parity]

- **WHEN** the Windows render job runs on a runner with only WARP
- **THEN** the DX12 probe accepts WARP and the parity suites report passed
  results

#### Scenario: FXC is the compiler on WARP [gpu_windows_fxc_probe]

- **WHEN** the Windows render job probes DX12 without `WGPU_DX12_COMPILER`
- **THEN** the probe succeeds and its detail names FXC

#### Scenario: A missing device fails the Windows job [gpu_windows_missing_device_fails]

- **WHEN** the Windows runner exposes no usable adapter even with the test
  feature
- **THEN** the job fails instead of passing on skipped tests
