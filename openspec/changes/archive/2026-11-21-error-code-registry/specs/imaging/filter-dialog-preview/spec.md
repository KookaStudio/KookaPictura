# Filter and Adjustment Commit Operation Status

## ADDED Requirements

### Requirement: A failed staged filter or adjustment commit preserves undo

The shared filter and adjustment commit worker SHALL deliver a structured status to Qt on completion. An unwind during engine work, compositing, or worker history capture SHALL report `OP_ERR_PANIC`. The failed commit SHALL restore the pre-preview canvas, preserve existing undo and redo states, the current history index and named snapshots, and leave the committed document and dirty state as before the commit attempt. The next successful commit SHALL apply normally and record one state. A failed adjustment dialog OK commit SHALL present operation-specific feedback based on its status without exposing a panic payload.

#### Scenario: Panicking filter and adjustment commits preserve history [os_filter_panic]

- **WHEN** a filter commit panics in staged history capture, and an adjustment commit panics in staged engine work, with an undo/redo branch and a named snapshot present
- **THEN** each reports `OP_ERR_PANIC`, displays the original canvas, and preserves the history position, undo, redo and named snapshot, and a later successful commit records exactly one state

#### Scenario: A rejected filter commit reports its expected failure [os_filter_refused]

- **WHEN** a filter is applied to a pixel-locked layer after a prior successful filter commit
- **THEN** Qt receives `OP_ERR_INVALID_ARG` with the application reason, the busy state clears, and the document, dirty state, and history are unchanged by the refusal

#### Scenario: An adjustment dialog reports a failed OK commit [os_adjustment_dialog_failure]

- **WHEN** the user accepts an adjustment dialog and the shared commit worker panics
- **THEN** the user receives operation-specific failure feedback based on `OP_ERR_PANIC` without the panic payload and the pre-commit document and history remain intact
