# Spec Delta

## MODIFIED Requirements

### Requirement: Raster output is encoded at the Qt boundary

The application SHALL encode a document's flattened composite to a raster file
using Qt's image writers at the C++ app boundary and MUST NOT add a raster
encoder to the engine crates, with the sole exceptions of the native WebP
encoder required because Qt's WebP writer is an optional plugin (see the
requirement below) and the engine GIF89a encoder used by GIF Save As
(`interop/gif-save`). The engine SHALL remain Qt-free; the app SHALL produce a
packed RGBA8888 buffer from the composite and hand it to the boundary, which
SHALL build a `QImage` and save it, except for WebP and GIF Save As, which the
engine SHALL encode. Supported output formats for this Qt edge SHALL be PNG,
JPEG, TIFF, WebP, and BMP, selected by the output path's extension and
independent of case. PNG, TIFF, and WebP SHALL preserve the composite's alpha;
JPEG and BMP, which cannot portably carry it, SHALL be written flattened onto an
opaque background. The boundary SHALL accept a percent scale and SHALL rescale
the image with a smooth filter when the scale is not 100, for the WebP path as
well. Writes SHALL be atomic (temp file plus rename), so a failure never
truncates an existing file and a missing or unwritable path, an unwritable
format, or a format the runtime build cannot write SHALL return failure without
producing a partial file. GIF Save As is outside this Qt edge and is governed by
`interop/gif-save`.

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

#### Scenario: GIF Save As uses the engine, not the Qt edge [raster-export-gif]

- **WHEN** a document is saved to a `.gif` destination
- **THEN** the engine GIF89a encoder writes the file and the Qt raster edge is not used
