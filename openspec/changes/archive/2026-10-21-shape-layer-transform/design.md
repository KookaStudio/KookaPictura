# Design

## The shape's box

A shape layer made by the shape tools is a channel-less fill layer whose `rect`
is the canvas; its outline lives in the `vmsk` block. Every move and transform
of a shape therefore uses `shape_layer_rect`: the outline's bounds rounded out
to whole pixels, with a small slack so the fixed-point `vmsk` coordinates do
not widen the box by one. The transform session's `orig_rect`, the Move tool's
controls box (`content_rect`), and the engine's map all use the same box, so
the similarity map's centre agrees between the preview and the commit.

## One point map

`map_shape_layer` sends every anchor and handle through a document-space map
and writes the outline back. The existing engine entry points branch to it
before the raster path, the way a type layer already branches to its own
re-set:

- `translate_layer_path` (the Move commit): a translation; a live shape is
  translated and stays live.
- `transform_layer_with`: the similarity `Map` about the shape's box; a pure
  translation keeps it live, anything else leaves a regular path.
- `transform_layer_quad_with`: the homography from the box's corners to the
  quad.
- `transform_layer_warp`: the warp followed by the frame's homography.

A position lock refuses all of them; the document is unchanged on refusal.
Mapping the control points is exact for an affine map. Under a projective map
or a warp it approximates the curve (marked `ponytail:`).

## App side

`can_free_transform` accepts a visible, unlocked shape layer with an outline of
some area. The Move tool's target (`move_cache_target`) and its Alt duplicate
accept a shape layer. Its moving image is the layer rendered alone, the same
image the transform preview already used. The cached move image is never
reused for a shape: a move shifts the outline without changing the layer's
rect, so the cache key would not see it.

## Layers panel

The row snapshot leaves `vector_mask_thumbnail` empty for a shape row. The
delegate already hides the vector slot, its link glyph, and its hit areas when
that image is null. `has_vector_mask` stays true, so the vector-mask commands
still see the mask.
