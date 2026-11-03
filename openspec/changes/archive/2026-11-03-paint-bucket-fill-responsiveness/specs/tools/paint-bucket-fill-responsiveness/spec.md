## ADDED Requirements

### Requirement: A uniform surface floods whole

The Paint Bucket's flood SHALL test each pixel against the seed's colour once.
When every pixel of the surface matches, the flood SHALL cover the whole surface,
contiguous or not, without walking it. A contiguous flood MUST still cover only
the matching pixels 4-connected to the seed.

#### Scenario: An empty layer and an enclosed seed [pbf_uniform_flood]

- **WHEN** an empty layer is flooded, contiguous and not, and a layer with an
  opaque ring is flooded from inside the ring at tolerance 0
- **THEN** the empty layer floods whole both ways, the contiguous flood covers
  only the ring's inside, and the global flood covers every pixel but the ring

### Requirement: The anti-alias box is unchanged

The anti-aliased coverage of a flood SHALL equal the per-pixel 3×3 box average,
floored at 128 for covered pixels and 0 elsewhere, for any mask and width,
leaving bytes past the last whole row untouched.

#### Scenario: Parallel and per-pixel boxes agree [pbf_antialias_exact]

- **WHEN** masks of solid runs, gaps and partial coverage, one to many rows and
  columns wide, are anti-aliased
- **THEN** each output equals the per-pixel box's

### Requirement: A fill blends each pixel as if alone

A fill SHALL lay down, at each pixel, the brush-mode blend of that pixel's
backdrop, colour and strength, whether or not the blend is reused from the
previous pixel of the run.

#### Scenario: Runs of equal pixels [pbf_blend_reuse]

- **WHEN** a layer with runs of equal backdrops is filled at strengths that
  change within the runs, in Normal, Multiply and Behind
- **THEN** every pixel equals the blend of that pixel computed alone

#### Scenario: A large document fills promptly [pbf_large_manual]

- **WHEN** on a real display, a 16507×16196 document gets a new layer and the
  Paint Bucket clicks it, then the fill is undone
- **THEN** the fill lands with PICTURA_PAINT_TIMING showing no flood or
  anti-alias phase dominating, and the undo restores the empty layer
