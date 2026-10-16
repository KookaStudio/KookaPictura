# codec/psd-mask-parameters Specification

## Purpose
Reads and writes the per-layer mask-parameters block (density and feather) in
PSD layer records.

## Requirements

### Requirement: Read the mask-parameters block

When a layer's mask-record flags have bit 4 set, the reader SHALL parse the
mask-parameters block: a selector byte followed, in order, by user-mask density
(`u8`, selector bit 0), user-mask feather (big-endian `f64`, bit 1),
vector-mask density (`u8`, bit 2), and vector-mask feather (`f64`, bit 3). The
block MUST be read after the 18-byte real-user-mask header when the layer
carries a `-3` channel, and directly after the flags byte otherwise. User
values SHALL be stored on the layer mask and vector values on the vector mask;
absent fields MUST keep the defaults (density 255, feather 0). A mask record
whose layer has no `-2` channel SHALL NOT produce a raster layer mask. A
truncated block MUST NOT panic.

#### Scenario: User density and feather are read

- **WHEN** a layer's mask record has flag bit 4 and selector `0x03` with density 128 and feather 12.5
- **THEN** the decoded layer mask has density 128 and feather 12.5

#### Scenario: The real-mask header precedes the parameters

- **WHEN** a layer carries a `-3` channel and a 36-byte-plus mask record with flag bit 4
- **THEN** the parameters are read after the real-mask header and the header bytes are preserved

#### Scenario: Vector parameters reach the vector mask

- **WHEN** a layer with a `vmsk` block and no `-2` channel has a parameter-only mask record with vector density 64
- **THEN** its vector mask has density 64 and the layer has no raster mask

### Requirement: Write the mask-parameters block

The writer SHALL set flag bit 4 and emit the parameters block exactly when a
layer mask or vector mask has a non-default density or feather, emitting only
the fields that differ from their defaults, after any preserved real-user-mask
header, and SHALL pad a record that carries the block to a multiple of 4
bytes; a record without the block MUST re-save its preserved bytes unchanged. A layer
with non-default vector parameters and no raster mask SHALL get a zero-rect
record with default colour 255. A written file MUST read back through
psd-tools with the same `MaskParameters` values.

#### Scenario: Parameters round-trip

- **WHEN** a layer mask with density 128 and feather 12.5 and a vector mask with feather 3 are saved and reopened
- **THEN** the reopened masks carry the same density and feather values

#### Scenario: psd-tools reads the written parameters

- **WHEN** the saved file is read by psd-tools
- **THEN** its `MaskParameters` report user density 128, user feather 12.5, and vector feather 3

#### Scenario: Defaults write no parameters

- **WHEN** a layer mask has density 255 and feather 0
- **THEN** its record has flag bit 4 clear and no parameters block
