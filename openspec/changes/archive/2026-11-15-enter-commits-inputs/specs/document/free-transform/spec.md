# Spec Delta

## ADDED Requirements

### Requirement: Free Transform Enter waits for an unfocused value input

The Free Transform session's Enter commit SHALL run only when no
keyboard-editable value input has focus. While a value input has focus, Enter
SHALL belong to the input (see `ui/value-input-enter`), and the options bar's
Apply control SHALL still commit regardless of focus.

#### Scenario: A focused value field keeps the session active [transform_enter_field_focus]

- **WHEN** a session is active, a tool-options value field has focus, and Enter
  is pressed
- **THEN** the field commits and the session stays active; a later Enter with no
  value input focused commits the transform
