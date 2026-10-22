# tools/text-rasterize-bundled Specification

## MODIFIED Requirements

### Requirement: The Rasterize Type command can render through the Qt backend

`PictureView::rasterize_type` SHALL attempt a Qt `QFont`/`QPainter` text render
of the layer's text and style and, when it yields a non-empty raster, replace
the layer's channels with it, drop `TySh`, clear the type tool, and record one
`Rasterize Type` state. When the Qt render is empty it SHALL fall back to the
bundled backend. A non-type layer SHALL still be refused without mutation.
The Qt render SHALL align the text by the justification bytes 0, 1 and 2; the
bytes 3 to 6 SHALL render left-aligned, because the Qt text layout cannot
justify a paragraph's last line to the right or centre it.

#### Scenario: A type layer rasterizes through Qt

- **WHEN** `rasterize_type` is called on a type layer
- **THEN** the layer's channels hold the Qt-rendered text, it is no longer a type layer, and one state is recorded

#### Scenario: An empty Qt render falls back to the bundled face

- **WHEN** the Qt render yields no pixels
- **THEN** the bundled backend materializes the layer instead

#### Scenario: A non-type layer is refused

- **WHEN** `rasterize_type` is called on a plain pixel layer
- **THEN** it returns false and records no state
