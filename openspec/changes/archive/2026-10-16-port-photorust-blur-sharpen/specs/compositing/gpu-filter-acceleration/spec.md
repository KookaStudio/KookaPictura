## MODIFIED Requirements

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
