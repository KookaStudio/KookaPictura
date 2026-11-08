# Proposal

## Why

Issue #1214 is step 6 of #1209 (bounded memory for large documents). Since
#1207, a fill is limited to the box of the selection. A fill with no selection
(scenario M5), or a Magic Wand over a large region, still builds several masks
the size of the region: a match mask, a flood mask, an anti-alias mask and an
RGBA copy of the source. The selections themselves are dense planes the size
of the document. #1213 gave planes 256² tiles that can be `Empty`, `Uniform`
or `Dense`. Masks and floods do not use those tiles yet.

## What Changes

- **Tiled masks.** `pictura_core::mask_ops::TiledMask` is the tiled
  `Plane<u8>` from #1213, laid out on the document grid. A selection is always
  stored this way.
  - A marquee's interior is `Uniform(255)` tiles.
  - Tiles a selection does not reach are `Empty`.
- **Tile-pulling flood.** A scanline flood, `mask_ops::flood_tiled`, pulls
  source tiles on demand.
  - Its visited state is 1 bit per pixel, allocated only for the tiles the
    flood reaches.
  - A tile whose source holds one matching value is reached whole and comes
    out `Uniform`.
- **Bucket, wand and related tools.** The Paint Bucket, Magic Wand, Grow,
  Similar, Magic Eraser and Quick Selection run on that flood and on tiled
  masks.
- **Fills.** Fill, Stroke, Gradient and the bucket composite tile by tile
  through the mask.
  - A target tile whose inputs are all uniform is written as a `Uniform` tile.
  - A whole-canvas bucket on an empty layer therefore costs tile metadata, not
    pixels.
- **Selection operations.** Add, subtract, intersect, inverse, feather,
  expand, contract, border, smooth, translate and bounds work on the tiles.
  The operations that read a neighbourhood work band by band, reading a margin
  as wide as their reach.
- **Bit-identical results.** Every operation gives the same bytes as the dense
  implementation it replaces.
- **The memory gate's estimate** for the bucket and the wand counts the tiles
  they touch.

## Capabilities

### New Capabilities

- `tools/tiled-flood`: the tile-pulling flood, uniform tiles, compositing tile
  by tile, and the memory bounds for the bucket and the wand.
- `tools/tiled-selection-masks`: selections stored as sparse tiles, and
  selection operations on those tiles.

### Modified Capabilities

None. The behaviour the existing specs describe is unchanged.

## Impact

- `pictura-core`: `mask_ops` (tiled flood, banded window operations, bounds),
  and a new tile setter on `Plane`.
- `pictura-select`: constructors, boolean operations, morphology, feather,
  wand, grow, similar, bounds, translate.
- `pictura-paint`: bucket, fill, native fill, gradient, stroke band, Magic
  Eraser.
- `pictura-app`: fills, wand, Quick Selection, Magic Eraser, memory gate,
  memory guards.
- No new dependency, no change to the document format, no change to any
  golden.
