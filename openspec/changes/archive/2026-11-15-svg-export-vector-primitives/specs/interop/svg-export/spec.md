# interop/svg-export Specification

## ADDED Requirements

### Requirement: Gradient fills emit SVG paint servers

A shape or fill layer whose decoded fill is a linear or radial gradient, whose
layer has no pixel mask, and whose only active effect is a representable stroke
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
mask, and whose only active effect is a representable stroke SHALL emit a
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

## MODIFIED Requirements

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
