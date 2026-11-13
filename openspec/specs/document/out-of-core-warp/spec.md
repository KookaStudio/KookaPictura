# document/out-of-core-warp Specification

## Purpose
A Free Transform warp commit rasterizes into spillable tiles a strip at a time, reading only the source windows its samples reach, and its gate counts the output strips.

## Requirements

### Requirement: A warp commit rasterizes into spillable tiles

The Free Transform warp commit SHALL rasterize the layer's channels, its layer
mask and every retained 16/32-bit plane one strip of 256 destination rows at a
time, into tiled planes. Each block of destination pixels SHALL read only the
source window that its samples' kernel taps and shrink footprints reach. Each
block SHALL split until that window fits the per-thread window budget. Tiles
whose samples are all equal SHALL hold no allocation. The output SHALL be
byte-identical to rasterizing the whole layer at once.

#### Scenario: Tiled warp matches the dense rasterizer [ocw_warp_exact]

- **WHEN** a seeded 8-bit layer with a mask is warped through an arc, a twist, a folding custom mesh and a strongly shrinking custom mesh, with each interpolation
- **THEN** every channel and the mask are tiled, and they equal the dense rasterizer's output byte for byte

#### Scenario: Retained deep planes warp through tiles [ocw_warp_deep]

- **WHEN** a 16-bit layer and a 32-bit layer, each with a retained store and a mask, are warped through an arc
- **THEN** every retained plane is tiled and equals the dense rasterizer's output byte for byte

#### Scenario: Uniform tiles hold no allocation [ocw_uniform_tiles]

- **WHEN** a 2048 × 2048 layer of one solid colour is warped through a flag warp
- **THEN** each channel's allocated bytes are less than a quarter of the dense plane's size

#### Scenario: The warped tiles can be spilled [ocw_output_spills]

- **WHEN** a 1024 × 1024 seeded layer is warped with a scratch volume configured, and eviction passes are run that spill every live tile
- **THEN** some of the layer's tiles are spilled, and reading the layer back equals the dense rasterizer's output

#### Scenario: A cancelled warp changes nothing [ocw_cancel]

- **WHEN** a warp runs under a job whose cancel flag is already set
- **THEN** it reports no change, and the layer's rect and pixels are unchanged

#### Scenario: A large warp commits under a hard memory limit [ocw_memory_max]

- **WHEN** memory scenario M14 runs at 17000 × 17000 under `MemoryMax=4G`
- **THEN** the warp commits without an OOM kill or a refusal

### Requirement: A warp commit's gate counts its strips

A warp commit's working set SHALL include its output strips: five planes of
32-bit samples, 256 rows across the result's width. It SHALL otherwise be gated
as the other transform commits are.

#### Scenario: A large warp spills instead of refusing [ocw_gate_spills]

- **WHEN** the commit cost of warping a 17000 × 17000 full-canvas layer is gated with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** its working set includes the strip bytes, and the commit is admitted
