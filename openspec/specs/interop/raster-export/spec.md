# raster-export Specification

## Purpose

Qt-boundary encoding of a document's flattened composite to the supported raster
formats (PNG, JPEG, TIFF, WebP, BMP), with format/quality/scale control, and the
Export As and Quick Export entry points that use it.

## Requirements

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

### Requirement: Quick Export as PNG writes beside the source

The application SHALL provide `File ▸ Quick Export as PNG`. With no dialog it
SHALL write `<source stem>.png` beside the document's file and record no history
state. When the document is untitled it SHALL request a path once before
writing. On success it SHALL NOT change the document's path or modified state.

#### Scenario: Quick Export writes next to the source

- **WHEN** Quick Export is invoked on a document saved at `dir/photo.psd`
- **THEN** `dir/photo.png` is written, holds the flattened composite, and no history state is added

#### Scenario: Quick Export prompts once when untitled

- **WHEN** Quick Export is invoked on an untitled document
- **THEN** a path is requested before the write and, once chosen, the PNG is written without re-pointing the document

#### Scenario: Quick Export does not overwrite its own source

- **WHEN** Quick Export is invoked on a document whose own path already ends in `.png`
- **THEN** a path is requested instead of writing the flattened composite over the source file

### Requirement: The Layers row menu exports the composite

The Layers panel row context menu SHALL offer `Export As…` and `Quick Export as
PNG` on pixel and smart-object rows, and SHALL NOT offer them on group,
adjustment, and type rows. Both entries SHALL act on the flattened document
composite — never on the row's layer alone — and SHALL behave exactly as the
File menu commands of the same name.

#### Scenario: Pixel and smart-object rows offer export

- **WHEN** the user opens the row menu for a pixel layer or a smart-object layer
- **THEN** it contains Export As… and Quick Export as PNG

#### Scenario: Group and adjustment rows do not offer export

- **WHEN** the user opens the row menu for a group or an adjustment layer
- **THEN** it does not contain Export As… or Quick Export as PNG

#### Scenario: The row export matches the File menu command

- **WHEN** the user picks Quick Export as PNG from a pixel row's menu
- **THEN** the same file is written as the File menu Quick Export as PNG command writes

### Requirement: Export As format list and controls

The Export As Format list SHALL offer PNG, JPG, PNG-8, GIF, SVG, and WebP, and
the written suffix SHALL be `png`, `jpg`, `png`, `gif`, `svg`, and `webp`
respectively. PNG-8 SHALL be quantized to an indexed palette by the engine web
palette and written by Qt; GIF SHALL be encoded as GIF89a by the engine web
encoder; SVG SHALL be written by the engine SVG writer (`interop/svg-export`);
WebP SHALL be written by the engine's native WebP encoder. A Transparency
control SHALL be enabled for PNG, PNG-8, GIF, and WebP, and a Quality control
for JPG only (WebP export is lossless); a control that does not apply to the
chosen format SHALL be disabled.

#### Scenario: PNG-8 writes an indexed image

- **WHEN** the user exports a colourful document as PNG-8
- **THEN** the written PNG decodes to the target size and holds no more than 256 distinct colours

#### Scenario: GIF writes a GIF89a file

- **WHEN** the user exports the document as GIF
- **THEN** the written file begins with the GIF89a signature and decodes to the target size

#### Scenario: Quality applies only to JPG

- **WHEN** the Format is PNG-8 or WebP
- **THEN** the Quality control is disabled and the Transparency control is enabled

#### Scenario: The full format list is offered

- **WHEN** the Format list is opened
- **THEN** it contains PNG, JPG, PNG-8, GIF, SVG, and WebP

### Requirement: Export As exports selected layers

The dialog's layer list SHALL contain the flattened document plus each top-level
layer that has pixels (adjustment layers SHALL be excluded). Each row SHALL carry
a checkbox, and the checked rows SHALL be the export targets; the document SHALL
be checked by default. With exactly one row checked, `Export…` SHALL request a
file and write that row; with several, it SHALL request a folder and write one
file per row, the document named from the document and a layer from the layer
name. Each layer target SHALL be the layer rendered alone against transparency,
cropped to the layer's bounds, so a group exports its subtree and a pixel layer
exports its own pixels.

#### Scenario: A single checked layer writes one file

- **WHEN** only a layer row is checked and the user confirms a path
- **THEN** one file is written holding that layer's pixels on transparency, and the flattened document is not written

#### Scenario: Several checked rows write one file each

- **WHEN** the document and one layer are checked and the user chooses a folder
- **THEN** the folder contains a file named from the document and a file named from the layer

#### Scenario: Adjustment layers are not offered

- **WHEN** the document contains an adjustment layer
- **THEN** the layer list has no row for it

#### Scenario: A group exports its subtree

- **WHEN** a group layer row is checked and exported
- **THEN** the written image holds the group's composited children on transparency, cropped to their union bounds

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

### Requirement: Raster Save As preserves document metadata

When a document is saved to JPEG or TIFF, the application SHALL write the
document's EXIF, IPTC-IIM, and XMP metadata into the output container,
preserving the values read by File Info, including Title, ObjectName, and Make.
The write SHALL remain atomic: encode to a temporary file, inject the metadata,
and rename only after both succeed. A failure at any step SHALL leave the
previous file untouched and report failure. PNG, BMP, and WebP are outside this
change and continue to drop metadata.

#### Scenario: JPEG and TIFF keep the File Info fields

- **WHEN** the `metadata.psd` fixture is saved as `.jpg` and `.tif` and inspected with exiftool
- **THEN** each output carries the fixture's Title, ObjectName, and Make values
