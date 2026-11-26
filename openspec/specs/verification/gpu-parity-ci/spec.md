# verification/gpu-parity-ci Specification

## Purpose
Run the GPU parity tests in CI on lavapipe and a DX12 device smoke check on
WARP, and report a GPU test that did not run as skipped, never as passed. The
CPU oracle stays the only reference.

## Requirements

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

### Requirement: Fixture scenes run through the active backend

The ImageMagick fixture scenes, which cover the blend modes they map and the
partial-alpha scene, SHALL be composited through
`composite(CompositeRequest { doc, region: None, gpu_enabled: true })` in a
device-dependent test. Each result SHALL either report `Backend::Gpu` and lie
within ±1 LSB per channel of `composite_rgba(doc)`, or report `Backend::Cpu`
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

### Requirement: Windows CI runs a DX12 device smoke check on WARP with FXC
The Windows render CI job SHALL keep both `pictura-render` Clippy steps,
including the `software-gpu` feature, and SHALL create the DX12 device on WARP
without running the compositor validation composite, because WARP loses the
device during that composite. The check SHALL fail when the device cannot be
created or when the configured compiler is not FXC. The job SHALL NOT run the
Windows CPU test commands that the Linux workspace nextest run already covers.
The full GPU parity suite SHALL remain on the Linux lavapipe job.

#### Scenario: The Windows job creates the DX12 device [gpu_windows_warp_device]
- **WHEN** the Windows render job runs with the `software-gpu` feature
- **THEN** it creates the DX12 device on WARP and reports success

#### Scenario: FXC is the compiler on WARP [gpu_windows_fxc_probe]
- **WHEN** the Windows render job creates the device without `WGPU_DX12_COMPILER`
- **THEN** the configured compiler is FXC

#### Scenario: A missing device fails the Windows job [gpu_windows_missing_device_fails]
- **WHEN** the Windows runner exposes no usable adapter even with the test feature
- **THEN** the job fails instead of passing on skipped tests

#### Scenario: Windows does not duplicate Linux CPU tests [gpu_windows_no_cpu_tests]
- **WHEN** the Windows render job runs
- **THEN** it runs Clippy and the DX12 device smoke, and no `cargo test` command other than the DX12 smoke

### Requirement: Render CI jobs use the precise render bucket
The `gpu-parity` and `windows-render` jobs SHALL run only when the classifier's
`render` output is true or when that output is missing because classification
failed. The render bucket SHALL include these paths:

- `crates/pictura-render/**`
- `crates/pictura-core/**`
- `crates/pictura-codec/**`
- `crates/pictura-color/**`
- `crates/pictura-adjust/**`
- `crates/pictura-filters/**`
- `crates/pictura-ops/**`
- `crates/pictura-paint/**`
- `crates/pictura-select/**`
- `crates/pictura-testkit/**`
- `crates/pictura-app/src/gpu*`
- `crates/pictura-app/Cargo.toml`
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and `.cargo/**`
- `.github/workflows/**`

It SHALL NOT include changes limited to `crates/pictura-mcp/**` or
`crates/pictura-script/**`.

#### Scenario: An engine-only change skips GPU CI [gpu_precise_render_gate]
- **WHEN** a pull request changes only `crates/pictura-mcp/**` or `crates/pictura-script/**`
- **THEN** `render` is false and neither `gpu-parity` nor `windows-render` runs

#### Scenario: A renderer change runs GPU CI [gpu_renderer_change]
- **WHEN** a pull request changes `crates/pictura-render/**`
- **THEN** `render` is true and both render jobs run

#### Scenario: A CI workflow change runs GPU CI [gpu_workflow_change]
- **WHEN** a pull request changes `.github/workflows/ci.yml`
- **THEN** `render` is true and both render jobs run

#### Scenario: Classifier failure cannot skip render coverage [gpu_classifier_failure]
- **WHEN** classification fails and publishes the run-all outputs
- **THEN** both render jobs run even though the classifier job failed
