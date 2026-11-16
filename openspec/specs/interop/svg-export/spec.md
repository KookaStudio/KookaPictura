# interop/svg-export Specification

## Purpose
`File ▸ Export As…` SVG output: a Qt-free engine writer that serializes a
document to Photoshop-CC-shaped SVG text, emitting vector layers as SVG
primitives and raster layers as an embedded base64 PNG fallback.

## Requirements

### Requirement: SVG export writes a CC-shaped document

The application SHALL offer `SVG` in the `File ▸ Export As…` Format list and
SHALL write the document as UTF-8 SVG text. The root element SHALL be `<svg>`
with `xmlns="http://www.w3.org/2000/svg"`,
`xmlns:xlink="http://www.w3.org/1999/xlink"`,
`preserveAspectRatio="xMidYMid"`, `width`, `height`, and
`viewBox="0 0 W H"` in that attribute order, where `W` and `H` are the document
pixel dimensions. The output SHALL use two-space indentation, a newline after
the opening tag, and a trailing newline, matching the shipped CC sample. SVG is
text, so the writer MUST NOT go through the Qt `QImageWriter` path; writes SHALL
be atomic (temp file plus rename), so a failure never truncates an existing file.

#### Scenario: A raster document matches the CC sample shape

- **WHEN** a document with no vector content is exported to `.svg`
- **THEN** the file is an `<svg>` whose header matches the sample attribute
  order and whose body is one `<image>` covering the document

#### Scenario: An unwritable SVG target fails without a partial file

- **WHEN** the export edge is given an unwritable path
- **THEN** it returns failure and no partial file is produced

### Requirement: Raster layers embed a base64 PNG image

For a layer whose content is pixels, type, a smart object, or a baked effect,
the writer SHALL rasterize that layer alone, at full opacity and Normal blend, to
a PNG and embed it as
`<image xlink:href="data:img/png;base64,…" width="W" height="H"/>`. The MIME
subtype SHALL be spelled `img/png` to match the CC sample. A raster document
SHALL therefore produce exactly one `<image>` with no `x` / `y` attributes.

#### Scenario: A pixel layer is embedded, not traced

- **WHEN** a document has one opaque pixel layer covering the canvas
- **THEN** the SVG body is a single `<image>` whose decoded PNG equals that
  layer's pixels

#### Scenario: Hidden and zero-opacity layers are skipped

- **WHEN** a layer is hidden or has zero opacity
- **THEN** the SVG contains no element for that layer

### Requirement: Vector layers emit SVG primitives

For a layer carrying a vector mask or a live shape, the writer SHALL emit an SVG
primitive rather than a raster fallback: a live rectangle as `<rect>`, a live
rounded rectangle as `<rect>` with `rx` / `ry`, a live ellipse as `<ellipse>`,
and any other outline as `<path d="…">`. A closed subpath SHALL end with `Z`;
each segment SHALL be a `C` cubic (a straight segment, whose handles sit on its
anchors, MAY be written as `L`). Coordinates SHALL be document pixels with
`f64` precision trimmed to a fixed number of decimals.

#### Scenario: A rectangle emits a rect

- **WHEN** a document has a live-shape rectangle at `(l, t, r, b)`
- **THEN** the SVG contains `<rect>` with `x=l y=t width=r-l height=b-t`

#### Scenario: A curved outline emits a path with cubics

- **WHEN** a layer's vector mask has a smooth closed subpath
- **THEN** the SVG contains a `<path>` whose `d` is `M` followed by `C`
  segments and a closing `Z`

### Requirement: Layer opacity and blend are represented

The writer SHALL wrap a layer that carries a non-default opacity or a
CSS-representable blend mode in a `<g>` element with `opacity` (a fraction of
`opacity/255`) and `mix-blend-mode` respectively. A raster fallback MUST NOT also
bake in the opacity or blend its `<g>` applies. A blend mode with no CSS
equivalent SHALL NOT emit a `mix-blend-mode`; the layer is flattened instead
(see *Content SVG cannot express is flattened*).

#### Scenario: Opacity becomes a group attribute

- **WHEN** a layer has opacity 128
- **THEN** its element is wrapped in `<g opacity="0.502">`

#### Scenario: Multiply maps to mix-blend-mode

- **WHEN** a layer has the Multiply blend mode
- **THEN** its group carries `mix-blend-mode:multiply`

### Requirement: Groups nest as SVG groups

The writer SHALL emit a layer group as a `<g>` containing its children's
elements, bottom-first, carrying the group's opacity and CSS blend. A
non-pass-through group SHALL be isolated (`isolation:isolate` when neither an
opacity nor a blend already isolates it); a pass-through group at full opacity
SHALL be a bare `<g>` so its children blend with the content beneath it. A
masked group SHALL be rasterized alone.

#### Scenario: A faded group of shapes stays vector

- **WHEN** a Normal group at opacity 128 contains two solid shape layers
- **THEN** the SVG has a `<g opacity="0.502">` containing two primitives and no
  `<image>`

### Requirement: Content SVG cannot express is flattened

A layer stack that reads the content beneath it — an adjustment layer, a blend
mode with no CSS equivalent, a clipping stack that is not plain source-over, or
a faded or masked pass-through group — SHALL be flattened with every lower layer
of its container into one raster, replacing their elements. Inside an isolated
group the flattened run is the group's own lower children; a pass-through group
SHALL defer the flatten to its parent container. A plain clipping stack (base
and clipped layers all Normal, base at full opacity) SHALL be one raster of the
base and its clipped layers.

#### Scenario: An adjustment layer flattens what lies beneath it

- **WHEN** a document has two pixel layers under an Invert adjustment layer
- **THEN** the SVG has one `<image>` for all three layers

#### Scenario: A clipped layer does not escape its base

- **WHEN** a pixel layer is clipped to a base layer
- **THEN** the base and the clipped layer are one `<image>`

### Requirement: Type layers use the raster fallback

The writer SHALL emit a type layer as its raster fallback, not as live `<text>`
or outlined glyphs, so the export does not depend on the viewer's fonts.

#### Scenario: A type layer is an image

- **WHEN** a document has one visible type layer
- **THEN** the SVG has an `<image>` for it and no `<text>`

### Requirement: The export renders like the composite

For a vector-only document, the SVG rendered by an independent SVG renderer
SHALL match the CPU composite at the pixel level within an antialiasing
tolerance. The check SHALL self-skip when the renderer is absent.

#### Scenario: librsvg agrees with the compositor

- **WHEN** a document of solid shapes with a Multiply layer and a faded group is
  exported and rendered by librsvg onto white
- **THEN** it matches the CPU composite flattened onto white within tolerance

#### Scenario: librsvg agrees for gradient, pattern, and stroke content [svg_oracle_primitives]

- **WHEN** a document of gradient and pattern fills and a solid outside stroke is
  exported and rendered by librsvg onto white
- **THEN** it matches the CPU composite flattened onto white within tolerance

### Requirement: Gradient fills emit SVG paint servers

A shape or fill layer whose decoded fill is a linear or radial gradient, whose
layer has no pixel mask that changes its appearance (a disabled or fully opaque
white mask is ignored), and whose only active effect is a representable stroke
SHALL emit an SVG primitive whose `fill` references a gradient paint server in
`<defs>` instead of a raster fallback. A linear gradient SHALL emit
`<linearGradient>` and a radial gradient SHALL emit `<radialGradient>`, each in
document user space with one `<stop>` per PSD colour stop at
`Lctn / 4096`. The geometry SHALL reproduce the compositor's layer-rect scale
and angle, and a reversed gradient SHALL reverse the stop offsets. A gradient
kind SVG cannot represent faithfully (angle, reflected, or diamond) or a
degenerate layer rectangle SHALL keep the raster fallback.

#### Scenario: A linear gradient shape references a linearGradient [svg_gradient_linear]

- **WHEN** a shape layer is filled with a linear two-stop gradient at angle 0
- **THEN** the SVG has a `<linearGradient>` in `<defs>` and the shape's `fill`
  is `url(#…)`, with no `<image>` for that layer

#### Scenario: A radial gradient shape references a radialGradient [svg_gradient_radial]

- **WHEN** a shape layer is filled with a radial two-stop gradient
- **THEN** the SVG has a `<radialGradient>` in `<defs>` and the shape's `fill`
  is `url(#…)`, with no `<image>` for that layer

#### Scenario: An unsupported gradient kind rasterizes [svg_gradient_fallback]

- **WHEN** a shape layer is filled with an angle, reflected, or diamond gradient
- **THEN** the layer is emitted as its raster fallback

### Requirement: Pattern fills emit an SVG pattern tile

A shape or fill layer whose decoded fill is a pattern, whose layer has no pixel
mask that changes its appearance (a disabled or fully opaque white mask is
ignored), and whose only active effect is a representable stroke SHALL emit a
`<pattern>` in `<defs>` whose `fill` reference is the shape's `fill`. The
pattern SHALL be in document user space, SHALL tile at the compositor's
`round(pattern size * scale / 100)` size (at least one pixel), SHALL embed the
nearest-neighbour scaled pattern pixels as a base64 PNG `<image>`, and SHALL
anchor the tile by the fill's link and origin. A missing pattern SHALL use the
compositor's opaque grey placeholder tile.

#### Scenario: A pattern shape references a pattern tile [svg_pattern]

- **WHEN** a shape layer is filled with a document pattern at scale 100
- **THEN** the SVG has a `<pattern>` in `<defs>` containing a base64 PNG
  `<image>` and the shape's `fill` is `url(#…)`, with no `<image>` for that layer

### Requirement: Solid vector strokes emit SVG stroke attributes

A shape layer whose only active effect is an enabled, Normal-blend, solid-colour
stroke SHALL emit the stroke as SVG `stroke`, `stroke-width`, and
`stroke-opacity` attributes instead of rasterizing the layer. A center stroke
SHALL use the primitive's centerline. An inside or outside stroke SHALL use
twice the stroke width and SHALL be masked to the shape's interior or exterior
so the visible band matches the compositor. A stroke with a gradient or pattern
fill, a non-Normal blend, or any other active layer effect SHALL keep the raster
fallback.

#### Scenario: A solid outside stroke emits stroke attributes [svg_stroke_outside]

- **WHEN** a shape layer has a solid outside stroke and no other active effect
- **THEN** the SVG has a stroke-only element with `stroke`, `stroke-width`, and
  a mask that keeps only the exterior band

#### Scenario: A solid inside stroke emits stroke attributes [svg_stroke_inside]

- **WHEN** a shape layer has a solid inside stroke and no other active effect
- **THEN** the SVG has a stroke-only element masked to the shape's interior

#### Scenario: A stroke with another effect rasterizes [svg_stroke_fallback]

- **WHEN** a shape layer has a stroke plus a drop shadow, a non-solid stroke
  fill, or a non-Normal stroke blend
- **THEN** the layer is emitted as its raster fallback

### Requirement: Per-layer SVG export writes one file per checked row

When `File ▸ Export As…` uses the SVG format, each checked row SHALL be written
as its own document-sized `.svg` file. The document row SHALL be the whole
document; a layer row SHALL be that top-level layer alone, including its group
subtree, on the document canvas. One checked row SHALL prompt for a single file.
Several checked rows SHALL prompt for a folder and write one file per row, using
the document name or layer name as the base and disambiguating duplicate names.
Every SVG file SHALL be written atomically (temp file plus rename).

#### Scenario: A checked layer row writes a document-sized SVG [svg_export_per_layer]

- **WHEN** the only checked Export As row is a top-level layer and the format is
  SVG
- **THEN** the written file has the document's `width`, `height`, and `viewBox`
  and contains only that layer's content

#### Scenario: Several checked rows write one file each [svg_export_per_layer_multi]

- **WHEN** several Export As rows are checked and the format is SVG
- **THEN** the chosen folder receives one `.svg` per checked row, named from the
  document or layer name and without overwriting a duplicate
