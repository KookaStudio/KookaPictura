## MODIFIED Requirements

### Requirement: Raster output is encoded at the Qt boundary

The application SHALL encode a document's flattened composite to a raster file
using Qt's image writers at the C++ app boundary and MUST NOT add a raster
encoder to the engine crates, with the sole exception of the native WebP encoder
required because Qt's WebP writer is an optional plugin (see the requirement
below). The engine SHALL remain Qt-free; the app SHALL produce a packed RGBA8888
buffer from the composite and hand it to the boundary, which SHALL build a
`QImage` and save it, except for WebP, which the engine SHALL encode. Supported
output formats SHALL be PNG, JPEG, TIFF, WebP, and BMP, selected by the output
path's extension and independent of case. PNG, TIFF, and WebP SHALL preserve the
composite's alpha; JPEG and BMP, which cannot portably carry it, SHALL be written
flattened onto an opaque background. The boundary SHALL accept a percent scale
and SHALL rescale the image with a smooth filter when the scale is not 100, for
the WebP path as well. Writes SHALL be atomic (temp file plus rename), so a
failure never truncates an existing file and a missing or unwritable path, an
unwritable format, or a format the runtime build cannot write SHALL return
failure without producing a partial file.

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

### Requirement: Export As writes the flattened composite

The application SHALL provide `File ▸ Export As…`. It SHALL open the CC-style
Export As dialog (`ui/export-as-dialog`), whose checked rows are the export
targets: the flattened document composite and, optionally, one or more top-level
layers. It SHALL write each target through the encode edge — WebP through the
engine's native encoder, PNG-8 and GIF through the engine web encoders, every
other raster format through the Qt boundary. WebP and the formats that carry
alpha SHALL preserve the target's transparency. It SHALL NOT change the
document's path, modified state, or history, and SHALL NOT write a selection. A
refusal SHALL write nothing and leave the document unchanged.

#### Scenario: Export As writes a raster without modifying the document

- **WHEN** the user runs Export As, chooses PNG, and confirms a path
- **THEN** a PNG holding the flattened composite is written, the document's path and modified state are unchanged, and no history state is added

#### Scenario: Quality applies only where meaningful

- **WHEN** the user runs Export As and selects JPEG
- **THEN** the quality control is enabled and its value is passed to the encoder

#### Scenario: Export As offers WebP and preserves alpha

- **WHEN** the user runs Export As, chooses WebP, and confirms a path
- **THEN** a WebP holding the flattened composite is written, its transparent pixels stay transparent, and the document's path and modified state are unchanged

#### Scenario: Export As honors the typed extension

- **WHEN** the user runs Export As with the format combo on PNG but types `photo.jpg`
- **THEN** the written file is a JPEG, because the extension names the format

#### Scenario: Export As on an empty view refuses

- **WHEN** Export As is invoked with no open document
- **THEN** nothing is written and no history state is added

#### Scenario: Export As honors the dialog settings

- **WHEN** the user runs Export As, sets Image Size to 50 percent and Canvas Size to the image size, and confirms a path
- **THEN** the written raster is half the document size in each dimension

## ADDED Requirements

### Requirement: The engine encodes WebP natively

The codec SHALL expose `pictura_codec::encode_webp(rgba: &[u8], width: u32,
height: u32) -> Option<Vec<u8>>`. It SHALL encode tightly packed RGBA8888 as a
WebP container without Qt and SHALL preserve alpha. It SHALL return `None`
without panicking when `width` or `height` is zero or when `rgba` is shorter
than `width * height * 4`. The application SHALL route a `WEBP` save or export
target to this encoder — rescaling through the boundary first when a percent
scale is given — and SHALL write the result atomically (temp file plus rename),
so a failed encode leaves no partial file and does not change the document's
path, modified state, or history.

#### Scenario: WebP export preserves alpha

- **WHEN** a document whose composite has a transparent pixel is exported to a `.webp`
- **THEN** the written WebP has that pixel transparent

#### Scenario: WebP save round-trips through the engine

- **WHEN** a document is saved to a `.webp` path and reopened
- **THEN** the reopened pixels match the flattened composite, because both the write and the read use the engine's WebP codec rather than Qt

#### Scenario: A malformed buffer is refused

- **WHEN** `encode_webp` is given a zero dimension or a buffer shorter than `width * height * 4`
- **THEN** it returns `None` and does not panic
