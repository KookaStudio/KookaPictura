# Design: rasterize-type-on-canvas

## Rasterize what the compositor shows

Every set or re-set type layer already holds an exact render in its `0/1/2/-1`
planes. `replace_type_layer` → `render_type` produces it through the type's
matrix, warp or path, at the transformed rect. The compositor reads those planes
and applies the text gamma curve to `coverage × opacity × Fill` because the
layer is type.

Rasterizing therefore does not re-render. `render_text_layer` checks for the
four planes at the rect's area. When they are present, it maps the alpha
through the same `curve(a·k)/k` table `materialize_text_rgba` used and drops the
type state. The colour planes and the rect are untouched, so the raster
composites as the type did (within rounding).

A type layer without planes is composited by `composite_type_source`, which
paints `render_text_buffer` over the layer rect. Rasterize keeps that path, so
it still matches the canvas.

ponytail: proxy-less type (only synthetic documents; Photoshop writes pixels
for every type layer) is drawn and rasterized at the identity matrix, as before.

## Dropping type state

The raster loses `TySh`, the type tool, the applied style names and overrides,
and the transparency and pixel locks `add_type_layer` sets. CS6 does not keep
those locks on rasterized type. Other channels, such as a mask, are kept.

## Font picker

`isColorFont(family)` loads the face once through `QRawFont` and tests for a
colour glyph table. It covers emoji faces (`CBDT`/`COLR`/`sbix`) and Latin
colour fonts (`COLR`, `SVG `) without a coverage heuristic. The delegate caches
the answer per family as before.

Rows follow CS6's font menu: the name in the UI font, then `Sample` in the
face. Drawing the name in its own face made thin and dotted weights unreadable,
and sizing each row from its face loaded every font just to measure it.

ponytail: one sample size; `Type > Font Preview Size` is not built.
