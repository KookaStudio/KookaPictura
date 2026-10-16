# select-menu Specification

## Purpose
The Select menu command surface: Reselect, Inverse, Modify, Grow, Similar, Save/Load Selection, and layer selection.

## Requirements

### Requirement: Select menu command surface

The system SHALL implement the CS6 Select menu rows Reselect, Inverse, the
Modify submenu (Border, Smooth, Expand, Contract, Feather), Grow, Similar, Save
Selection, Load Selection, All Layers, Deselect Layers, and Similar Layers
through stable command ids and bridge operations. Each row SHALL keep its
documented menu path and shortcut. The rows Color Range and Refine Edge SHALL be
implemented commands; the row Transform Selection SHALL remain present but
disabled with a documented reason.

#### Scenario: Implemented rows are enabled with a document

- **WHEN** a document is open with an active selection
- **THEN** Reselect, Inverse, Modify, Grow, Similar, Save Selection, Color Range, and Refine Edge are enabled

#### Scenario: Deferred rows stay disabled

- **WHEN** the Select menu is opened
- **THEN** Transform Selection is visible and disabled

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

`Select > Save Selection` SHALL append the current selection to the document's
extra channels as an 8-bit coverage plane, and `Select > Load Selection` SHALL
reconstruct a selection from a chosen extra channel. Save SHALL be disabled
without a selection; Load SHALL be disabled with a document that has no extra
channels. Loading SHALL record exactly one undo state and SHALL refuse, without
recording, a channel whose byte length does not equal `width * height`. A
save-then-load round trip SHALL reproduce the coverage byte for byte.

#### Scenario: Round trip is exact

- **WHEN** a selection is saved to a channel and loaded back
- **THEN** the resulting selection equals the saved selection byte for byte and one undo state is recorded

#### Scenario: Load lists existing channels

- **WHEN** the document has saved selections and Load Selection is invoked
- **THEN** the dialog lists each channel and loads the chosen one

#### Scenario: Load is disabled with no channels

- **WHEN** the document has no extra channels
- **THEN** Load Selection is disabled and applying it changes nothing

#### Scenario: Wrong-length channel is refused

- **WHEN** the chosen channel's data length is not `width * height`
- **THEN** the command is refused and no history state is recorded

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
