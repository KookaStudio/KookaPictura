## ADDED Requirements

### Requirement: Non-linear history appends

When `Allow Non-Linear History` is on, a successful mutating command made after
the user has stepped back SHALL append a new state at the end of the state list
instead of discarding the states after the cursor. The appended state SHALL be
current, every previously retained state SHALL remain available, and undo/redo
SHALL follow recording order rather than a branch tree. When the option is off,
the existing truncation behavior SHALL remain.

#### Scenario: Non-linear capture keeps later states [hp_nonlinear]
- **WHEN** non-linear history is on, the user steps back two states, and makes a new edit
- **THEN** the new state is appended, the two later states remain in the list, and the new state is current

#### Scenario: Undo follows recording order [hp_nonlinear_undo]
- **WHEN** non-linear history has appended a state after a step back
- **THEN** undo restores the state recorded immediately before the appended state, not the state the edit branched from

### Requirement: History options affect capture

The history engine SHALL honor the application's History Options. When
`Make Layer Visibility Changes Undoable` is off, a layer visibility change SHALL
NOT add a history state; when it is on, it SHALL add one state. When
`Automatically Create First Snapshot` is on, adding a document with no named
snapshot SHALL create a `Snapshot 1` snapshot of its initial state. When
`Automatically Create New Snapshot When Saving` is on, a successful non-copy save
SHALL add a named snapshot of the saved state.

#### Scenario: Visibility is not undoable when off [hp_visibility_off]
- **WHEN** `Make Layer Visibility Changes Undoable` is off and the user toggles a layer's visibility
- **THEN** the image updates and the history state count is unchanged

#### Scenario: Visibility is undoable when on [hp_visibility_on]
- **WHEN** `Make Layer Visibility Changes Undoable` is on and the user toggles a layer's visibility
- **THEN** exactly one history state is added and undo restores the previous visibility

#### Scenario: Initial snapshot on document add [hp_auto_first]
- **WHEN** `Automatically Create First Snapshot` is on and a document is opened or created
- **THEN** the History panel lists a `Snapshot 1` snapshot of the initial state

#### Scenario: Snapshot on save [hp_auto_save]
- **WHEN** `Automatically Create New Snapshot When Saving` is on and a document is saved successfully
- **THEN** the History panel gains a named snapshot of the saved state

### Requirement: Materialize a retained state

The history SHALL be able to materialize any retained state or named snapshot
into a document without moving the cursor or changing the live document.

#### Scenario: Materialize a middle state [hp_materialize]
- **WHEN** a retained middle state is materialized
- **THEN** the result's document and selection equal that state and the live document, cursor, and snapshots are unchanged

### Requirement: Clear versus purge

Plain clear SHALL drop every undo state but the current one and SHALL keep named
snapshots. Purge SHALL drop every undo state but the current one and every named
snapshot. Neither SHALL add an undoable state.

#### Scenario: Plain clear keeps snapshots [hp_engine_clear]
- **WHEN** the engine clears a history that has states and named snapshots
- **THEN** one state remains, every named snapshot remains, and undo is unavailable

#### Scenario: Purge drops snapshots [hp_engine_purge]
- **WHEN** the engine purges a history that has states and named snapshots
- **THEN** one state remains and no named snapshot remains

## MODIFIED Requirements

### Requirement: Snapshot capture on mutating commands

The system SHALL capture a snapshot of the full editing state — the loaded `Document` and the active `Option<Selection>` — before each successful mutating command. The commands that capture SHALL include: `apply_filter`, `add_adjustment`, `remove_layer`, `set_layer_visible` when `Make Layer Visibility Changes Undoable` is on, `resize_image`, `resize_canvas`, `rotate_doc`, `flip_doc`, `select_all`, `deselect`, `magic_wand`, `trim_image`, and `revert`. A command that fails validation or whose engine call errors SHALL NOT capture, leaving the history stack unchanged. A listed command that records a state on a document that already has history SHALL append it to that history and SHALL NOT replace or clear the history, its cursor, or its named snapshots.

#### Scenario: Successful command adds a history state

- **WHEN** any listed command completes successfully
- **THEN** the pre-command document and selection are recoverable via undo and the history depth increased by one

#### Scenario: Failed command leaves history untouched

- **WHEN** a command is rejected (invalid parameters, unknown kind, no document loaded)
- **THEN** the history depth and stack contents are unchanged

#### Scenario: A document-preserving command appends [eh_append_existing]

- **WHEN** a listed command records on a document that already has history
- **THEN** the prior states and named snapshots remain, the new state is current, and undo returns to the pre-command state

### Requirement: Bounded depth of 20 states

The history SHALL retain at most 20 captured states, matching the Photoshop CS6 default. When a capture occurs on a full stack, the oldest state SHALL be dropped. Redo information SHALL be discarded when a new successful mutating command captures after one or more undos unless `Allow Non-Linear History` is on; with it on, the later states SHALL be retained and the new state appended.

#### Scenario: Oldest state falls off the bounded stack

- **WHEN** 21 successful mutating commands have captured snapshots in sequence
- **THEN** the first command's state is no longer recoverable via undo, and `undo()` returns the state before the second command

#### Scenario: New op invalidates redo

- **WHEN** after one or more `undo()` calls a new successful mutating command runs with non-linear history off
- **THEN** `redo()` returns `false` for the previously undone state
