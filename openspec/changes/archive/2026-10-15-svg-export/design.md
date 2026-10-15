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

The tree walk (#477) replaced the top-level index list: the caller supplies a
tree of `SvgNode { content, opacity, blend, clip }`, where `content` adds
`Group { children, isolated }`. `pictura_render::svg_nodes` builds it and takes a
`raster` closure that composites a scratch document (the app passes the active
backend; tests pass the CPU compositor or a fake). See D10.

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
| pixel / smart object / effect / shape with effects | raster `<image>` |
| group | nested `<g>` (D10) |
| adjustment / non-CSS blend / clip stack | flattened raster (D10) |
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

Decided on #477: a type layer is emitted as its raster fallback. Live `<text>` requires
a render-time font match (CC itself outlines glyphs), and outlining needs a
glyph source (Qt `QPainterPath::addText` at the boundary, or a Rust font crate).
The raster fallback keeps the output faithful without either.

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
- An oracle (`pictura-render/tests/svg_oracle.rs`): ImageMagick's `RSVG:`
  coder (librsvg) renders a vector-only document's SVG onto white and it is
  compared with the CPU composite on white within a tolerance. `magick` is
  already in the CI `oracles` job; the test self-skips when the coder is
  absent.
- Qt Test `tst_export_as`: the `SVG` format writes a well-formed, non-empty
  `.svg` for a raster document and a vector document.

## D9. Provenance

No Adobe source. Header and raster shape from the shipped CC sample; primitives,
blend map, and text handling inferred and `ponytail:`-marked. The renderer is an
oracle only.

## D10. Groups, clipping, and flattening (#477)

The walk handles one container's layers as *stacks*: a base plus the layers
clipped to it. Each stack is skipped (hidden or zero opacity), emitted as a
node, or needs the **backdrop** — it reads what lies beneath it:

- an adjustment layer (an adjustment block that is not a fill);
- a blend with no CSS form;
- a clipping stack that is not plain source-over (a clipped stack composited
  alone equals its in-place result only when base and members are Normal and the
  base is opaque);
- a faded or masked pass-through group (a `<g opacity>` would isolate it).

A backdrop stack replaces the container's nodes so far with one raster of the
container's layers up to and including it. That is exact for the document and
for an isolated group (which composites on transparency). A pass-through group's
backdrop lies outside it, so its walk fails and the parent flattens instead.

A raster fallback is composited at full opacity and Normal blend and its node
carries the layer's opacity and blend, so the `<g>` does not apply them twice
(v1 baked the opacity into the raster and wrapped it again). A non-pass-through
group with no opacity or blend carries `isolation:isolate`; a masked group is
one raster. A shape primitive scales its fill alpha by the layer's fill opacity
and is a raster when it has layer effects (a shape stroke is an effect) or a
pixel mask.
