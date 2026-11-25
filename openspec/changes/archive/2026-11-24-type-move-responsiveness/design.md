# Design: type-move-responsiveness

## Context

`damage::paint_bounds` returns the canvas for a layer with an `lfx2`/`lrFX`
block unless `transparency_shapes` confines it, so a type layer with an empty or
disabled effects block has a canvas-sized reach. A type layer without raster
channels has no alpha, so `ink_bounds` returns `None` and the reach falls back to
the paragraph box. The Move press, backdrop patch, moving crop, and commit
damage all use that reach.

## Decision

`damage::type_ink_influence` handles a type layer with no knockout and no vector
mask directly: the reach is `ink_bounds` expanded by
`layer_effects::effects_pad` (the enabled effect reach), clipped to the canvas.
A clipped member's base is stationary, so its own ink is the change; a base with
a clipped run above it keeps the conservative union.
A type layer without ink falls back to its rect. Effects that draw around the
ink are still covered; a knockout, a clipped run, or a vector mask keeps the
conservative `content_influence` path.

`pictura_render::materialize_type_layer` renders a channel-less type layer's
uniform run set into its channels once. `begin_move_preview` calls it for every
move path before measuring or building the preview, so a channel-less layer has
alpha for the ink trim on that and later presses. The pixels are the
compositor's own render, so the appearance does not change.

## Alternatives considered

- Change `paint_bounds` globally to use `effects_reach`: affects every edit's
  damage and its tests, and an undecodable effects block is safer as the canvas.
- Render the type at a reduced level to find the ink: the commit needs the
  full-resolution ink, and the reduced box would leave edge residue.

## Risks

- The first press on a channel-less type layer pays its raster render. Later
  presses use the channels.
- The effect pad is the decoded effects' reach; an unknown effect is ignored by
  the compositor too.
