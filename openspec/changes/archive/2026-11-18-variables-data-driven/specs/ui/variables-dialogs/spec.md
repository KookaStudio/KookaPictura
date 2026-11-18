# Variables and data set commands

## Purpose

Exposes template authoring, import, preview, apply, and PSD export through the document menu commands.

## ADDED Requirements

### Requirement: Variable commands reflect the active document

The shell SHALL register Image > Variables > Define, Image > Variables > Data Sets, File > Import > Variable Data Sets, Image > Apply Data Set, and File > Export > Data Sets As Files. Without an open document all five SHALL be disabled. Define SHALL be available for an editable document. Data Sets and Import SHALL require variables. Apply and Export SHALL require a saved data set. An active document edit that cannot safely be suspended SHALL block these commands.

#### Scenario: Command availability follows the template state [vars_commands]

- **WHEN** there is no open document, then an open document, then a variable, then a stored data set
- **THEN** each command becomes available at the corresponding stage and blocking edit sessions prevent dispatch

#### Scenario: Desktop Open exposes template metadata [vars_desktop_open]

- **WHEN** a supported PSD template is opened through the desktop Open command using the lazy layer loading path
- **THEN** Variables and Data Sets show the same resolved bindings and values as an eager open, and command availability reflects them

### Requirement: The Variables dialog stages definition and set edits

Define and Data Sets SHALL open the matching page of the same dialog. The Define page SHALL identify bound layers, offer compatible kinds, allow sharing an existing compatible variable, and show placement controls for pixel replacement. The Data Sets page SHALL allow creating, navigating, naming, deleting, and editing sets with type-appropriate controls. Its Import action and the File command SHALL expose the same naming, replacement, and encoding options. A changed dialog SHALL commit its staged edits once on OK. Cancel SHALL discard staged definitions, values, and in-dialog imports. An invalid edit SHALL leave the dialog open with an explanation.

#### Scenario: The two pages share staged data [vars_dialog_edit]

- **WHEN** a user defines a variable, switches pages, edits or imports a set and accepts the dialog
- **THEN** the staged variable and set are committed together in one history entry and undo restores the prior template

#### Scenario: Cancelling removes staged changes [vars_dialog_cancel]

- **WHEN** a user previews a staged set, switches pages, and cancels the dialog
- **THEN** the original canvas, variables, set values, modified state, and history are restored

### Requirement: Preview does not commit a data set

The Data Sets page SHALL offer Preview and Image > Apply Data Set SHALL offer a set picker with Preview, Apply, and Cancel. Switching or turning off Preview SHALL discard the previous preview. Preview SHALL render against the same starting document each time. Cancel SHALL restore the document presentation with no history or dirty-state change. Apply SHALL commit the chosen set through the document edit path. Export SHALL open a dialog for destination, naming and set selection and SHALL report progress, skipped sets, failure, or cancellation.

#### Scenario: Preview can switch and cancel without a commit [vars_preview]

- **WHEN** a user previews A then B, where A replaces pixels and B chooses Do Not Replace, and cancels
- **THEN** B previews the original pixels and Cancel restores the original presentation, modified state and history

#### Scenario: Export dialog presents the batch choices [vars_export_dialog]

- **WHEN** the user opens export, selects a subset, then cancels before starting
- **THEN** destination, naming, and set selection are available and no output is written
