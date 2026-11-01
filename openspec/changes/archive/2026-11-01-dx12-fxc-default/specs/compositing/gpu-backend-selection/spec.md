# Spec Delta

## ADDED Requirements

### Requirement: The DX12 backend defaults to the FXC shader compiler

The system SHALL configure every DX12 instance with the FXC shader compiler by
default. When `WGPU_DX12_COMPILER` is set, the system SHALL use the compiler it
names instead. The policy SHALL apply to every device-creation path: the
compositor device, the isolated probe child, and the offscreen render device.
The default MUST NOT depend on the presence or version of a deployed
`dxcompiler.dll`. The system requests no shader-model-6-only feature, so the
FXC (shader model 5.1) path SHALL be the supported default.

#### Scenario: FXC is the default compiler [gb_dx12_fxc_default]

- **WHEN** a DX12 instance is created with `WGPU_DX12_COMPILER` unset
- **THEN** its shader compiler is FXC

#### Scenario: The environment overrides the compiler [gb_dx12_compiler_env]

- **WHEN** `WGPU_DX12_COMPILER` names FXC, DXC, or the static DXC
- **THEN** the DX12 instance uses that compiler instead of the default

#### Scenario: Every device-creation path uses the policy [gb_dx12_compiler_all_paths]

- **WHEN** a device is created for the compositor, the probe, or the offscreen render
- **THEN** it is created from the same DX12 compiler policy

### Requirement: A probe outcome carries a diagnostic detail

A probe SHALL produce a one-line detail alongside its outcome. On success the
detail SHALL name the adapter, its driver, and, for DX12, the compiler. On
failure the detail SHALL contain the `RequestAdapterError` or
`RequestDeviceError` text, or the validation failure. The parent SHALL write the
detail into the `detail` field of that backend's `gpu-backend.json` entry on
every probe and SHALL bound its length. The detail is diagnostic text and MUST
NOT change the outcome classification.

#### Scenario: Success names the adapter and compiler [gb_probe_detail_ok]

- **WHEN** a DX12 probe succeeds
- **THEN** its detail names the adapter, the driver, and the FXC compiler

#### Scenario: Failure names the request error [gb_probe_detail_failure]

- **WHEN** a probe fails because no adapter or device can be created
- **THEN** its detail contains the request error text

#### Scenario: The detail is persisted [gb_probe_detail_persisted]

- **WHEN** the parent records a probe outcome
- **THEN** the backend's `gpu-backend.json` entry carries the probe detail

#### Scenario: The detail is bounded [gb_probe_detail_bounded]

- **WHEN** a probe produces a detail longer than the cache allows
- **THEN** the stored detail is truncated to the bound and the launch continues
