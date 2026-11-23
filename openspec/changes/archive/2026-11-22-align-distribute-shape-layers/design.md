# Design: align-distribute-shape-layers

## Context

Align and Distribute compute a content box per selected layer. Pixel layers use
the box round their non-transparent pixels; groups and adjustment layers are
skipped. A shape layer is a fill-content layer cut by a vector mask, so
`Layer::has_adjustment_content` reports true and the current code skips it even
though it has visible content. Its `rect` is the whole canvas because the fill
has no pixels of its own, so the pixel rule cannot stand in.

## Decision: shape content box is the outline bounds

A shape layer's content box is `shape_layer_rect(layer, doc.width, doc.height)`:
the vector outline bounds rounded out to whole pixels. This is the same box the
Move tool and Free Transform hug, so Align, Distribute, the options bar, and the
transform controls agree on where the shape is.

## Decision: moving a shape translates its outline

`shift` detects a shape layer and calls `map_shape_layer` with a pure
translation. That keeps the live-shape definition when the shape is live, moves
the Bézier anchors and handles, and leaves the canvas-sized fill `rect` alone.
A position-locked shape refuses the move, as before.

## Alternatives considered

- Rasterize the shape to pixels before aligning: rejected because it destroys
  the vector shape and its live-shape data.
- Offset the layer `rect`: rejected because the fill layer's rect is the canvas,
  not the shape, so the shape would not move.

## Risks

- `layer_content_bounds` is also the transform bridge's "ink box as Align
  computes it"; making it shape-aware is intended and makes `contentRect`
  report the outline for shape layers.
- Non-shape layers keep the existing content and movement rules.
