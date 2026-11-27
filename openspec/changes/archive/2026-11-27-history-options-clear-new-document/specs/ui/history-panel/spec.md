## ADDED Requirements

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
