# stroke-tile-save delta

## ADDED Requirements

### Requirement: A stroke saves each tile with one read

A stroke SHALL save each 64 px tile of its target layer the first time a dab
reaches it, reading the tile's rectangle from the layer's plane in one request
and never materializing a tiled plane. A cancelled stroke SHALL restore every
pixel it changed, including the partial tiles at the layer's right and bottom
edges.

#### Scenario: Cancelling restores a tiled layer's partial edge tiles [sts_restore_tiled]

- **WHEN** a Brush stroke and a per-dab stroke each paint corner to corner across a tiled layer whose size is not a multiple of 64, without growing it, and are cancelled
- **THEN** the layer equals its state before each stroke
