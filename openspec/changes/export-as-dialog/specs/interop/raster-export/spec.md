# raster-export Specification (delta)

## MODIFIED Requirements

### Requirement: Export As writes the flattened composite

The application SHALL provide `File ▸ Export As…`. It SHALL open the CC-style
Export As dialog (`ui/export-as-dialog`), whose checked rows are the export
targets: the flattened document composite and, optionally, one or more top-level
layers. It SHALL write each target through the Qt-boundary encode edge or, for
PNG-8 and GIF, through the engine web encoders. It SHALL NOT change the
document's path, modified state, or history, and SHALL NOT write a selection. A
refusal SHALL write nothing and leave the document unchanged.

#### Scenario: Export As writes a raster without modifying the document

- **WHEN** the user runs Export As, chooses PNG, and confirms a path
- **THEN** a PNG holding the flattened composite is written, the document's path and modified state are unchanged, and no history state is added

#### Scenario: Quality applies only where meaningful

- **WHEN** the user runs Export As and selects JPEG
- **THEN** the quality control is enabled and its value is passed to the encoder

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

### Requirement: Export As supports PNG-8 and GIF

The Export As Format list SHALL offer PNG, JPG, PNG-8, and GIF. PNG-8 and GIF
SHALL be produced by the engine web encoders (`pictura_codec::web`), not by a Qt
writer: the flattened target SHALL be quantized to an indexed palette and, for
GIF, encoded as GIF89a. A Transparency control SHALL be offered for PNG, PNG-8,
and GIF, and a Quality control for JPG; a control that does not apply to the
chosen format SHALL be disabled. The written suffix SHALL be `png`, `jpg`,
`png`, and `gif` respectively.

#### Scenario: PNG-8 writes an indexed image

- **WHEN** the user exports a colourful document as PNG-8
- **THEN** the written PNG decodes to the target size and holds no more than 256 distinct colours

#### Scenario: GIF writes a GIF89a file

- **WHEN** the user exports the document as GIF
- **THEN** the written file begins with the GIF89a signature and decodes to the target size

#### Scenario: Quality applies only to JPG

- **WHEN** the Format is PNG-8
- **THEN** the Quality control is disabled and the Transparency control is enabled

#### Scenario: SVG is not offered

- **WHEN** the Format list is opened
- **THEN** it contains PNG, JPG, PNG-8, and GIF, and does not contain SVG

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
