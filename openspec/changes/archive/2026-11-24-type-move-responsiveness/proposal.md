# Proposal: type-move-responsiveness

## Why

Moving a type layer on a large document can stall for seconds. The Move press
and commit measured the whole paragraph box: `paint_bounds` reports the canvas
for any effects block (even an empty or disabled one), and a channel-less type
layer has no alpha to measure its ink. A paragraph box of tens of millions of
pixels around a few words then drives the preview, the dirty region, and the
deferred composite.

## What Changes

- A channel-less type layer is materialized once on the Move press, so later
  presses have the raster to measure.
- A type layer's Move reach is its ink plus the enabled effect pad, not its
  paragraph box or the canvas, whenever no knockout, clipping, or vector mask
  makes the reach depend on something else.
- The existing empty-block, large-box, and channel-less cases are covered by
  tests.

## Capabilities

### Modified Capabilities

- `tools/move-press-responsiveness`: a type layer's Move is sized by its ink
  (plus its effect pad), not its paragraph box.

## Impact

- `pictura-render` damage reach and a type materialization helper; the Move
  press in `pictura-app`. Rust tests only; no document-format change.

## Provenance

Behavior follows the existing Move and type-rendering contracts; no external
implementation is used.
