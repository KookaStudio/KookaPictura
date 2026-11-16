# Spec Delta

## MODIFIED Requirements

### Requirement: Raster output is encoded at the Qt boundary

This requirement governs the Export As and Quick Export encode edge. `File ▸
Save As` raster writers are specified by `codec/raster-save-encoders` and
`interop/save-format-options`, and MUST NOT be routed through the Export As
encode path. A JPEG payload callback used by the Save As TIFF/EPS writers is a
payload encoder, not the Export As encode path, and is permitted.

For export, the application SHALL encode a document's flattened composite to a
raster file using Qt's image writers at the C++ app boundary and MUST NOT add a
raster encoder to the engine crates for the Export As path, with the sole
exception of the native WebP encoder required because Qt's WebP writer is an
optional plugin (see the requirement below). The engine SHALL remain Qt-free;
the app SHALL produce a packed RGBA8888 buffer from the composite and hand it to
the boundary, which SHALL build a `QImage` and save it, except for WebP, which
the engine SHALL encode. Supported export output formats SHALL be PNG, JPEG,
TIFF, WebP, and BMP, selected by the output path's extension and independent of
case. PNG, TIFF, and WebP SHALL preserve the composite's alpha; JPEG and BMP,
which cannot portably carry it, SHALL be written flattened onto an opaque
background. The boundary SHALL accept a percent scale and SHALL rescale the
image with a smooth filter when the scale is not 100, for the WebP path as well.
Writes SHALL be atomic (temp file plus rename), so a failure never truncates an
existing file and a missing or unwritable path, an unwritable format, or a
format the runtime build cannot write SHALL return failure without producing a
partial file.

#### Scenario: PNG preserves the composite alpha

- **WHEN** a document whose composite has a transparent pixel is exported to a `.png`
- **THEN** the written PNG has that pixel transparent

#### Scenario: JPEG flattens alpha onto an opaque background

- **WHEN** a document whose composite has a transparent pixel is exported to a `.jpg`
- **THEN** the written JPEG has that pixel opaque

#### Scenario: BMP flattens alpha onto an opaque background

- **WHEN** a document whose composite has a transparent pixel is exported to a `.bmp`
- **THEN** the written BMP has that pixel opaque

#### Scenario: Scale resizes the output

- **WHEN** a `W × H` document is exported at 50 percent
- **THEN** the written image is `W/2 × H/2` (rounded) and keeps its aspect ratio

#### Scenario: An unwritable target fails without a partial file

- **WHEN** the encode edge is given a path in a directory that does not exist
- **THEN** it returns failure and no file is created
