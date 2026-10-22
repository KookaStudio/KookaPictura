# Spec Delta

## MODIFIED Requirements

### Requirement: Default-on GPU filters

Filter application SHALL use the GPU by default when the M26 `gpuCompute`
preference is enabled and a usable adapter exists, consistent with the default-on
compositing requirement. When `gpuCompute` is disabled the system MUST force the
CPU implementation for every filter. A layer filter SHALL run on the CPU, even
with the GPU enabled, for the filters whose row-parallel CPU path outruns the
GPU round trip; which filters those are is decided by the filter type's own
fact (see `imaging/filter-engine-facts`) and is not enumerated by this
requirement. Its result is then
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
