# Spec Delta

## ADDED Requirements

### Requirement: Perspective Crop Enter waits for an unfocused value input

The Perspective Crop tool's Enter commit SHALL run only when no
keyboard-editable value input has focus. While a value input has focus, Enter
SHALL belong to the input (see `ui/value-input-enter`), and a double-click
inside the quad SHALL still commit regardless of focus.

#### Scenario: A focused value field keeps the quad staged [perspective_crop_enter_field_focus]

- **WHEN** a complete quad is staged, a value input has focus, and Enter is
  pressed
- **THEN** the field commits and the quad stays staged; a later Enter with no
  value input focused commits it
