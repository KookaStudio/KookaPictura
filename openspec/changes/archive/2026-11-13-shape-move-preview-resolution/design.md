# Design

## D1. Shape content and moving region

A shape layer's own `rect` is the canvas, so it cannot bound the preview. The
shape's content is `damage::content_influence(doc, path)`: its outline box, its
effect reach, and its clipped run, clipped to the canvas. Both the backdrop
patch and the moving image's region use that reach instead of `layer.rect`.
`MoveView::content` keeps reporting the reach so the canvas boxes the same
content with marching ants as the full-resolution preview did.

`moving_region` aligns the reach to the level grid inside itself, exactly as it
does a pixel layer's content. `follow` intersects the shifted view with the
reach, so a drag that carries the shape off the moving image rebuilds it. Using
`layer.rect` there would always contain the view and never rebuild.

## D2. Reduced composite of the shape alone

`moving_layer_image` already renders a layer whose channels are not its whole
appearance over a region-sized scratch document. For a shape at level `L`, the
scratch document contains only the shape layer, shifted into the region; the
vector mask is reduced by `FilterProxy` as for any rendered appearance.

`FilterProxy` does not scale layer-effect sizes. Before the proxy is built, a
shape layer's effects are scaled by `1 / 2^L` through
`pictura_render::scale_layer_effects`. A 20 px stroke at level 2 then draws as
5 px on the preview image, which the canvas scales back to 20 document pixels.
The scale is approximate (the descriptor rounds integer pixel fields), which is
acceptable for a transient drag preview; the commit stays exact.

## D3. Backdrop fidelity

The backdrop is the document with the moving layer hidden. Above level 0 the
reduction must still be faithful for every layer that actually draws. A hidden
layer draws nothing, so `FilterProxy::reduces_faithfully` now skips invisible
layers (and their children, for an invisible group). For a shape press the
check runs on the document with the shape hidden; a styled layer elsewhere that
is visible still refuses the reduction, as before.

## D4. Release

The existing deferred Move commit already handles a dirty union at or above
16 MP. A shape's reach union is the same kind of bound, so no new release path
is introduced. A Qt regression commits a large shape and asserts the composite
is pending and lands with one `Move Layer` history state.

## D5. Limits

- One shape layer only; link sets and groups keep the full-resolution preview.
- A shape with an undecodable vector fill or a smart object is still refused by
  the existing plain-content checks.
- The moving image is built once at the press level, as the plain-layer view
  preview is; zooming past that level shows a coarser image until the next
  press.
