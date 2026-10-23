# Spec Delta — document/document-lifecycle

## MODIFIED Requirements

### Requirement: Modified state tracking
The system SHALL expose `is_dirty()` for the active document. Every successful
mutating command SHALL mark the document modified. Saving, opening, New, and a
successful Revert SHALL establish the document as unmodified at the current
history state. Undo, redo, history jump, and named-snapshot restore SHALL mark
the document modified unless the resulting history state is the state recorded
as saved. Returning to the saved state SHALL clear the modified state. When the
saved state is no longer retained by the bounded history, the document SHALL
report modified. A rejected command SHALL NOT change the modified state. A view
with no document SHALL report unmodified.

#### Scenario: A mutating command marks modified
- **WHEN** a filter, adjustment, document op, or selection command succeeds
- **THEN** the document reports modified

#### Scenario: Save clears modified
- **WHEN** a modified document is saved successfully
- **THEN** the document reports unmodified

#### Scenario: Open clears modified
- **WHEN** a document is opened
- **THEN** it reports unmodified

#### Scenario: History navigation marks modified [ml_history_navigation]

- **WHEN** a saved document is edited and then navigated by undo, redo, history jump, or named-snapshot restore to a state other than the saved state
- **THEN** the document reports modified

#### Scenario: Returning to the saved state clears modified [ml_return_to_saved]

- **WHEN** a saved document is edited and history navigation returns to the saved state
- **THEN** the document reports unmodified

#### Scenario: A dropped saved state stays modified [ml_saved_state_dropped]

- **WHEN** enough new edits or bounded-history drops remove the saved state from the retained history
- **THEN** the document reports modified and no navigation can return it to unmodified

### Requirement: Revert
The system SHALL revert a saved document by reloading it from its path into the
existing document identity. Revert SHALL decode the file before changing the
document; if the file cannot be read or decoded, including when the stored PDF
password no longer opens it, Revert SHALL return failure and leave the document,
pixels, selection, dirty state, path, history, and any live session unchanged.
On success, Revert SHALL cancel the live session, clear the outgoing selection
and the rest of the document-scoped state, install the reloaded document,
preserve the existing history and its named snapshots, append exactly one
current history state labelled `Revert`, and mark the document unmodified. Undo
SHALL restore the pre-Revert document. Revert SHALL be unavailable for an
untitled document.

#### Scenario: Revert reloads from disk
- **WHEN** the user reverts a modified document with a path
- **THEN** the document matches the file on disk and reports unmodified

#### Scenario: Revert unavailable when untitled
- **WHEN** the active document has no path
- **THEN** Revert is disabled

#### Scenario: A successful Revert is undoable [dl_revert_undoable]

- **WHEN** a modified document with prior history and named snapshots is reverted from an intact file
- **THEN** the history retains the pre-Revert states and named snapshots and appends one current `Revert` state; undo then restores the pre-Revert pixels

#### Scenario: A failed Revert keeps the document [dl_revert_failure_keeps]

- **WHEN** Revert runs and the file is missing, truncated, cannot be decoded, or its stored PDF password no longer opens it
- **THEN** Revert returns failure and the document, pixels, selection, dirty state, path, history, and any live session are unchanged
