# Spec Delta

## ADDED Requirements

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
