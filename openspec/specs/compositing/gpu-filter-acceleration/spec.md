# gpu-filter-acceleration Specification

## Purpose
GPU filter application matching the CPU oracle, default-on, with deterministic CPU fallback for unsupported kernels.

## Requirements

### Requirement: GPU filter application entry point

The system SHALL provide a way to apply a `pictura_filters::Filter` to a buffer
through `pictura_render::gpu::apply_filter_active(filter, buf, gpu_enabled) ->
Result<Backend, FilterError>`. It SHALL run a wgpu compute pass and return
`Ok(Backend::Gpu)` when GPU compute is enabled, a usable adapter is available,
and a GPU kernel exists for the filter; otherwise it SHALL run
`pictura_filters::apply` on the CPU and return `Ok(Backend::Cpu)`. It MUST NOT
panic and MUST surface invalid filter parameters as `Err(FilterError)` from the
CPU oracle.

#### Scenario: GPU available and enabled

- **WHEN** `apply_filter_active` is called with `gpu_enabled` true on a machine
  with a usable adapter and a filter that has a GPU kernel
- **THEN** it returns `Ok(Backend::Gpu)` and the buffer was produced by the
  compute pass

#### Scenario: GPU disabled or unavailable

- **WHEN** `apply_filter_active` is called with `gpu_enabled` false, or no
  usable adapter exists
- **THEN** it returns `Ok(Backend::Cpu)` and the buffer was produced by
  `pictura_filters::apply`

#### Scenario: Invalid parameters surface an error

- **WHEN** `apply_filter_active` is called with filter parameters the CPU oracle
  rejects
- **THEN** it returns `Err(FilterError)` and does not panic

### Requirement: GPU filter results match the CPU oracle

The accelerated set SHALL include the deterministic high-cost kernels Surface
Blur, Median, Maximum, Minimum, Custom (5×5), and Oil Paint. Every accelerated
pointwise, separable-convolution, window, and neighbourhood/effect filter MUST
match the CPU oracle: the GPU result MUST match `pictura_filters::apply` within
±1 LSB per channel for representative inputs (gradient, flat, and single-colour
buffers), and MUST preserve the alpha channel bit-for-bit on 4-channel buffers.

#### Scenario: Newly accelerated window kernels match the oracle

- **WHEN** Surface Blur or Median from the accelerated set is applied on a usable
  adapter to a representative input
- **THEN** every colour channel differs from `pictura_filters::apply` by at most
  1 LSB and alpha is unchanged on 4-channel buffers

#### Scenario: Oil Paint matches the oracle

- **WHEN** Oil Paint is applied on a usable adapter to a representative input
- **THEN** every colour channel differs from `pictura_filters::apply` by at most
  1 LSB and alpha is unchanged on 4-channel buffers

#### Scenario: Maximum and Minimum morphology matches the oracle

- **WHEN** Maximum or Minimum is applied on a usable adapter to a representative
  input
- **THEN** every colour channel differs from `pictura_filters::apply` by at most
  1 LSB and alpha is unchanged on 4-channel buffers

#### Scenario: Custom 5×5 matches the oracle

- **WHEN** Custom is applied with a 5×5 kernel on a usable adapter to a
  representative input
- **THEN** every colour channel differs from `pictura_filters::apply` by at most
  1 LSB and alpha is unchanged on 4-channel buffers

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

### Requirement: Unsupported kernels fall back to CPU

A filter with no GPU kernel SHALL fall back to the CPU implementation and return
a result byte-identical to `pictura_filters::apply`, without an error or a
panic.

#### Scenario: Painterly filter falls back

- **WHEN** a filter with no GPU kernel (for example `ColoredPencil` or
  `OilPaint`) is applied while GPU compute is enabled and an adapter is
  available
- **THEN** the CPU implementation runs and the result is byte-identical to
  `pictura_filters::apply`, with no error and no panic

### Requirement: CPU fallback is deterministic

With GPU compute disabled or no adapter available, filter application SHALL be
byte-identical to `pictura_filters::apply` for every filter, including those with
a GPU kernel.

#### Scenario: Disabled GPU matches the oracle

- **WHEN** a supported (accelerated) filter and an unsupported (kernel-less)
  filter are each applied with GPU compute disabled
- **THEN** both results are byte-identical to `pictura_filters::apply`

### Requirement: Profile-driven accelerated-kernel selection

A filter SHALL be GPU-accelerated only when it is high-cost at 1024² (a profiling
baseline of at least ~150 ms recorded in
`crates/pictura-filters/tests/profile.rs`) AND a GPU kernel reaches ±1 LSB parity
with `pictura_filters::apply`. A kernel that cannot reach parity MUST stay on the
CPU with a byte-identical fallback, the tolerance MUST NOT be widened to admit it,
and every stochastic (seeded) filter — `Crystallize`, `Watercolor`,
`ConteCrayon`, `PaintDaubs`, `DryBrush`, `OceanRipple`, `Spatter`, `Sponge`,
`PaletteKnife`, `AddNoise`, and `ColoredPencil` — SHALL remain on the CPU with a
byte-identical fallback.

#### Scenario: Stochastic filter falls back byte-identically

- **WHEN** a stochastic (seeded) filter such as `Crystallize` or `AddNoise` is
  applied while GPU compute is enabled and an adapter is available
- **THEN** the CPU implementation runs via `pictura_filters::apply` and the
  result is byte-identical to the oracle, with no error and no panic

#### Scenario: High-cost filter with a working kernel runs on the GPU

- **WHEN** a filter whose profiled 1024² baseline is at least ~150 ms and whose
  GPU kernel reaches ±1 LSB parity is applied with GPU compute enabled on a
  machine with a usable adapter
- **THEN** the reported backend is `Backend::Gpu` and the result matches the
  oracle within ±1 LSB

#### Scenario: Kernel without parity stays on the CPU

- **WHEN** a kernel is admitted on cost but cannot reach ±1 LSB parity with the
  CPU oracle on the parity corpus
- **THEN** the filter stays on the CPU, its fallback is byte-identical to
  `pictura_filters::apply`, and the tolerance is not widened to admit it

### Requirement: Heavy-kernel performance evidence

The GPU path for the accelerated heavy kernels SHALL be faster than the CPU
where it runs. Surface Blur's CPU path slides a histogram, O(r) per pixel,
where the GPU scans the window, O(r²), so the GPU plan SHALL cover Surface Blur
only up to radius 16 and wider radii SHALL run on the CPU. At radius 4, a
1024×1024 RGB Surface Blur SHALL complete faster on the GPU than on the CPU.

#### Scenario: Surface Blur speedup is recorded

- **WHEN** Surface Blur at radius 4 is timed at 1024×1024 on the GPU and on the
  CPU with `PICTURA_GPU_BENCH=1`
- **THEN** the GPU completes the filter faster than the CPU

#### Scenario: Wide Surface Blur runs on the CPU

- **WHEN** Surface Blur with a radius above 16 is applied with GPU compute
  enabled
- **THEN** the reported backend is `Backend::Cpu` and the result is
  byte-identical to `pictura_filters::apply`

### Requirement: Large-buffer GPU filter dispatch

The GPU filter path SHALL dispatch its compute shader as a two-dimensional grid
(an equivalent tiling) so that a filter applied to a buffer whose pixel count
exceeds the one-dimensional workgroup limit (65535 × 64 ≈ 4.19 MP) runs on the
GPU. It SHALL preserve byte-identical CPU fallback semantics and the existing
±1 LSB parity with `pictura_filters::apply` on the large buffer.

#### Scenario: A filter above the one-dimensional limit runs on the GPU

- **WHEN** an accelerated filter such as Surface Blur is applied to a buffer
  whose pixel count exceeds ~4.19 MP with GPU compute enabled on a machine with a
  usable adapter
- **THEN** the reported backend is `Backend::Gpu` and every colour channel
  differs from `pictura_filters::apply` by at most 1 LSB, with alpha unchanged on
  4-channel buffers

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
