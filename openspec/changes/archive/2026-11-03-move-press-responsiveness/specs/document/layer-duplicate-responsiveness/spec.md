## MODIFIED Requirements

### Requirement: Undo does not rebuild the Move preview

Restoring a history state SHALL NOT build the Move tool's preview. Switching to
the Move tool SHALL NOT build it either; a press SHALL build it.

#### Scenario: Undo leaves the warm alone [ldr_undo_no_warm]

- **WHEN** the Move tool is active on a movable layer and an edit is undone
- **THEN** no preview is built, and the next press builds it
