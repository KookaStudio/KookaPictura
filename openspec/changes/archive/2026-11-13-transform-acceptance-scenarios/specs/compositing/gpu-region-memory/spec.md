## ADDED Requirements

### Requirement: A GPU region composite fits the memory room or declines

Once the device limits are checked (an oversized region stays `TooLarge`) and
before any region-sized buffer exists, a GPU region composite SHALL reserve with
the memory governor the host bytes it may hold at once: 16 bytes per region
pixel (canvas, planar readback, its staging buffer and the output), plus 5
bytes per region pixel (a 4-plane source and a mask) for each layer and each
effect pass, plus 5 more for each isolated or clipping group's inner canvas.
When they do not fit the room, the composite SHALL decline with the `memory`
reason, without evicting tiles and without recording a memory refusal, so the
CPU composites the region in bands. The reservation SHALL be held until the
readback has been copied out.

#### Scenario: The estimate counts every upload [gpu_region_estimate]

- **WHEN** the estimate is taken for an 8 × 8 region of two plain layers, and
  for a group holding two layers
- **THEN** the first is 64 × (16 + 2 × 5) bytes, and the group's per-pixel
  bytes count both children

#### Scenario: A region over the room falls back to the CPU [gpu_region_memory]

- **WHEN** a two-layer 8 × 8 region is composited on a GPU device while the
  governor reports no available memory
- **THEN** it declines with `memory` before any region-sized buffer, and no
  refusal is recorded

#### Scenario: A reservation is held or declined without eviction [gpu_region_reserve]

- **WHEN** an accelerator reserves bytes that fit the room, and later bytes
  that do not
- **THEN** the first reservation is held until dropped, and the second is
  declined with no refusal recorded
