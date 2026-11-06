# codec/empty-layer-rect Specification

## Purpose
Keeps a pixel layer's stored rect in PSD: a new empty layer writes a 0×0 record,
a layer that grew writes its grown rect, and an existing file still round-trips
byte-identically.

## Requirements

### Requirement: An empty layer round-trips

A pixel layer whose rect is empty SHALL be written with a 0×0 layer record and
zero-length channel payloads. Reading it back SHALL yield the same 0×0 rect, the
same channel ids, and empty planes.

#### Scenario: An empty layer round-trips [elr_empty]

- **WHEN** a document with an empty-rect pixel layer is written and read back
- **THEN** the layer's rect is still 0×0, its channel ids are preserved, and its
  planes are empty

### Requirement: A grown layer round-trips

A layer that grew SHALL be written with its grown rect and its padded pixels.
Reading it back SHALL return the same rect and pixels, and psd-tools SHALL report
the same rect and pixels.

#### Scenario: A grown layer matches the psd-tools oracle [elr_grown_oracle]

- **WHEN** a layer that grew is written and the file is read by psd-tools
- **THEN** psd-tools reports the grown rect and the same pixels, and Pictura
  reads back the same rect and pixels

### Requirement: Existing files stay byte-identical

Reading and writing an existing PSD without an edit SHALL produce byte-identical
output.

#### Scenario: An existing file is byte-identical [elr_identical]

- **WHEN** an existing PSD is read and written without an edit
- **THEN** the bytes are identical to the input
