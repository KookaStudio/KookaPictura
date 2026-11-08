## ADDED Requirements

### Requirement: Square planar tiles

A tiled `Plane` SHALL store each channel as square tiles of 256 × 256 samples,
with the tiles on the right and bottom edges clipped to the channel. For a
plane that stacks several channels, the tile grid SHALL restart at each
channel, so that no tile holds samples from two channels. Each tile SHALL be in
exactly one of these states:
- `Empty`: it reads as the plane's default value and holds no allocation;
- `Uniform(v)`: every sample reads as `v`, and it holds no allocation;
- `Dense`: a shared allocation, copied on its first write.

16-bit and 32-bit planes SHALL use the same tile size in samples. A plane that
was never tiled SHALL keep one contiguous allocation.

Every read of a tiled plane (sample, row copy, rectangle copy, range copy, tile
visit, and the counted contiguous view) SHALL return the same samples as a
`Vec` that received the same writes.

#### Scenario: Random edits match a Vec reference [tp_oracle]

- **WHEN** 10 000 seeded random operations (rectangle and row writes, reads,
  clones followed by writes, settles, and re-tiling) run against a tiled plane
  and a `Vec` at sizes including 1 × 1, 255 × 257, 256 × 256, 300 × 700 and a
  three-channel stack, for 8-bit, 16-bit and 32-bit samples
- **THEN** every read of the plane equals the `Vec`

#### Scenario: Tiles restart at each channel [tp_channels]

- **WHEN** a plane stacking three 300 × 300 channels is tiled
- **THEN** it has 2 × 2 tiles per channel, and a write to the last row of one
  channel forks no tile of the next channel

### Requirement: A write forks only the tiles it touches

A mutable access SHALL fork only the tiles it touches. Every other tile SHALL
stay shared with the plane's clones. Writing one sample SHALL allocate at most
one tile per written channel (64 KiB at 8-bit).

#### Scenario: One pixel forks one tile [tp_fork]

- **WHEN** a 1024 × 1024 8-bit plane full of varied bytes is cloned and one
  sample of the clone is written
- **THEN** exactly one tile of the clone is no longer shared, its new
  allocation is 65 536 bytes, and the original keeps its bytes

### Requirement: Uniform tiles collapse when a write settles

When a write settles, every tile it wrote whose samples are all equal SHALL
become `Uniform`, or `Empty` when that value equals the plane's default. The
check SHALL be made per channel. Collapsing SHALL NOT change any sample, the
plane's stamp, its ident, or any tile generation.

#### Scenario: Collapse is per channel [tp_collapse]

- **WHEN** a three-channel plane is filled so that channel 0 is all 7, channel
  1 is all 7 except one sample, and channel 2 is all 0, and the write settles
- **THEN** channel 0's tiles are `Uniform(7)`, the tile holding the odd
  sample of channel 1 stays `Dense`, and channel 2's tiles are `Empty`

#### Scenario: Default-valued tiles are empty [tp_empty]

- **WHEN** a 17000 × 17000 plane is built filled with 255, one is built filled
  with 0, and a flat plane of zeros is tiled
- **THEN** none holds a tile allocation, the zero-valued tiles are `Empty`,
  and every sample reads back as its value

### Requirement: Plane and tile identity

A plane's `stamp` and `ident` SHALL keep their guarantees over a tiled plane:
two planes with the same nonzero stamp or ident hold the same bytes, a clone
keeps both, and any mutable access clears both before a sample can change.
Each tile SHALL carry a generation: a clone keeps it, a mutable access to that
tile replaces it, and two tiles at the same index with the same generation
hold the same samples.

#### Scenario: Tile writes clear the plane's identity [tp_identity]

- **WHEN** a stamped tiled plane is cloned and the clone is written through
  `tiles_mut`, `write_rect_from` or `write_row_from`
- **THEN** the clone's stamp and ident are cleared, only the written tiles have
  new generations, and the original keeps its stamp, ident and generations

#### Scenario: Settling keeps generations [tp_generation]

- **WHEN** a written tile collapses to `Uniform` on settle
- **THEN** its generation, the stamp and the ident are unchanged

### Requirement: Documents store pixels in tiles

A new document's Background SHALL be built from `Uniform` tiles. Growing an
empty layer, a stroke's scratch coverage, and the history's captured planes
SHALL reuse tiles: an untouched tile SHALL be the same allocation in the live
document and in its history states. The memory gate's fork estimate SHALL count
the tiles a write touches, times the plane's sample size.

#### Scenario: A white Background costs almost nothing [tp_background]

- **WHEN** a 17000 × 17000 RGB document with a white Background is created
- **THEN** the Background's planes hold at most 4 MiB

#### Scenario: A new layer costs almost nothing [tp_new_layer]

- **WHEN** a layer is added to a 17000 × 17000 document
- **THEN** the new layer's planes and the history state that records it hold
  at most 1 MiB

#### Scenario: A fill costs the tiles it covers [tp_fill]

- **WHEN** a 1000 × 1000 square is filled on a layer of a 17000 × 17000
  document, through Edit › Fill or the Paint Bucket in a selection
- **THEN** the memory the fill adds is at most 1.2 × the square's area rounded
  out to the 256 grid, times the channels written

#### Scenario: A stroke costs the tiles it covers [tp_stroke]

- **WHEN** the same brush stroke is drawn on a 4250² and on a 17000² document
- **THEN** the memory it adds is the same within one tile per channel, and does
  not grow with the width of the document

#### Scenario: History shares untouched tiles [tp_history]

- **WHEN** one sample of a captured layer is written and the history captures
  the result
- **THEN** every other tile of the new state is the same allocation as the
  previous state's

#### Scenario: The memory gate counts tiles [tp_gate]

- **WHEN** the fork estimate is asked for a 10 × 10 write inside one tile of
  an 8-bit, a 16-bit and a 32-bit layer
- **THEN** it is one tile per channel, times 1, 2 and 4 bytes

#### Scenario: No scenario slows down [tp_time]

- **WHEN** the memory scenarios and the brush scenarios run before and after
  this change
- **THEN** no scenario's time grows by more than 10 %, and the brush dab
  latency by more than 5 %

### Requirement: PSD read and write stream through tiles

PSD write SHALL read a tiled plane row by row across a row of tiles, without a
contiguous copy of the plane, and SHALL produce byte-identical files. PSD read
SHALL decode channel rows into tiles.

#### Scenario: Writing a tiled document is byte-identical [tp_psd_write]

- **WHEN** a document is written to PSD once with flat planes and once with
  the same planes tiled and diverged from a clone
- **THEN** both files are byte-identical and the tiled write materializes no
  plane

#### Scenario: Reading fills tiles [tp_psd_read]

- **WHEN** a PSD with a white Background and a sparse layer is opened
- **THEN** the layer planes are tiled, the uniform regions are `Uniform` or
  `Empty`, and the round-trip suites are unchanged
