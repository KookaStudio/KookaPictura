# Design

## D1. Preview level

`transform_preview_level` picks the level `L` (document pixels per preview pixel
`2^L`, `L ≤ 16`): the smallest one at which the target box's area `/ 4^L` is at
most `TRANSFORM_PREVIEW_PIXELS` (16 Mpx). It is 0, the full-resolution path that
exists today, unless every one of these holds:

- one target, not a linked set, and no lifted selection (the backdrop is a
  patch over the live canvas);
- the layer's channels are its whole appearance (`needs_rendered_appearance` is
  false), it heads no clipping group, and it is not a smart object;
- `FilterProxy::reduces_faithfully(doc)` (the Move view preview's condition);
- the backdrop has a patch rect (`backdrop_patch_rect`).

These are the Move tool view preview's conditions (#1148), and its reduction
code does the work: `moving_layer_image` over the layer's rect aligned to the
level grid, and `view_preview::backdrop` over the aligned patch. A layer under
16 Mpx keeps today's exact path, so every existing preview test is unchanged.

ponytail: a reduced preview stays at its begin level when the view zooms in;
past 16 Mpx it is coarser than the screen at 100 %. Other targets (styled,
clipped, linked, lifted) keep full resolution and rely on the gate (D3).

## D2. Drawing a reduced preview

The level's scale travels like the Move view preview's, through its own
bridge call so a Move preview never reads it: `transform_preview_scale` reports
`2^L` while a transform session is live (from a new `transform_preview_level` on
the view state, reset with the preview), and `beginTransformPreview` takes it
into `moveScale_`, which the base patch already uses. The transform layer draw scales the image to document size: the affine
path draws into `size · scale`; the projective `QTransform` fallback scales the
painter before drawing; `warpTransformPreview` takes a `sourceScale` and samples
the image at `source / sourceScale`. The default source rect (a stored warp) is
`size · scale`. `transform_leave_warp` reads the moving image's document size
the same way.

The Warp preview (`refresh_warp_preview`) warps a clone at full resolution, as
today, and then builds its moving image at the session's level, so one scale
holds for base and layer.

## D3. Begin gate

`transform_preview_bytes` charges 20 bytes a preview pixel of the target box at
the chosen level (the layer image and its sRGB copy; the backdrop composite,
sRGB copy, and image: the measured 15–19 rounded up), plus 12 bytes a canvas
pixel when the backdrop or a linked set's image is the whole document.
`compute_transform_preview` asks `memory_gate::admit_fresh` before building
anything; a refusal returns false, which already restores the begin. The C++
begin reports `transformMemoryRefusalMessage()` when the refusal was for memory.

## D4. Premultiplied once

`ImageView::refreshTransformWarp` keeps a premultiplied copy of `moveLayer_`
keyed by its `cacheKey()`; `warpTransformPreview`'s own conversion is then a
shallow copy.

## D5. Mode switch

`can_set_mode` admits a projective target mode from any mode unless a warp is
being edited or stored, or the session holds editable type and the mode is
Distort or Perspective (the begin refusal). Leaving a projective mode is still
refused. `switch_mode` turns a similarity session into a quad session: the quad
is `session_quad` (the similarity over the source rect or the perspective base),
`start_quad` the same; scale, angle, skew and translation reset to identity; the
reference point becomes its document position (`pivot_point`), and
`pivot_origin` is carried by the old translation so the relative X/Y base stays.
The menu handlers for Skew, Distort, and Perspective call `setTransformMode`
while a modal session is open, as Scale and Rotate do, and their enabled state
follows `transform_can_set_mode` then.
