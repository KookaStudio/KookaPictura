## ADDED Requirements

### Requirement: Per-pixel stacks composite in bands

The CPU compositor SHALL composite a region taller than two bands of a stack
whose every layer composites per pixel (no layer effects, no smart object, and
no type layer that renders its text and spans more than four bands) a band of
rows at a time, the bands in parallel, writing each band straight into the
output planes, so it never holds the region's whole floating-point canvas. The
result SHALL be byte-identical to compositing the region on one canvas, and the
canvas-to-buffer conversion SHALL fill its planes in parallel.

#### Scenario: Bands equal one canvas [bc_exact]

- **WHEN** a stack with offset and partial layers, Dissolve and other blend
  modes, a raster mask, an isolated group with a clipped child, and an
  adjustment layer is composited whole and over a sub-region
- **THEN** both are byte-identical to the single-canvas composite

#### Scenario: A short type layer renders in its bands [bc_type]

- **WHEN** the same stack carries a type layer without a rasterized proxy
- **THEN** the banded composite is byte-identical to the single-canvas one
