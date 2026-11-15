# Spec Delta

## ADDED Requirements

### Requirement: Save into an existing extra channel

Saving a selection into an existing extra channel SHALL combine the channel's
current coverage `a` with the incoming selection coverage `b` using the
selection-model algebra and the chosen operation: Replace = `b`;
Add = `max(a, b)`; Subtract = `max(a - b, 0)`; Intersect =
`round(a * b / 255)`. Soft coverage MUST NOT be thresholded or quantized by the
combine. The operation SHALL record exactly one undo state and SHALL make the
previous channel coverage losslessly recoverable through undo.

#### Scenario: Replace overwrites the channel

- **WHEN** a selection is saved into an existing channel with Replace
- **THEN** the channel coverage equals the incoming selection byte for byte

#### Scenario: Boolean operations combine soft coverage

- **WHEN** an existing channel holding 128 is combined with an incoming selection holding 64
- **THEN** Add yields 128, Subtract yields 64, and Intersect yields 32

#### Scenario: Saving into a channel is one state

- **WHEN** Save Selection writes into an existing channel
- **THEN** the history gains exactly one state and undo restores the previous channel bytes

### Requirement: Load with invert and operation

Loading a selection from an extra channel or raster layer mask SHALL derive the
source coverage, replace it with `255 - coverage` when Invert is checked, and
then combine it with the current selection using the chosen operation from the
selection-model algebra. With no current selection, New and Add SHALL produce
the effective source coverage, while Subtract and Intersect SHALL produce an
empty selection. The load SHALL record exactly one undo state.

#### Scenario: Invert is applied before the combine

- **WHEN** coverage 64 is loaded with Invert and Add into a selection of 128
- **THEN** the effective source is 191 and the result is `max(128, 191) = 191`

#### Scenario: No selection with each operation

- **WHEN** a channel is loaded with no current selection
- **THEN** New and Add load the effective source, while Subtract and Intersect load an empty selection

#### Scenario: Load is one state

- **WHEN** a channel is loaded with Add, Subtract, or Intersect
- **THEN** the history gains exactly one "Load Selection" state

### Requirement: Channel names round-trip through PSD

Document extra-channel names SHALL be stored in the PSD alpha-name image
resources: resource 1006 as Pascal strings and resource 1045 as Unicode strings.
On read, 1045 SHALL be preferred and 1006 SHALL be the fallback; names SHALL map
to extra channels by channel order. An extra channel with no stored name SHALL
be displayed as `Alpha N` where N is its one-based position. Writing names SHALL
preserve every other image-resource block and the unparsed tail byte for byte.
Deleting an extra channel SHALL remove its name and shift the following names so
that the remaining channels keep their names. A document whose names are
unchanged SHALL re-emit its original resource bytes.

#### Scenario: ASCII and non-ASCII names round-trip

- **WHEN** extra channels named "Hair" and "Ciel étoilé" are written and read back
- **THEN** both names are restored in channel order

#### Scenario: Missing names fall back

- **WHEN** an extra channel has no stored name
- **THEN** the panel and dialogs display `Alpha N` for its position

#### Scenario: Deleting a channel keeps later names aligned

- **WHEN** the second of three named channels is deleted
- **THEN** the first and third keep their names and the third becomes the second channel

#### Scenario: Unedited names preserve the resource bytes

- **WHEN** a document with alpha-name resources is saved without changing its names
- **THEN** the alpha-name resource bytes are unchanged

#### Scenario: psd-tools reports the names

- **WHEN** psd-tools opens a written PSD with named extra channels
- **THEN** it reports the same alpha-channel names in order
