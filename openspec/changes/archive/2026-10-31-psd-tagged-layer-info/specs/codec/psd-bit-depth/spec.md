# Delta: PSD bit depth

## MODIFIED Requirements

### Requirement: Layer channels are narrowed with the document

When a 16- or 32-bit document carries layers, `read_psd` SHALL decode each layer
channel — its color channels, the `-1` transparency channel, the `-2` mask
channel, and unmodeled channels — at the document depth and SHALL narrow it to
an 8-bit plane before assembling the layer, so the layer path agrees with the
narrowed composite. The layer's color channels SHALL then be converted by the
document's color-mode normalization. For a Grayscale or RGB document the
read path SHALL also retain the decoded source-depth samples of each layer
channel so an unchanged channel can be re-emitted at the source depth on save
(see the source-depth-on-save requirement).

#### Scenario: A 16-bit layer color channel is narrowed

- **WHEN** a depth-16 document carries a pixel layer whose color channel stores `u16` samples
- **THEN** the returned layer's color channel holds the 8-bit narrowing of those samples

#### Scenario: A 16-bit mask channel is narrowed

- **WHEN** a depth-16 document carries a `-2` mask channel
- **THEN** the returned mask plane is the 8-bit narrowing of the stored samples and is not decoded as an 8-bit row

#### Scenario: A 16-bit unmodeled channel is narrowed for the engine and restored on save

- **WHEN** a depth-16 document carries a layer channel the engine does not model (for example a `-3` or positive spot channel)
- **THEN** the channel is decoded at the document depth and narrowed to an 8-bit plane for the engine, and a save of the unedited document re-emits it at the source depth

#### Scenario: Layers stored in a tagged block are narrowed the same way

- **WHEN** a depth-16 or depth-32 document stores its layers in an `Lr16` or `Lr32` block
- **THEN** the returned layers, their narrowed channels, and their retained source-depth samples equal those of the same layers stored in the layer-info subsection

## ADDED Requirements

### Requirement: High-depth layer info is read from its tagged block

When the layer-info subsection is empty, `read_psd` SHALL read the document's
layers from the trailing tagged block keyed for the document depth: `Lr16` at
depth 16 and `Lr32` at depth 32, under either the `8BIM` or the `8B64` signature.
The contents SHALL be parsed as layer info, including group records. The parsed
block SHALL NOT remain in `Document.layer_section_extra`, and every other trailing
block SHALL stay verbatim. A block whose key does not match the document depth
SHALL be preserved unparsed. A malformed block SHALL fail the read.

#### Scenario: Grouped layers are read from a tagged block

- **WHEN** a depth-16 document stores a group of two layers in an `Lr16` block
- **THEN** the layer stack holds the group and its two layers

#### Scenario: The parsed block leaves the preserved blocks

- **WHEN** an `Lr16` block sits between two other trailing blocks
- **THEN** `layer_section_extra` holds exactly those two blocks

#### Scenario: A mismatched key stays preserved

- **WHEN** a depth-16 document carries an `Lr32` block and an empty layer-info subsection
- **THEN** the document has no layers and the block stays in `layer_section_extra`

#### Scenario: A PSB block with the 8B64 signature is read

- **WHEN** a depth-16 PSB stores its layers in an `Lr16` block signed `8B64` with an 8-byte length
- **THEN** the layer stack is read from it

#### Scenario: A malformed tagged block fails the read

- **WHEN** an `Lr16` block declares channel data past its end
- **THEN** `read_psd` returns an error and does not panic

### Requirement: High-depth layer info is written as a tagged block

When `write_psd` writes a document with layers at output depth 16 or 32, it SHALL
write a zero-length layer-info subsection and SHALL store the layers in one
`Lr16` or `Lr32` block ahead of the other trailing blocks. `read_psd` SHALL still
read layers stored in the subsection at those depths.

#### Scenario: An unchanged high-depth document saves in tagged layout

- **WHEN** a layered depth-16 or depth-32 document is written
- **THEN** the layer-info length is zero, the file has exactly one layer block, and reading it back returns an equal document

#### Scenario: A high-depth file with layers in the subsection still opens

- **WHEN** a depth-16 file stores its layers in the layer-info subsection
- **THEN** `read_psd` returns those layers

#### Scenario: An independent decoder reads tagged layers

- **WHEN** a tagged depth-16 fixture is read and written
- **THEN** `psd-tools` reports the same native composite planes and layer channels
