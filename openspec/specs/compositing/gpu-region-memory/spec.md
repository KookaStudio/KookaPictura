# compositing/gpu-region-memory Specification

## Purpose
A GPU region composite is an accelerator: on a unified-memory GPU its
buffers are host memory, so it fits the memory governor's room or leaves the
region to the CPU's banded path, never pushing a large document out of memory
(#1290).

## Requirements

### Requirement: A GPU region composite fits the memory room or declines

Before a GPU region composite creates device work, it SHALL reserve the host
bytes it holds at once with the memory governor: 16 bytes per region pixel
(canvas, planar readback, its staging buffer and the output) plus 4 bytes per
region pixel for each layer's source upload. When they do not fit the room, the
composite SHALL decline with the `memory` reason, without evicting tiles and
without recording a memory refusal, so the CPU composites the region in bands.
The reservation SHALL be held until the readback has been copied out.

#### Scenario: A region over the room falls back to the CPU [gpu_region_memory]

- **WHEN** a two-layer 8 × 8 region is composited on the GPU path while the
  governor reports no available memory
- **THEN** it declines with `memory` before any device work, the estimate is
  64 × (16 + 4 × 2) bytes, and no refusal is recorded

#### Scenario: The room is held, not evicted for [gpu_region_reserve]

- **WHEN** an accelerator reserves bytes that fit the room, and later bytes
  that do not
- **THEN** the first reservation is held until dropped, and the second is
  declined with no refusal recorded
