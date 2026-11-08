# tools/tiled-flood Specification

## Purpose
The Paint Bucket, the Magic Wand, Grow and the Magic Eraser flood over the 256² tiles of the document grid, pulling source tiles as they reach them and keeping one visited bit per pixel only for the tiles they reach, so a flood costs the tiles it touches; a tile of one matching colour is reached and filled whole as a `Uniform` tile, and Fill, Stroke, Gradient and the bucket composite tile by tile through the mask.

## Requirements

### Requirement: The flood pulls source tiles on demand

The flood behind the Paint Bucket, the Magic Wand, Grow and the Magic Eraser
SHALL be a four-connected scanline flood that reads the source and the
selection wall per 256 × 256 tile of the document grid, as the flood reaches
each tile. It SHALL keep its visited state at one bit per pixel, allocated only
for the tiles it reaches. Its output SHALL be a `TiledMask` on the document grid
whose unreached tiles are `Empty`. It SHALL select exactly the pixels the dense
flood it replaces selects, for every tolerance, with Contiguous on and off,
with and without a selection.

#### Scenario: The tiled flood equals the dense flood [tf_parity]

- **WHEN** seeded random sources of several sizes, some not multiples of 256,
  are flooded at several tolerances, contiguous and global, with and without a
  selection wall
- **THEN** the tiled flood's mask equals the dense flood's mask, byte for byte

#### Scenario: Visited state covers only the reached tiles [tf_reached]

- **WHEN** a contiguous flood on a 2048 × 2048 source stops at a closed region
  inside one tile
- **THEN** visited state is allocated for that tile only, and every other tile
  of the output is `Empty`

### Requirement: Whole tiles of one matching value stay uniform

When the flood reaches a tile whose source samples are all one matching value
and whose selection coverage is uniform and non-zero, it SHALL mark the whole
tile reached without visiting its pixels, and the output tile SHALL be
`Uniform(255)`. A fill through such a tile, onto a target tile whose samples are
uniform, with a paint that does not vary by position and any blend mode except
Dissolve, SHALL write the target tile as `Uniform` without allocating it.

#### Scenario: A uniform source floods whole tiles [tf_uniform]

- **WHEN** a 4250 × 4250 uniform source is flooded from its centre
- **THEN** every output tile is `Uniform(255)`, and no output tile is dense

#### Scenario: A whole-canvas bucket on an empty layer costs no pixels [tf_empty_layer]

- **WHEN** the Paint Bucket fills an empty layer of a document with no
  selection, with anti-aliasing on
- **THEN** every pixel of the layer holds the foreground colour, every tile of
  each layer plane is `Uniform`, and the layer's planes hold no dense tile

### Requirement: Fills composite tile by tile through the mask

Fill, Stroke, Gradient and the Paint Bucket SHALL composite tile by tile
through a `TiledMask` coverage and selection. They SHALL read each tile's mask
rows as they visit it, and SHALL create no buffer the size of the fill region.
Their results SHALL be bit-identical to the dense compositing they replace.

#### Scenario: The bucket and fill suites are unchanged [tf_fill_parity]

- **WHEN** the existing Paint Bucket, fill and stroke unit tests run against
  the tiled masks
- **THEN** they pass unchanged, including the boxed fill matching the
  document mask and the windowed stroke matching the whole-document stroke

#### Scenario: A uniform tile is written as Uniform [tf_uniform_write]

- **WHEN** a solid foreground fills, at 60 % opacity in Normal mode, a layer
  whose tiles are uniform, through a selection whose interior tiles are
  `Uniform(255)`
- **THEN** the interior target tiles are `Uniform`, and every pixel equals the
  per-pixel blend

### Requirement: The selection tools use the tiled flood

The Magic Wand, Grow, Similar, Quick Selection and the Magic Eraser SHALL read
their source per tile and build their masks as `TiledMask`s. Each result SHALL
be byte-identical to the dense implementation it replaces.

#### Scenario: The wand, Grow and Similar match the dense versions [tf_wand]

- **WHEN** the Magic Wand (contiguous and global), Grow and Similar run on a
  seeded composite
- **THEN** each selection equals the dense reference's selection

#### Scenario: The Magic Eraser matches the dense version [tf_magic_eraser]

- **WHEN** the Magic Eraser erases a seeded layer, with anti-aliasing on and
  off
- **THEN** the layer's pixels equal the dense reference's pixels

### Requirement: Bucket and wand memory is bounded by the tiles they touch

The memory gate's estimates for the Paint Bucket and the Magic Wand SHALL
count the tiles the operation's box crosses. They SHALL NOT charge masks the
size of the region. The heap guards SHALL hold M4 and M5 at or below their
lowered baselines with no materialization. They SHALL also bound a Magic Wand
on the M12 fixture.

#### Scenario: The bucket heap guards hold [tf_heap]

- **WHEN** the M4 (bucket in a marquee) and M5 (bucket on a whole empty layer)
  heap guards run at 4250²
- **THEN** each peak stays within 1.1 × its lowered baseline, and nothing is
  materialized

#### Scenario: The wand's peak follows the tiles it reaches [tf_wand_heap]

- **WHEN** the Magic Wand floods one disc of the M12 fixture at 4250²
- **THEN** the step's heap peak stays within 1.1 × its baseline, far below
  one plane the size of the document

#### Scenario: The large bucket benchmarks [tf_large_manual]

- **WHEN** `scripts/profile-tools.py` runs M4 and M5 at 17000² with `--peak`,
  before and after the change
- **THEN** each peak is at most 64 MiB above the steady state, the layer after
  M5 costs at most 4 MiB, and each time is at most 1.1 × the baseline
