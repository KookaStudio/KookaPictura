# Design: type-layer-move-locks

## Context

`MoveToolHandler::onPress` starts `beginContentMove` whenever a selection exists.
`move_selection_content` refuses a type layer because type layers carry the pixel
lock, but only on release; the drag has already offset the selection contour. For
a whole-layer move, `compute_move_preview`, `backdrop_patch_rect`, and
`move_reach` use `content_rect`, which returns a type layer's whole paragraph box
(`trim_bounds` never trims type). A paragraph box can be document-sized around a
few words.

## Decision

A new `PictureView::selection_content_movable(duplicate)` query mirrors the
engine's content-move refusals: no mask target, and the active pixel layer must
be visible, not type or shape, and not position-, pixel-, or (for a cut)
transparency-locked. `MoveToolHandler::onPress` starts the content move only when
it is true; otherwise it falls through to the existing whole-layer move. That
path already filters position-locked layers, so a locked target starts no
preview.

For a type layer, `move_content_rect` returns `damage::ink_influence` clipped to
the layer rect, and `move_reach` and `backdrop_patch_rect` do the same. The
helper also returns the crop offset from the layer rect. `begin_move_preview`
recomputes that offset on a cache hit after a commit instead of resetting it to
zero, because a type layer is never trimmed to its content.

`ink_influence` only trims a type layer whose reach is the raster alone and whose
rect covers at least three quarters of the canvas, so ordinary type boxes keep
their current bounds.

## Alternatives considered

- Refuse the press outright when a selection covers a type layer: spec-legal but
  contradicts the issue's expected "moving a type layer moves it"; moving the
  whole layer is what the user asked for.
- Trim every type move to ink unconditionally: changes the box for small point
  type where the box is already the text, and needs a wider regression surface.

## Risks

- A type layer with a non-Normal blend, effects, or a clipped run keeps its whole
  rect because `ink_influence` refuses to trim when the reach is not the raster
  alone.
- The content-move query duplicates the engine's lock checks; a future lock flag
  must be added in both places.
