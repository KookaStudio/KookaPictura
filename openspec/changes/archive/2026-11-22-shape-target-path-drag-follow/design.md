# Design: shape-target-path-drag-follow

## Context

`ImageView::paintTargetPathOverlay` maps the stored target-path curve through the
view transform. During a Move drag the document is unchanged and the preview
lives in `ImageView` (`movePreviewActive_`, `moveDelta_`), so the curve keeps the
old document geometry.

## Decision

`refreshTargetPathOverlay` marks whether the target path is the active shape
layer's outline: the overlay comes from the layer (no Paths-panel row, or the
layer-target flag) and the active layer is a shape. `ImageView::targetPathCurve`
then translates the curve by the rounded Move preview delta while
`movePreviewActive_` is set. Painting uses that helper, so the curve follows the
shape preview.

## Alternatives considered

- Hide the Target Path during a Move drag: avoids the stale outline but drops
  useful geometry the user is positioning.
- Rewrite the stored overlay every drag step: the document outline is unchanged,
  so this would either need an inverse each step or a preview-only offset; the
  draw-time offset is simpler and resets with the preview.

## Risks

- A Work Path, saved path, or non-shape vector mask stays put because the flag
  requires the active layer to be a shape.
- The offset uses the rounded preview delta, matching the moving pixels rather
  than the fractional pointer.
