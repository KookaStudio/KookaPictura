# Spec Delta

## ADDED Requirements

### Requirement: Prepared effect mattes preserve scalar mask and blur values

Effect matte quantization, mask application, and emission SHALL preserve the
original scalar results exactly when performed through bulk buffers, resolved
mask gates, and independent rows. Density, feather, disabled masks, inverted
vector masks, default coverage outside raster bounds, and zero coverage SHALL
retain their existing behavior.

#### Scenario: Prepared matte processing matches the scalar oracle [pep_matte_exact]

- **WHEN** generated mattes are quantized and masked with combinations of raster
  and vector masks, density, feather, inversion, and disabled masks
- **THEN** every output sample equals the former scalar algorithm

### Requirement: Row effect replay preserves the original per-pixel composite

Effect passes replayed by independent row kernels SHALL preserve every component
of the original per-pixel composite bit for bit. This includes constant and
per-pixel effect color, blend modes, deterministic Dissolve, layer effect opacity,
clipped pass bounds, and knockout coverage tracking.

#### Scenario: Row replay matches the former scalar replay [pep_replay_exact]

- **WHEN** generated effect passes with constant and per-pixel colors are applied
  over transparent and partially opaque canvases with clipped bounds, varied
  effect opacity, blend modes, and coverage tracking enabled or disabled
- **THEN** every floating-point component and coverage bit equals scalar replay

### Requirement: Cache-local Gaussian tiles preserve filter bytes

A tiled Gaussian pass SHALL preserve the original clamp-to-edge samples, ordered
floating-point accumulation, and quantized bytes, including partial tiles and
kernel support larger than the image.

#### Scenario: Tile boundaries match scalar convolution [pep_gaussian_tiles]

- **WHEN** deterministic multi-plane images straddle horizontal and vertical
  tile boundaries and are blurred with short and oversized kernels
- **THEN** every output byte equals the original scalar convolution
