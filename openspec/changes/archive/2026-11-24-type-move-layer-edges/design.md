# Design: type-move-layer-edges

## Context

`refreshLayerEdges` computes the outline from `layer_content_bounds`, which is
the type layer's ink. The Move preview and damage, however, used
`damage::ink_influence`, whose canvas-fraction guard only trims a box covering at
least three quarters of the canvas. A paragraph box of tens of millions of pixels
around a few words is still large in absolute terms, so the move did work
proportional to the box. The outline cache also keyed on `(active path, canvas
revision)`; a deferred move can present before the revision moves, so the cache
returned the old rect.

## Decision

Add `damage::type_ink_influence`, which trims any type layer whose reach is the
raster alone, and use it in the Move reach, backdrop patch, and moving-image crop.
The alpha scan is bounded by the box, which the move would otherwise composite.
Keep `ink_influence` for type edits, where the canvas-fraction guard avoids
scanning a large box for a small edit.

Key the Layer Edges cache on the active layer's rect as well as the canvas
revision, so a rect move invalidates it. The rect key is cheap; the content
bounds scan stays behind the key.

A deferred commit keeps the Move preview while the document already holds the
moved layer. The outline refresh then lands at the old outline plus the drag
delta, which `layerEdgesRect` would offset again. `setLayerEdges` recognises
that exact rect and marks the preview's overlays committed, so the delta stops
applying; if no refresh arrives, the existing delta still places the outline.

## Alternatives considered

- Lower the `ink_influence` threshold for everything: changes type-edit damage on
  large boxes and its established tests.
- Refresh the outline on every `changed`: the content-bounds scan is the cost the
  cache exists to avoid.

## Risks

- The Move press now scans a large type layer's alpha once; the scan is smaller
  than the box composite the move used before.
- Effects, a clipped run, or a non-type layer keep their untrimmed rect.
