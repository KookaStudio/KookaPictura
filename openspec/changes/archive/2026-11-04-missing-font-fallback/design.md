# Design

## Where the opaque fill comes from

`content_route` sends a type layer with a `TypeTool` and no colour channel to
`Body::TypeSource`. The compositor asks `layer_source::text_raster` for the live
render. When the raster is `None`, the body falls through to
`composite_pixels`; for a layer with no colour channel that reads every sample
as zero and alpha as 255, so it paints an opaque rect over the layer rect.

The raster is `None` when `render_text_buffer` finds no style, non-positive
size, or no painted glyph. `fonts::face_for` already resolves an unregistered
family to the bundled Liberation Sans, so a missing family alone is not enough.
The failure appears when a face *is* resolved — for example the host's
substitute registered under the missing family — but paints no glyph pixel.

## The retry

`render_text_buffer` keeps one paint path, `paint_text`, which lays out and
rasterizes with the given face. It tries the resolved face first; when that
paints nothing and the face is not the bundled face, it runs the same path with
`text_render::bundled()`. Re-layout is required because line breaks, advances,
and the box all come from the active face's metrics.

`paint_with_fallback(first, fallback, paint)` holds the two-step order so a unit
test can prove the retry without a font that fails to rasterize.

## The transparent no-ink layer

`Body::TypeSource` now composites its raster when it has one and paints nothing
when it does not. A proxy still goes through `composite_pixels`, as before.
`composite_type_source` returning `false` (an empty clipped region) also leaves
the backdrop alone.

This is the same shape as the existing smart-object guard, which already
returns instead of painting a channel-less rect.

## Ceiling

The retry triggers only when no glyph pixel paints. A substitute face that
paints `.notdef` boxes or the wrong glyphs still paints, so the text is not
replaced with the bundled face; detecting that would need coverage or glyph-id
validation against the requested family. Missing Glyph Protection's per-glyph
fallback is unchanged.

## Tests

- `missing_family_renders_with_the_bundled_face`: a proxy-less layer in an
  unregistered family composites text coverage, not a fill.
- `a_face_that_paints_nothing_retries_with_the_bundled_face`: the retry helper
  calls the bundled face after the resolved face returns `None`.
- `a_missing_family_keeps_its_stored_name`: compositing does not rewrite the
  layer's family.
- `proxy_less_type_that_paints_nothing_leaves_the_backdrop`: a blank-text type
  layer over a solid base leaves the base unchanged.
- The existing live-composite and stored-proxy tests cover the unchanged
  scenarios.
