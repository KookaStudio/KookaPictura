# history-panel Specification

## Purpose
Labelled history states, jumping to a state, history snapshots, and the History dock and toggle.

## Requirements

### Requirement: Labeled history states
The system SHALL record a label with every captured history state and SHALL list
the states oldest-first with their labels in the History panel, highlighting the
current state. States after the current one (redoable) SHALL be shown as not
current.

#### Scenario: States are listed with labels
- **WHEN** several edits have been made
- **THEN** the panel lists one labeled entry per state in order, with the current state highlighted

#### Scenario: A new edit truncates redo states
- **WHEN** a new edit is made after undoing
- **THEN** the panel no longer lists the previously undone states

### Requirement: Jump to a history state
The system SHALL restore the document and selection to the state selected in the
History panel.

#### Scenario: Jump backwards
- **WHEN** the user selects an earlier state in the panel
- **THEN** the document and selection match that state and it becomes current

#### Scenario: Jump forwards
- **WHEN** the user selects a later state
- **THEN** the document and selection match that state

### Requirement: History snapshots
The system SHALL let the user capture the current state as a named snapshot and
SHALL list snapshots separately from the linear states; selecting a snapshot
SHALL restore it.

#### Scenario: Capture a snapshot
- **WHEN** the user captures a snapshot with a name
- **THEN** it appears in the panel and can be selected later to restore that state

### Requirement: History panel docking and toggle
The system SHALL host the History panel in a registered dock with a stable
`objectName` and SHALL expose a `Window > Panels > History` toggle.

#### Scenario: Toggle the History panel
- **WHEN** the user toggles History from the Window menu
- **THEN** the panel is shown or hidden

### Requirement: Undo remains available in the History flyout
The application SHALL allow Ctrl-Z to invoke the existing Edit > Undo command
while the History panel is hosted in its iconic flyout. A focused editable text
control SHALL retain its native Ctrl-Z undo behavior.

#### Scenario: Undo from the History flyout
- **WHEN** the user has an undoable document edit and presses Ctrl-Z while the History panel's iconic flyout is open
- **THEN** the edit is undone without requiring the user to close the flyout

#### Scenario: Text controls keep their own undo
- **WHEN** the user presses Ctrl-Z while an editable text control has focus
- **THEN** the text control undoes its edit and the document history is unchanged

### Requirement: History Options dialog

The History panel SHALL provide a `History Options…` menu entry that opens a
dialog with five checkboxes: `Automatically Create First Snapshot`,
`Automatically Create New Snapshot When Saving`, `Allow Non-Linear History`,
`Show New Snapshot Dialog By Default`, and `Make Layer Visibility Changes
Undoable`. The dialog SHALL start from the current values and SHALL apply the
accepted values without changing the document or its history.

#### Scenario: Options dialog shows current values [hp_options_dialog]
- **WHEN** the user opens `History Options…`
- **THEN** each checkbox reflects the stored option value

#### Scenario: Options persist across restarts [hp_options_persist]
- **WHEN** the user accepts changed option values and the application restarts
- **THEN** the History Options dialog shows the accepted values

### Requirement: Clear History and Alt purge

The History panel SHALL provide a `Clear History` menu entry enabled when a
document is open. Plain `Clear History` SHALL collapse the undo state list to the
current state and SHALL keep every named snapshot. `Alt`/`Option` `Clear History`
SHALL also drop every named snapshot. Neither form SHALL be undoable, and neither
SHALL change the image or the current state's pixels.

#### Scenario: Clear keeps named snapshots [hp_clear_keep_snapshots]
- **WHEN** a document with several states and a named snapshot is cleared
- **THEN** the state list holds only the current state, the snapshot remains, and the image is unchanged

#### Scenario: Alt clear drops snapshots [hp_clear_purge]
- **WHEN** a document with states and named snapshots is cleared with Alt/Option held
- **THEN** the state list holds only the current state, no named snapshot remains, and undo is unavailable

### Requirement: New Document from a state or snapshot

The History panel SHALL provide a `New Document` menu entry enabled when a
document is open. It SHALL create an untitled document from the selected history
state or named snapshot, carrying that state's layers and pixels. The source
document, its history, and its snapshots SHALL be unchanged. The new document's
history SHALL contain exactly one state labelled `Duplicate State`.

#### Scenario: New document from a history state [hp_new_document_state]
- **WHEN** the user selects a middle history state and chooses `New Document`
- **THEN** a new untitled document shows that state's pixels and its history holds one `Duplicate State` state

#### Scenario: New document from a named snapshot [hp_new_document_snapshot]
- **WHEN** the user selects a named snapshot and chooses `New Document`
- **THEN** a new untitled document shows the snapshot's pixels and its history holds one `Duplicate State` state

#### Scenario: Source document is unchanged [hp_new_document_source]
- **WHEN** a new document is created from a state or snapshot
- **THEN** the source document's current state, selection, history list, and snapshots are unchanged

### Requirement: Snapshot button prompt option

The bottom `Create Snapshot` button SHALL add an automatically named snapshot
without prompting when `Show New Snapshot Dialog By Default` is off, and SHALL
prompt for a name when it is on. The `New Snapshot…` menu entry SHALL always
prompt for a name.

#### Scenario: Automatic snapshot from the button [hp_snapshot_dialog]
- **WHEN** `Show New Snapshot Dialog By Default` is off and the user clicks the Create Snapshot button
- **THEN** a `Snapshot N` row appears without a name dialog
