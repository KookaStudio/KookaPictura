# Spec Delta

## MODIFIED Requirements

### Requirement: Edit Contents eligibility and refusal

The application SHALL enable Edit Contents for a non-group, non-adjustment
layer with an embedded PSD/PSB source that parses or a supported embedded PDF
or PDF-compatible AI source. A read-only eligibility check SHALL reject every
other target and SHALL NOT mutate the document. A refused dispatch SHALL
record no history state.

#### Scenario: A PSD/PSB source is editable

- **WHEN** `layer_can_edit_smart_object_contents` is called on a non-group, non-adjustment layer whose `Embedded` smart object has a payload that parses as a PSD/PSB document
- **THEN** it returns `true`

#### Scenario: A non-parsing source is refused

- **WHEN** the target layer carries a non-empty payload of an unsupported format such as a placed JPEG
- **THEN** the predicate returns `false` and the command is disabled

#### Scenario: A non-embedded object is refused

- **WHEN** the target layer's smart object is not of kind `Embedded` (for example an external, alias, or unresolved record)
- **THEN** the predicate returns `false` and the command is disabled

#### Scenario: A non-smart layer is refused

- **WHEN** the target layer has no smart object
- **THEN** the predicate returns `false`

#### Scenario: A group or adjustment layer is refused

- **WHEN** the target path resolves to a group or a layer with adjustment data
- **THEN** the predicate returns `false`

#### Scenario: An empty payload is refused

- **WHEN** the target layer's smart object has no payload or an empty payload
- **THEN** the predicate returns `false`

#### Scenario: An unresolved path is refused

- **WHEN** `path` does not resolve to a layer
- **THEN** the predicate returns `false`

#### Scenario: The predicate mutates nothing

- **WHEN** the predicate is called on any target and the document is compared with a pre-call clone
- **THEN** the two documents are equal

### Requirement: The Edit Contents command opens the source as a new untitled editor document

The application SHALL expose `Layer > Smart Objects > Edit Contents` with the
stable id `LayerSmartObjectEditContents` (`"layer.smartObject.editContents"`).
The command SHALL be enabled according to Edit Contents eligibility. When
dispatched on an editable PSD/PSB source it SHALL export the target's embedded payload byte-for-byte to a
per-session temporary file, load that file into a new document as an ordinary
document (its own layers and composite), and add it through
`addDocument(editor, QString())` so a new untitled tab opens. The editor document
SHALL be the decoded source, NOT a document wrapping the source as a smart-object
layer, and the editor tab SHALL be untitled regardless of the temporary file's
path.

#### Scenario: The command is registered with its stable id

- **WHEN** the command registry is built
- **THEN** `LayerSmartObjectEditContents` is registered as an implemented `Layer > Smart Objects > Edit Contents` command

#### Scenario: Enabled only for an editable source

- **WHEN** the current layer is an editable smart object
- **THEN** the command is enabled, and it is disabled when the current layer is a non-smart layer, a group, an adjustment layer, a non-`Embedded` object, an object with an empty payload, or an unsupported payload

#### Scenario: One new untitled editor tab opens

- **WHEN** the command runs on an editable PSD/PSB smart-object layer
- **THEN** exactly one tab is added, its document path is empty, and it holds the decoded source content

#### Scenario: Opening records no state on the origin

- **WHEN** the command opens an internal PSD/PSB editor
- **THEN** the origin document is unchanged and its history count is unchanged

#### Scenario: An ineligible target adds no tab

- **WHEN** the command is dispatched while the current layer is not editable
- **THEN** no tab is added, no history state is recorded, and no origin state changes

## ADDED Requirements

### Requirement: Vector Edit Contents opens an external session

For an editable embedded PDF or PDF-compatible AI, Edit Contents SHALL export
the source to a private temporary file with its format-appropriate extension,
launch the desktop-associated application for that file, and offer Apply and
Discard. Opening the session SHALL add no origin history state. Failure to
launch SHALL report an error and clean up the session files.

#### Scenario: Launch the desktop-associated editor [vector_edit_launch]
- **WHEN** Edit Contents runs on an eligible vector Smart Object
- **THEN** the desktop handoff receives a temporary copy with the exact source bytes and correct extension, Apply and Discard are available, and no editor tab or origin history state is added

#### Scenario: The desktop handler cannot launch [vector_edit_handler]
- **WHEN** the desktop handoff refuses the file
- **THEN** the command reports failure, removes the temporary copy, and records no history

### Requirement: Apply a vector edit to shared instances

Apply SHALL validate and render the temporary source before committing. For a
changed source it SHALL update every instance sharing that embedded source in
the originating document, preserving per-layer transforms, masks, effects,
and Smart Filters. Independent sources SHALL retain their content. Apply
SHALL record one Edit Contents state and Undo and Redo SHALL restore the
matching bytes and preview for every affected layer. An unchanged file, an
invalid or missing page, a discarded edit, or a stale source SHALL record no
state. Failed and stale Apply operations SHALL report the error. Discard and
origin close SHALL release the temporary file.

#### Scenario: Apply after layer reordering [vector_edit_apply]
- **WHEN** the editor saves valid changed artwork and Apply runs after layers have been reordered
- **THEN** the shared source and all its instances update together without affecting an independent source, and Undo and Redo restore the bytes and previews

#### Scenario: Unchanged or failed Apply [vector_edit_refused]
- **WHEN** Apply reads unchanged bytes, invalid bytes, a source without the selected page, or encounters an origin source that was replaced
- **THEN** no history state or artwork change is committed

#### Scenario: Discard or close a vector session [vector_edit_discard]
- **WHEN** the user discards the external edit or closes the originating document
- **THEN** no edit is committed and the session files are removed
