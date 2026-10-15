# svg-export Specification

## Purpose

`File ▸ Export As…` SVG output: a Qt-free engine writer that serializes a
document to Photoshop-CC-shaped SVG text, emitting vector layers as SVG
primitives and raster layers as an embedded base64 PNG fallback.

## ADDED Requirements

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

For a layer whose content is pixels, a smart object, or a baked effect (and for
an adjustment layer, which has no pixels of its own), the writer SHALL rasterize
that layer alone to a PNG and embed it as
`<image xlink:href="data:img/png;base64,…" width="W" height="H"/>`. The MIME
subtype SHALL be spelled `img/png` to match the CC sample. A raster document
SHALL therefore produce exactly one `<image>` with no `x` / `y` attributes.

#### Scenario: A pixel layer is embedded, not traced

- **WHEN** a document has one opaque pixel layer covering the canvas
- **THEN** the SVG body is a single `<image>` whose decoded PNG equals that
  layer's pixels

#### Scenario: Hidden and zero-opacity layers are skipped

- **WHEN** a top-level layer is hidden or has zero opacity
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
`opacity/255`) and `mix-blend-mode` respectively. A blend mode with no CSS
equivalent SHALL fall back to a rasterized layer rather than emit a wrong
`mix-blend-mode`.

#### Scenario: Opacity becomes a group attribute

- **WHEN** a layer has opacity 128
- **THEN** its element is wrapped in `<g opacity="0.502">`

#### Scenario: Multiply maps to mix-blend-mode

- **WHEN** a layer has the Multiply blend mode
- **THEN** its group carries `mix-blend-mode:multiply`
