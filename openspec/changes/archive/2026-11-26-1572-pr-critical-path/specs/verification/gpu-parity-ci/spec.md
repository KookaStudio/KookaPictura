# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
