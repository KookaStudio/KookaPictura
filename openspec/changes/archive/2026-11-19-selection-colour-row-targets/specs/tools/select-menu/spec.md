# Spec Delta

## REMOVED Requirements

### Requirement: Save and Load Selection

**Reason**: The Load half changes source selection: it now lists the document's
colour rows and no longer refuses a document with no saved channel. That
reverses the "Load is disabled with no channels" scenario, so the requirement is
replaced rather than amended.

**Migration**: Read "Save Selection destinations" and "Load Selection sources".
Save keeps its channel set; Load gains colour rows, the first-extra-channel
default, and a usable list for a document with no saved channels.

## ADDED Requirements

### Requirement: Save Selection destinations

`Select > Save Selection` SHALL open a dialog with Document, Channel, Name, and
Operation controls. Document SHALL default to the active document and SHALL list
the active document, every other open document with equal pixel width and
height, and a `New` entry; a document with different dimensions MUST NOT be
listed. Channel SHALL default to a new extra channel and SHALL list a new
channel, every existing extra channel in the chosen document (alpha and spot
alike), and every existing raster layer mask in the chosen document. Vector
masks and colour rows MUST NOT be listed: a colour row is a derived view of the
composite rather than a stored channel, and the dialog has no control that would
choose which layer receives a write. Name SHALL be enabled only for a new
channel; when the field is left empty the saved channel name SHALL default to
`Alpha N`, where N is the next creation-order number. Operation SHALL be
enabled only for an existing extra channel or layer mask and SHALL offer
Replace, Add, Subtract, and Intersect, defaulting to Replace.

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

#### Scenario: Save does not list colour rows [sm_save_excludes_colour]

- **WHEN** the Save Selection dialog lists channels for an RGB document with one extra channel and one raster layer mask
- **THEN** it lists New Channel, the extra channel, and the layer mask, and no RGB, Red, Green, or Blue row

#### Scenario: The document list is same-size only

- **WHEN** another open document has different pixel dimensions
- **THEN** it is absent from both the Save and Load Document lists

#### Scenario: The 56-channel limit refuses a new channel

- **WHEN** a save to a new channel would create channel 57
- **THEN** the save is refused with no channel added and no history state

### Requirement: Load Selection sources

`Select > Load Selection` SHALL open a dialog with Document, Channel, Invert, and
Operation controls. Document SHALL default to the active document and SHALL list
the active document and every other open document with equal pixel width and
height, with no `New` entry. Channel SHALL list, in order, the chosen document's
colour rows as the Channels panel lays them out (the composite row where the
mode has one, then its colour plates), every existing raster layer mask, and
every extra channel under its stored name. The composite row SHALL derive
coverage as the luminance of all its colour channels; each plate row SHALL use
the Channels panel's mode-specific derivation for that row (the colour plane for
RGB, the naive ink level for CMYK and Multichannel, and the luminance for Lab).
Invert SHALL default off. Operation SHALL offer New, Add, Subtract, and
Intersect and SHALL default to New. The default Channel SHALL be the first extra
channel when the document has one, otherwise the first listed colour row (the
composite row where the mode has one). Loading SHALL derive coverage from the
chosen source, invert it when Invert is checked, and combine it with the current
selection using the chosen operation. Loading SHALL record exactly one undo
state in the active document. Load SHALL remain available for a document that
has no extra channel and no raster layer mask. A source whose coverage length
does not equal `width * height` MUST be refused without recording. A
save-then-load round trip at the same dimensions SHALL reproduce the coverage
byte for byte.

#### Scenario: Load inverts before combining

- **WHEN** a channel holding coverage 64 is loaded with Invert checked and Add into a selection of 128
- **THEN** the resulting coverage is 191

#### Scenario: Load from another document

- **WHEN** Load Selection reads an extra channel from another open document of the same size
- **THEN** the active document's selection becomes the source coverage and exactly one state is recorded in the active document

#### Scenario: Load lists existing channels

- **WHEN** the document has saved selections and Load Selection is invoked
- **THEN** the dialog lists each extra channel by its stored name and loads the chosen one

#### Scenario: Load lists colour rows in channel order [sm_load_lists_colour]

- **WHEN** Load Selection is invoked for an RGB document with one extra channel and one raster layer mask
- **THEN** the Channel list reads RGB, Red, Green, Blue, the layer mask, then the extra channel

#### Scenario: Load loads a colour plate [sm_load_colour_plane]

- **WHEN** the Red, Green, or Blue row of a flat (100, 150, 200) image is loaded with New
- **THEN** every pixel's selection coverage is 100, 150, or 200 respectively

#### Scenario: The composite row loads its luminance [sm_load_composite]

- **WHEN** the RGB row of a flat (100, 150, 200) image is loaded with New
- **THEN** every pixel's selection coverage is 140

#### Scenario: A document with no saved channels still loads [sm_load_no_saved]

- **WHEN** Load Selection is invoked for a document with no extra channel and no raster layer mask
- **THEN** the Channel list holds that document's colour rows and OK is enabled

#### Scenario: A single-row mode lists one composite row [sm_load_single_row_mode]

- **WHEN** Load Selection is invoked for a Grayscale document with no saved channels
- **THEN** the Channel list holds one `Gray` row and loading it selects the image's luminance

#### Scenario: The default source is the first extra channel [sm_load_default]

- **WHEN** Load Selection opens for a document that has an extra channel and a raster layer mask
- **THEN** the Channel selection is the first extra channel rather than the composite row

#### Scenario: A composite-less mode defaults to its first colour row [sm_load_multichannel_default]

- **WHEN** Load Selection opens for a Multichannel document with no extra channel
- **THEN** the default target is the first colour row (Cyan), `color:0`

#### Scenario: Cross-document colour row load [sm_load_colour_cross_doc]

- **WHEN** Load Selection reads a colour row from another open document of the same size
- **THEN** the active document's selection becomes that row's derived coverage and exactly one state is recorded in the active document

#### Scenario: Wrong-length channel is refused

- **WHEN** the chosen source's data length is not `width * height`
- **THEN** the command is refused and no history state is recorded

#### Scenario: Round trip is exact

- **WHEN** a selection is saved to a channel and loaded back
- **THEN** the resulting selection equals the saved selection byte for byte and one undo state is recorded
