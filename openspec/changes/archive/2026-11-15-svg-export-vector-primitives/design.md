# Design: svg-export-vector-primitives

## Context

`pictura_codec::svg` is the pure serializer. It receives a tree of
`SvgNode { content, opacity, blend, clip }`; `SvgContent::Shape` currently
carries a solid `[u8; 4]` fill. `pictura_render::svg` builds that tree and owns
the `vscg` fill decode and the compositor.

The compositor already defines exactly how a gradient or pattern fill covers a
layer rectangle (`fill.rs`): gradients are generated over `layer.rect` with the
PSD angle/scale geometry, and patterns are tiled from the document `Patt` block
with the fill's scale, link, and origin. This change reuses that geometry
instead of inventing a second one.

## D1. Codec types

`SvgContent::Shape` becomes a struct variant carrying:

- `subpaths`, `live` (unchanged);
- `paint: SvgPaint` — `Solid([u8; 4])`, `Gradient(SvgGradient)`, or
  `Pattern(SvgPattern)`;
- `stroke: Option<SvgStroke>`.

`SvgGradient` is `Linear { x1, y1, x2, y2 }` or `Radial { cx, cy, rx, ry }` plus
`stops: Vec<SvgGradientStop { offset, rgba }>`, all in document user space.
`SvgPattern` carries the tile as straight-alpha RGBA8888, its pixel size, and
its user-space origin. `SvgStroke` carries the solid RGB, the visible width, the
opacity fraction, and the position (`Center`, `Inside`, `Outside`).

The writer gains a defs registry with document-unique ids for gradients,
patterns, and stroke masks, beside the existing clip ids.

## D2. Gradient geometry

`fill.rs` samples a gradient at local layer-rect pixel `(i, j)` through

```
x = bx + i * ax,  y = by + j * ay
bx = -w / s, by = -h / s
ax = 2w / (s (w - 1)), ay = 2h / (s (h - 1))
s  = (scale / 100) * ((90 - angle mod 90) / 90 * w + (angle mod 90) / 90 * h)
```

with `w, h` the layer-rect size, `angle` in degrees, and `scale` in percent.

**Linear.** The engine's position is

```
z = 0.5 (cos t * x - sin t * y + 1) = c + g · (i, j)
g = 0.5 (cos t * ax, -sin t * ay)
c = 0.5 (cos t * bx - sin t * by + 1)
```

SVG's linear gradient parameter is the projection onto `P2 - P1`. Setting
`P1 = -c g / |g|²` and `P2 = P1 + g / |g|²` makes that projection exactly `z`,
so stops keep their `Lctn / 4096` offsets. The points are translated by
`(rect.left, rect.top)` into document space. A zero `|g|` (a one-pixel axis)
is a constant gradient and emits a solid colour sampled at `c`.

**Radial.** The engine's position is

```
z = sqrt((ax (i - i0))² + (ay (j - j0))²)
i0 = -bx / ax, j0 = -by / ay
```

so it is an ellipse centered at `(i0, j0)` with radii `1 / |ax|` and `1 / |ay|`.
SVG emits a radial gradient at the origin with `r="1"` and
`gradientTransform="translate(cx cy) scale(rx ry)"`; the user-space ellipse then
has the engine's radii. A one-pixel axis is not representable this way and
falls back to the raster.

`Rvrs` reverses the stop list (offset `1 - Lctn / 4096`) before emission. The
engine's gradients are opaque; the shape's Fill opacity still becomes
`fill-opacity`.

## D3. Pattern geometry

The compositor tiles at `tw = round(pattern.width * scale / 100)` and
`th = round(pattern.height * scale / 100)`, at least one pixel, and samples the
source with `floor(tile_x / tw * width)`. The renderer pre-scales the tile with
that exact nearest-neighbour mapping and embeds the scaled tile as a PNG, so
the viewer needs no resampling.

The tile's user-space origin is

```
x = (link_with_layer ? layer.rect.left : 0) + origin.x
y = (link_with_layer ? layer.rect.top  : 0) + origin.y
```

which reproduces `(sample_base - origin) mod tile`. A missing pattern uses the
compositor's opaque 50 % grey placeholder as a one-pixel tile.

## D4. Strokes

`decode_layer_effects` already returns every typed effect. A layer is stroke
vectorizable only when the stroke is enabled and present, its fill is
`SClr`, its blend is Normal, and every other effect is inactive. Otherwise the
layer keeps its raster fallback, because the current `SvgContent::Shape` cannot
represent a second effect pass.

A center stroke is one element with both `fill` and `stroke`. An inside or
outside stroke is split into a fill element and a stroke-only element of double
width, masked to the shape's interior or exterior. That leaves exactly the
engine's one-sided band: a centered `2 * size` stroke has `size` on each side of
the centerline, and the mask discards the unwanted side.

The stroke uses `stroke-linejoin="miter"` and `stroke-linecap="butt"`, matching
the compositor's square max-filter dilation. Stroke opacity is
`Opct / 100`; the layer opacity and blend stay on the wrapping `<g>`.

## D5. Per-layer export

`svg_from_layer(view, index)` clones the document, replaces its layer stack with
the one top-level layer, and runs the same `encode_document_svg`. The canvas
size and `viewBox` stay the document size, so the layer keeps its absolute
position and the header contract is unchanged.

In `exportAsFromView`, the entries already correspond to the document row plus
each non-adjustment top-level layer. SVG now follows the raster flow:

- one checked row: prompt for one `.svg` and write it;
- several checked rows: prompt for a folder, then write one `.svg` per row with
  the existing duplicate-name disambiguation and `QSaveFile` atomicity.

The document row uses `svg_from_view`; a layer row uses `svg_from_layer` with
the row's top-level layer index.

## D6. CC confirmation (blocked)

The issue asks to confirm the `<path>` fill rule and the `mix-blend-mode` value
set against a CC sample. The repository still has no CC sample with vector
content (the raster-only `horse-3svg.svg` sample fixes only the header and
raster fallback). No public Adobe source is used. The shipped even-odd path rule
and the CSS blend map stay as inferred; this change adds no unverified parity
claim and records the gap in the docs.

## D7. Fallbacks and ceilings

- Angle, reflected, and diamond gradients; one-pixel gradient axes; gradient
  and pattern strokes; non-Normal stroke blends; and any shape with another
  active effect keep the existing raster fallback.
- Imported `vstk` vector strokes remain undecoded; a layer carrying only `vstk`
  is unchanged by this change.
- Pattern scale still uses nearest-neighbour sampling, as the compositor does;
  CS6's resample filter remains a `ponytail:` ceiling.
- Per-layer SVG uses the document canvas, not a cropped layer box. The other
  raster formats crop to layer bounds; SVG keeps the document viewBox so the
  vector coordinates stay absolute.
