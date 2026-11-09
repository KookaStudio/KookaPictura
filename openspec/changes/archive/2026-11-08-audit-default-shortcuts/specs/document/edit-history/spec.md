## MODIFIED Requirements

### Requirement: App controls and accessors

The system SHALL expose `can_undo() -> bool`, `can_redo() -> bool`, and `history_depth() -> i32` on `PictureView`, and the Qt shell SHALL provide Undo and Redo controls wired to `undo()`/`redo()`, disabled when the corresponding availability accessor is `false`, refreshing after every state change. The Undo control SHALL carry `Ctrl+Z` and SHALL perform `redo()` when `can_undo()` is `false` and `can_redo()` is `true`, matching CS6's Undo/Redo toggle. The Redo control SHALL carry no default binding, because CS6 gives `Ctrl+Shift+Z` to Step Forward and `Ctrl+Y` to Proof Colors.

#### Scenario: Controls track availability

- **WHEN** the stack is empty, after a captured op, and after an undo
- **THEN** the Undo control is disabled, enabled, and enabled-with-Redo-enabled respectively

#### Scenario: Undo toggles to redo [eh_undo_toggles]

- **WHEN** no undo state remains and one redo state exists, and `Ctrl+Z` is pressed
- **THEN** the redo state is restored
