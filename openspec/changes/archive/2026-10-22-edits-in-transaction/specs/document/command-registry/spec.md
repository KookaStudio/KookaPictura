# Spec Delta

## ADDED Requirements

### Requirement: Command state is recomputed once per notifying committed edit

The system SHALL recompute command enablement, checked state and labels exactly once from the transaction's notification for each committed document edit whose outcome notifies. A command that commits such an edit SHALL NOT cause the command table to be recomputed a second time through its handler for the same edit. Menu-open and panel-driven recomputations are separate triggers and are not counted as a second presentation of the edit. The recomputation SHALL observe the history state of the edit it follows, so the undo and redo commands reflect the edit that has just been committed without a further refresh.

#### Scenario: A committed edit recomputes the command table once [cr_one_refresh]

- **WHEN** a menu command commits a notifying document edit
- **THEN** the command table's enablement, checked state and labels are recomputed once from the edit's notification, and the handler adds no second recomputation

#### Scenario: The recomputation sees the committed history state [cr_history_current]

- **WHEN** a notifying document edit is committed and the command table is recomputed
- **THEN** the undo command is enabled and its label names the edit that was just committed, without a further refresh
