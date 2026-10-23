# Spec Delta — document/edit-history

## MODIFIED Requirements

### Requirement: Snapshot capture on mutating commands

The system SHALL capture a snapshot of the full editing state — the loaded `Document` and the active `Option<Selection>` — before each successful mutating command. The commands that capture SHALL include: `apply_filter`, `add_adjustment`, `remove_layer`, `set_layer_visible`, `resize_image`, `resize_canvas`, `rotate_doc`, `flip_doc`, `select_all`, `deselect`, `magic_wand`, `trim_image`, and `revert`. A command that fails validation or whose engine call errors SHALL NOT capture, leaving the history stack unchanged. A listed command that records a state on a document that already has history SHALL append it to that history and SHALL NOT replace or clear the history, its cursor, or its named snapshots.

#### Scenario: Successful command adds a history state

- **WHEN** any listed command completes successfully
- **THEN** the pre-command document and selection are recoverable via undo and the history depth increased by one

#### Scenario: Failed command leaves history untouched

- **WHEN** a command is rejected (invalid parameters, unknown kind, no document loaded)
- **THEN** the history depth and stack contents are unchanged

#### Scenario: A document-preserving command appends [eh_append_existing]

- **WHEN** a listed command records on a document that already has history
- **THEN** the prior states and named snapshots remain, the new state is current, and undo returns to the pre-command state

## ADDED Requirements

### Requirement: Restoring a named snapshot records a history state

Restoring named snapshot `i` SHALL install that snapshot's document and selection, restore the layer target captured with it, and append exactly one current history state labelled with the snapshot's label. The pre-restore document SHALL remain the previous state, so `undo()` SHALL return to the pre-restore document and `redo()` SHALL return to the snapshot. The new state SHALL follow the normal history rules: the cursor SHALL move to it, a later capture after undo SHALL discard redo, and the 20-state bound SHALL apply. Restoring a snapshot SHALL cancel a live session before installing the snapshot.

#### Scenario: Undo returns to the pre-restore state [eh_snapshot_undo]

- **WHEN** a named snapshot is restored while the document has a later state
- **THEN** exactly one state is appended, the history cursor is the appended state, and undo restores the pre-restore document and selection

#### Scenario: Redo returns to the snapshot [eh_snapshot_redo]

- **WHEN** the undo of a snapshot restore succeeds and redo is called
- **THEN** the document, selection, and layer target equal the restored snapshot

#### Scenario: Restoring after undo discards redo [eh_snapshot_after_undo]

- **WHEN** a document is edited past a snapshot, undone once, and the snapshot is restored
- **THEN** exactly one state is appended after the undone states are discarded, undo returns to the state that was current before the restore, and redo returns to the snapshot

### Requirement: Trim records one state on the existing history

`Image > Trim` SHALL crop the live document in place and record exactly one
history state labelled `Trim` on the existing history. It SHALL refuse without
mutating while a live edit session or a pending opacity, fill, or mask-refine
preview is active. When the visible content already fills the canvas or the
composite is empty, Trim SHALL record nothing and leave the document, selection,
history, and dirty state unchanged.
 It SHALL keep the
active layer target when it still resolves and otherwise select the top layer.
On success it SHALL clear the selection because its coordinates no longer map.
It SHALL NOT replace the document, reset the history, or discard named
snapshots. Undo SHALL restore the pre-Trim document, selection, and history
depth.

#### Scenario: Trim is undoable [eh_trim_undoable]

- **WHEN** a document with prior edits, a named snapshot, an active layer target, and a selection is trimmed and undo is called
- **THEN** the pre-Trim document, selection, history depth, and layer target are restored, the prior states and snapshot remain, and the Trim state is no longer current

#### Scenario: A trim with nothing to remove records nothing [eh_trim_noop]

- **WHEN** Trim runs on a document whose visible content already fills the canvas, or on an empty composite
- **THEN** no history state is added and the document, selection, dirty state, and layer target are unchanged
