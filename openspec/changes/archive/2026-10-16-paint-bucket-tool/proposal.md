# Proposal: paint-bucket-tool

## Why

The Paint Bucket (issue #25) was catalogued but disabled. photorust's
`core/src/bucket.rs` fills the Magic Wand's flood with the foreground colour;
the port follows `docs/03-tools/gradient-and-paint-bucket.md` (TOOL-024).

## What Changes

- `pictura_paint::bucket`: `flood` (the pixels matching a seed within Tolerance
  on every channel, alpha included, 4-connected when Contiguous) and `fill`
  (the foreground colour or a pattern tiled from the document origin, through
  the flood mask at Opacity in a Brush mode), over the shared `fill` pass of
  the `gradient-tool` change.
- `cxxqt_object/paint_tools/fills.rs`: `bucket_fill_at` floods the active
  layer or, with All Layers, the composite, softens the edge with
  `eraser::antialias_mask`, fills the active layer through the selection, and
  records one `"Paint Bucket"` state.
- `tool_fills.cpp` click handler; the bar (`options_bar_fill.cpp`) has Fill
  (Foreground / Pattern) with the built-in pattern picker, Mode (Normal /
  Dissolve / Behind / Clear), Opacity, Tolerance 32, Anti-alias, Contiguous,
  and All Layers. Catalog row enabled; the G group is complete.
- Qt Test `tst_fill_tools::paintBucketTool` (first the self-test `paint_bucket_tool`, code 553, now retired).

## Capabilities

### New Capabilities

- `tools/paint-bucket-tool`: the Paint Bucket.

## Impact

- `pictura-paint` (`bucket.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/bucket.rs` and `core/src/wand.rs`
(<https://github.com/perfecto25/photorust>), extended with the pattern fill.
Behavioural parity only. Unlike the Magic Wand, the flood counts alpha, so
empty pixels are not black. Ceilings (`ponytail:`): 8-bit RGB only; the fill
stays within the layer's rectangle.
