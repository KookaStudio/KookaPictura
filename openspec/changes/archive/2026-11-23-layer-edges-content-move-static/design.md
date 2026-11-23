# Design: layer-edges-content-move-static

## Context

`ImageView::beginMovePreview` takes `movesLayer`, default false. The Move tool's
whole-layer preview passes true; the selection-content preview leaves the
default. `layerEdgesRect` and `targetPathCurve` only apply the preview delta when
the flag is true.

## Decision

The test creates a shape layer so both overlays have geometry, starts
`beginMovePreview` without the `movesLayer` argument, sets a preview delta, and
asserts both overlays keep their committed rectangles. This exercises the same
default the content-move caller uses without driving the whole selection-move
gesture.

## Alternatives considered

- Drive an Alt-drag content move through the Move tool: exercises more code but
  also duplicates the selection-move fixtures; the overlay contract is the
  `movesLayer` gate, which the direct preview pins.

## Risks

- None; test-only change.
