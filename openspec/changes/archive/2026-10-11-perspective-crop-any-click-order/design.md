# Perspective crop from any click order

## Context

`pictura_render::perspective_crop(doc, quad)` sends `quad[0..3]` to the output's
top-left, top-right, bottom-right, bottom-left. The tool must hand it that
canonical order; it currently passes the clicks in arrival order.

## Decision

On the fourth click, reorder the four points: sort them by angle about their
centroid (ascending is clockwise in the y-down screen frame), then rotate the
list so the top-left-most point (smallest `x + y`) comes first. The engine
contract is unchanged. A click that starts elsewhere, and a counter-clockwise
click, both land upright.

## Alternatives

- Normalize inside the engine: leaves the on-canvas grid axes swapped when the
  clicks are out of order, because the overlay reads the quad in its own order.
- Sort by x then y: cannot recover the winding, so a counter-clockwise click
  still mirrors.
