## MODIFIED Requirements

### Requirement: Default-on GPU filters

Filter application SHALL use the GPU by default when the M26 `gpuCompute`
preference is enabled and a usable adapter exists, consistent with the default-on
compositing requirement. When `gpuCompute` is disabled the system MUST force the
CPU implementation for every filter. A layer filter SHALL run on the CPU, even
with the GPU enabled, for the kernels whose row-parallel CPU path outruns the
GPU round trip: Gaussian Blur, Blur, Blur More, Sharpen, Sharpen More, Unsharp
Mask, High Pass, Maximum, Minimum, Median and Custom. Its result is then
byte-identical to `pictura_filters::apply`. `apply_filter_active` itself still
uses the GPU whenever a kernel exists.

#### Scenario: Default routes to the GPU

- **WHEN** a supported filter is applied with the default `gpuCompute` value on a
  machine with a usable adapter
- **THEN** the GPU backend runs the filter

#### Scenario: Disabling forces the CPU

- **WHEN** `gpuCompute` is false and a supported filter is applied
- **THEN** `pictura_filters::apply` runs on the CPU and the reported backend is
  `Backend::Cpu`

#### Scenario: A CPU-faster kernel skips the GPU on a layer

- **WHEN** a Gaussian Blur, Unsharp Mask, Median or Maximum is applied to a layer
  with GPU compute enabled
- **THEN** the layer equals the same filter applied with GPU compute disabled

## ADDED Requirements

### Requirement: Radial Blur Zoom runs on the GPU

With GPU compute enabled, Radial Blur's Zoom method SHALL run on the GPU and
match `pictura_filters::apply` within ±1 LSB. Its samples lie along each
pixel's ray, so neighbouring invocations read neighbouring memory. Spin SHALL
stay on the CPU, with a byte-identical result. Its samples sweep arcs across
thousands of rows, and on the measured integrated GPU it ran slower than the
CPU's tiles (12.6 s against 5.6 s at 8000²).

#### Scenario: Zoom matches the oracle on the GPU

- **WHEN** a Zoom Radial Blur is applied with GPU compute enabled on a machine with a usable adapter
- **THEN** the backend is the GPU and the result matches the oracle within ±1 LSB

#### Scenario: Spin stays on the CPU

- **WHEN** a Spin Radial Blur is applied with GPU compute enabled
- **THEN** the backend is the CPU and the result is byte-identical to the oracle
