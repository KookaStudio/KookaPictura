# select-menu Specification

## Purpose
The Select menu command surface: Reselect, Inverse, Modify, Grow, Similar, Save/Load Selection, and layer selection.

## Requirements

### Requirement: Reselect and Inverse

`Select > Inverse` SHALL replace the selection with its complement
(`255 - coverage` per pixel). `Select > Reselect` SHALL restore the selection
that was most recently cleared by Deselect or replaced by a New-mode selection.
Both SHALL record exactly one undo state on success and SHALL be disabled when
no document is open or when the required prior selection does not exist.

#### Scenario: Inverse flips coverage

- **WHEN** a selection exists and Inverse is applied
- **THEN** every pixel's coverage becomes its complement and the result is one undo state

#### Scenario: Reselect restores the last deselected selection

- **WHEN** a selection is deselected and Reselect is applied
- **THEN** coverage equals the deselected selection byte for byte and the result is one undo state

#### Scenario: Reselect is disabled with no stored selection

- **WHEN** no selection has been deselected since the document opened
- **THEN** Reselect is disabled and applying it changes nothing

### Requirement: Modify submenu operations

`Select > Modify` SHALL open a parameter dialog for the chosen operation and
apply the corresponding engine operation to the current selection: Border
(width 1-200 px), Smooth (radius 1-100), Expand (radius 1-100), Contract (radius
1-100), and Feather (radius 0-250 px, decimal). A successful operation SHALL
record exactly one undo state; a cancelled or refused dialog SHALL record
nothing and leave the selection unchanged. The Feather dialog SHALL apply the
same inferred `sigma = radius / 2` profile as tool-time feathering.

#### Scenario: Border replaces the selection with a band

- **WHEN** Border is applied with width 6 to a rectangular selection
- **THEN** the selection becomes a band straddling the original edge and one undo state is recorded

#### Scenario: Expand then Contract restores a binary block

- **WHEN** Expand 3 is applied to a binary block clear of the canvas edge and then Contract 3 is applied
- **THEN** the selection equals the original block

#### Scenario: Feather builds a ramp

- **WHEN** Feather is applied with a positive radius to a hard-edged selection
- **THEN** boundary pixels hold values strictly between 0 and 255

#### Scenario: Cancelling records nothing

- **WHEN** a Modify dialog is cancelled
- **THEN** the selection and history are unchanged

### Requirement: Grow and Similar from the Select menu

`Select > Grow` and `Select > Similar` SHALL operate on the current selection
against the visible composite buffer using the Magic Wand options-bar Tolerance.
Grow SHALL add only contiguously adjacent similar pixels; Similar SHALL add every
similar pixel regardless of adjacency. Each SHALL record exactly one undo state
on success and SHALL be disabled with no selection.

#### Scenario: Grow follows connectivity

- **WHEN** Grow is applied to a selection inside a uniform patch next to a differently coloured region
- **THEN** the patch becomes selected and the different-coloured region does not

#### Scenario: Similar reaches disconnected patches

- **WHEN** Similar is applied to a selection and a same-coloured patch exists elsewhere
- **THEN** the disconnected patch is selected as well

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

### Requirement: Layer selection commands

`Select > All Layers` SHALL select every layer row in the Layers panel,
`Select > Deselect Layers` SHALL clear the layer-row selection, and
`Select > Similar Layers` SHALL select every layer matching the current layer's
node class, adjustment kind, and blend mode. These commands change panel
selection only and SHALL record no history state. They SHALL be disabled with no
document, and Similar Layers SHALL be disabled when no layer is current.

#### Scenario: All Layers selects every row

- **WHEN** All Layers is applied
- **THEN** all layer rows are selected and no history state is recorded

#### Scenario: Deselect Layers clears the selection

- **WHEN** Deselect Layers is applied
- **THEN** no layer row is selected and no history state is recorded

#### Scenario: Similar Layers matches the current layer

- **WHEN** Similar Layers is applied with a pixel layer current
- **THEN** every pixel layer with that layer's blend mode is selected, and no history state is recorded

### Requirement: Select menu enablement and undo semantics

A Select menu command that requires a document SHALL be disabled when no
document is open; a command that requires a selection SHALL be disabled when no
selection is active. Every applied command SHALL produce at most one history
state, and a refusal or cancellation SHALL produce none. Enabling a previously
disabled row MUST NOT alter the disabled state of any deferred row.

#### Scenario: No document disables the menu

- **WHEN** no document is open
- **THEN** every document-requiring Select command is disabled and the application does not crash

#### Scenario: No selection disables selection commands

- **WHEN** a document is open with no active selection
- **THEN** Inverse, Modify, Grow, Similar, and Save Selection are disabled

#### Scenario: One undo state per applied command

- **WHEN** any enabled Select command is applied successfully
- **THEN** the history grows by exactly one state

#### Scenario: A refusal records nothing

- **WHEN** a Select command is refused by the bridge or cancelled in its dialog
- **THEN** the history count and the selection are unchanged

### Requirement: Live Select menu command surface

The system SHALL implement the CS6 Select menu rows Reselect, Inverse, the
Modify submenu (Border, Smooth, Expand, Contract, Feather), Grow, Similar, Save
Selection, Load Selection, All Layers, Deselect Layers, Similar Layers, Color
Range, Refine Edge, and Transform Selection through stable command ids and
bridge operations. Each row SHALL keep its documented menu path and shortcut.
Every row SHALL be a live command; Transform Selection SHALL be enabled only
when a non-empty selection exists.

#### Scenario: Implemented rows are enabled with a document [sm_surface]

- **WHEN** a document is open
- **THEN** Reselect, Inverse, Modify, Grow, Similar, Save Selection, Color Range, Refine Edge, and Transform Selection are present and live

#### Scenario: Transform Selection needs a selection [sm_transform_selection]

- **WHEN** a document is open with no active selection, or with an empty selection
- **THEN** Transform Selection is visible and disabled
