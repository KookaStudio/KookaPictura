# Design: svg-export

## Context

`File ▸ Export As…` (#457) is the CC dialog; its Format list stops at GIF and
`SVG` was explicitly deferred to #459. The dialog's encode path is
`QImageWriter` over a `QImage`, which cannot produce SVG text.

The vector sources already exist in the engine: `pictura_core::path::VectorPath`
(the Work Path), `Layer::vector_mask` (a decoded `vmsk` outline),
`pictura_codec::LiveShape` (a decoded `vogk` parametric shape), and
`Layer::type_tool` (a decoded `TySh` run with font, size, and fill colour).

The shipped CC sample (`horse-3svg.svg`) is a **raster** document and fixes the
header and raster-fallback shape precisely. No CC sample with vector content was
available, so the primitive emission is inferred.

## D1. Writer location and the dependency direction

The issue proposes `pictura-codec::svg::encode_svg(&Document, &SvgOptions)` that
rasterizes layers with `pictura-render::composite_*`. That is not buildable:
`pictura-render` already depends on `pictura-codec`, so a normal
`codec → render` dependency is a cycle (today `render` is only a codec
dev-dependency).

Resolution: keep the **pure serializer** in `pictura-codec::svg` (no
compositing, no fill decode, Qt-free, unit-testable) and let the caller supply
each top-level layer's [`SvgContent`]. The document walk lives in the codec, but
the *content* is decoded in `pictura-render::layer_content`, which owns the
`vscg` fill decode and the compositor, and already depends on the codec. The app
bridge calls `layer_content` per top-level layer and passes the list in.

`SvgContent` is one of `Shape { subpaths, fill, live }` (a shape layer's outline,
solid fill, and live form), `Raster { rgba, width, height }` (straight-alpha
RGBA8888 that the codec encodes to a base64 PNG `<image>`), or `None` (skip). The
codec writer never needs a compositor or a `vscg` decoder.

`ponytail:` the content list is indexed by top-level position; a future
nested-group walk would key it by index path instead.

### D1c. The raster fallback PNG is encoded in the engine

The fallback is a base64 PNG, and the shipped raster-export path cannot produce
it (it is Qt-boundary, and SVG is text). Rather than route PNG bytes back from
C++, `pictura-codec::png` gains a minimal Rust writer: 8-bit RGBA (colour type
6), one `IDAT` with filter 0 per row, no interlace or ancillary chunks, with a
hand-rolled CRC-32 and base64. `flate2` (already a codec dependency) supplies the
zlib stream, so no dependency is added. This keeps the whole SVG path in the
Qt-free engine; the shipped raster exporters are untouched.

### D1b. `vscg` shape decode is wired into the shape model

The `vscg` vector-fill decoder already shipped (`vector-fill-content`,
`pictura_render::decode_layer_fill`) and drives compositing, but the shape-layer
predicates ignored it: `is_shape_layer` required a modeled adjustment fill, and
`shape_fill_color` read only `layer.adjustment`. A CC-imported shape layer
(`vmsk` outline + `vscg` fill) was therefore not a shape layer to the app. This
change broadens both to `decode_layer_fill`, so an imported shape layer is
recognized, its solid fill colour is available, and `pictura_render::layer_content`
can emit it as a filled primitive. No decode is added or moved; the existing
decoder is surfaced (`decode_layer_fill`, `decode_vector_fill` become `pub`).

## D2. The CC header

From the sample, verbatim:

```
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" preserveAspectRatio="xMidYMid" width="1000" height="652" viewBox="0 0 1000 652">
  <image xlink:href="data:img/png;base64,…" width="1000" height="652"/>
</svg>
```

Two-space indent, a child at four, a trailing newline, no XML declaration and no
BOM. The `img/png` subtype is nonstandard but is what CC writes; parity wins.

## D3. Path serialization

`Subpath::segment(i)` yields `[p0, c1, c2, p3]` with missing handles defaulted to
their anchor. The serializer emits `M p0`, then per segment a `C c1 c2 p3`, and a
`Z` when `closed`. `ponytail:` a straight segment is written as `C` for
uniformity; CC may emit `L`, which is byte-different but pixel-identical, so the
oracle compares pixels, not bytes. Coordinates are rounded to three decimals and
trailing zeros are trimmed.

## D4. Primitives and fallback

| Layer content | Emitted |
|---|---|
| live rectangle (`vogk` type 1) | `<rect x y width height fill>` |
| live rounded rect (type 2) | `<rect … rx ry fill>` (uniform `rx` from the radii) |
| live ellipse (type 5) | `<ellipse cx cy rx ry fill>` |
| `vmsk` outline + solid fill | `<path d="…" fill-rule fill>` |
| type layer | `<text>` (see D6) or raster fallback |
| pixel / smart object / effect / adjustment | raster `<image>` |
| gradient / pattern shape | raster fallback |

The fill comes from `pictura_render::shape_fill_color`, which now reads the
`vscg` vector-fill block (D1b). `fill-opacity` carries the alpha when it is not
255. A gradient or pattern shape has no simple SVG `fill`, so it uses the raster
fallback until gradient/pattern `<defs>` support exists.

`ponytail:` the vector **stroke** (`vstk`) is still not decoded, so a shape's
outline stroke is baked into the raster fallback rather than emitted as
`stroke="…"`. The `<path>` fill rule is even-odd, matching the `vmsk` default.

## D5. Opacity and blend

A layer with `opacity < 255` or a CSS-representable blend is wrapped in a `<g>`.
The CSS map covers Normal (omitted), Multiply, Screen, Overlay, Darken, Lighten,
ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue,
Saturation, Color, Luminosity. Every other mode (`Dissolve`, `LinearBurn`,
`DarkerColor`, `LinearDodge`, `LighterColor`, `VividLight`, `LinearLight`,
`PinLight`, `HardMix`, `Subtract`, `Divide`, `PassThrough`) falls back to a
rasterized layer.

## D6. Text

`ponytail:` v1 emits a type layer as its raster fallback. Live `<text>` requires
a render-time font match (CC itself outlines glyphs), and outlining needs a
glyph source (Qt `QPainterPath::addText` at the boundary, or a Rust font crate).
Choosing that source is an open question on #459 and is deferred; the raster
fallback keeps the output faithful.

## D7. Export As integration

`export_as_dialog.{h,cpp}` gains a `SVG` format entry (`Format::Svg`, combo index
4, suffix `svg`); `formatForPath` and `retargetPath` recognize `.svg`, and
`refreshFormatControls` disables Transparency and Quality (SVG is neither lossy
nor alpha-gated).

SVG is **document-level text**, so it does not fit the dialog's per-entry
`QImage` model: `encodedAs` returns empty for SVG (the size estimate shows
dimensions only), and `exportAsFromView` branches before the entry flow. When
the chosen format is SVG it calls the `svg_from_view` bridge, prompts for one
`.svg` path (defaulting to `<document folder>/<name>.svg`), and writes the text
through `QSaveFile` (atomic). The checked rows do not select targets for SVG;
one document is one `.svg`. `ponytail:` per-layer SVG (one file per checked row)
is a later refinement.

## D8. Tests

- Rust `#[test]`: the `d` serializer over straight, curved, and closed subpaths;
  each primitive (`rect`, rounded `rect`, `ellipse`, `path`, `image`); XML
  escaping; the CC header byte-for-byte for a raster-only document; layer
  skipping; the opacity `<g>`.
- An oracle (self-skipping when absent): render the SVG with a system renderer
  (`rsvg-convert`, Inkscape, or ImageMagick's delegate) and compare pixels
  against the CPU composite for a vector-only document, within a tolerance.
- Qt Test `tst_export_as`: the `SVG` format writes a well-formed, non-empty
  `.svg` for a raster document and a vector document.

## D9. Provenance

No Adobe source. Header and raster shape from the shipped CC sample; primitives,
blend map, and text handling inferred and `ponytail:`-marked. The renderer is an
oracle only.
