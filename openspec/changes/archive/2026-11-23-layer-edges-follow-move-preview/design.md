# Design: layer-edges-follow-move-preview

## Context

`ImageView` already offsets the Target Path curve during a Move preview, but the
Layer Edges box is painted from the stored document rect. Both previews share
`beginMovePreview`, including the selection-content move, which must not move
either document overlay.

## Decision

`beginMovePreview` takes a `movesLayer` flag. The Move tool's layer preview passes
true; the content-move preview keeps the default false. `layerEdgesRect` and
`targetPathCurve` apply the rounded preview delta only when the flag is set, so a
layer move carries both overlays and a content move carries neither.

## Alternatives considered

- Hide Layer Edges during every Move preview: avoids the stale box but also drops
  a useful box the user is positioning.
- Detect the move kind from the selection at paint time: the canvas has no
  selection context; the preview caller already knows.

## Risks

- A content move now leaves the Target Path in place too; previously the path
  followed any preview, which was wrong for a content move.
- The flag resets with `endMovePreview`.
