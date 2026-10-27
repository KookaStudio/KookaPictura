# Spec Delta

## MODIFIED Requirements

### Requirement: Per-pixel stacks composite in bands

The CPU compositor SHALL composite a region taller than two bands of a stack
whose every layer can be sampled per region, including cached smart-object
sources, smart-filter outputs, type rasters, and cached effect passes, a band of
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

#### Scenario: Cached object layers stay in bands [bc_objects]

- **WHEN** a tall document with a smart-filtered layer, a proxy-less type layer,
  and a styled layer is composited banded and whole
- **THEN** the two outputs are byte-identical
