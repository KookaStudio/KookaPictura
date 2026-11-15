# Spec Delta

## MODIFIED Requirements

### Requirement: Save and Load Selection

`Select > Save Selection` SHALL open a dialog with Document, Channel, Name, and
Operation controls. Document SHALL default to the active document and SHALL list
the active document, every other open document with equal pixel width and
height, and a `New` entry; a document with different dimensions MUST NOT be
listed. Channel SHALL default to a new extra channel and SHALL list a new
channel, every existing extra channel in the chosen document, and every existing
raster layer mask in the chosen document; vector masks and colour channels MUST
NOT be listed. Name SHALL be enabled only for a new channel and SHALL default to
`Alpha N`, where N is the next creation-order number. Operation SHALL be enabled
only for an existing extra channel or layer mask and SHALL offer Replace, Add,
Subtract, and Intersect, defaulting to Replace.

Saving to a new channel SHALL append the selection coverage to the chosen
document's extra channels as an 8-bit plane and SHALL store the entered name.
Saving into an existing extra channel SHALL combine the selection with that
channel's coverage using the selection-model algebra and the chosen operation.
Saving into an existing raster layer mask SHALL write the combined coverage into
that mask, preserving its density, feather, flags, and enabled state, with
selected coverage becoming white (revealed). Each save SHALL record exactly one
undo state in the document that receives the change; a refused save SHALL record
nothing. Save SHALL be disabled without an active selection or without a
document. Save SHALL refuse, without recording, a new channel that would exceed
the document's 56-channel total limit. Saving to `New` SHALL create a
single-channel grayscale document of the same pixel dimensions containing the
selection as its extra channel.

`Select > Load Selection` SHALL open a dialog with Document, Channel, Invert, and
Operation controls. Document SHALL default to the active document and SHALL list
the active document and every other open document with equal pixel width and
height. Channel SHALL list every extra channel and every existing raster layer
mask in the chosen document. Invert SHALL default off. Operation SHALL offer New,
Add, Subtract, and Intersect and SHALL default to New. Loading SHALL derive
coverage from the chosen source, invert it when Invert is checked, and combine it
with the current selection using the chosen operation. Loading SHALL record
exactly one undo state in the active document. Load SHALL be disabled when no
listed document has an extra channel or raster layer mask. A source whose
coverage length does not equal `width * height` MUST be refused without
recording. A save-then-load round trip at the same dimensions SHALL reproduce the
coverage byte for byte.

#### Scenario: Save to a new channel stores the name

- **WHEN** a selection is saved to a new channel named "Hair"
- **THEN** the chosen document gains one extra channel holding the selection coverage and the channel is listed as "Hair"

#### Scenario: Save combines with an existing channel

- **WHEN** a selection is saved into an existing channel with Replace, Add, Subtract, or Intersect
- **THEN** the channel coverage becomes the incoming coverage, `max(existing, incoming)`, `max(existing - incoming, 0)`, or `round(existing * incoming / 255)` respectively

#### Scenario: Save into a layer mask reveals the selection

- **WHEN** a selection is saved into an existing raster layer mask with Replace
- **THEN** the selected pixels become white, the unselected pixels become black, and the mask's density, feather, flags, and enabled state are unchanged

#### Scenario: Save to a new image

- **WHEN** Save Selection is set to the `New` document
- **THEN** a new grayscale document of the same dimensions opens with the selection as its extra channel and the source document is unchanged

#### Scenario: The document list is same-size only

- **WHEN** another open document has different pixel dimensions
- **THEN** it is absent from both the Save and Load Document lists

#### Scenario: Load inverts before combining

- **WHEN** a channel holding coverage 64 is loaded with Invert checked and Add into a selection of 128
- **THEN** the resulting coverage is 191

#### Scenario: Load from another document

- **WHEN** Load Selection reads an extra channel from another open document of the same size
- **THEN** the active document's selection becomes the source coverage and exactly one state is recorded in the active document

#### Scenario: Load lists existing channels

- **WHEN** the document has saved selections and Load Selection is invoked
- **THEN** the dialog lists each extra channel by its stored name and loads the chosen one

#### Scenario: Load is disabled with no channels

- **WHEN** no listed document has an extra channel or raster layer mask
- **THEN** Load Selection is disabled and applying it changes nothing

#### Scenario: The 56-channel limit refuses a new channel

- **WHEN** a save to a new channel would create channel 57
- **THEN** the save is refused with no channel added and no history state

#### Scenario: Wrong-length channel is refused

- **WHEN** the chosen source's data length is not `width * height`
- **THEN** the command is refused and no history state is recorded

#### Scenario: Round trip is exact

- **WHEN** a selection is saved to a channel and loaded back
- **THEN** the resulting selection equals the saved selection byte for byte and one undo state is recorded
