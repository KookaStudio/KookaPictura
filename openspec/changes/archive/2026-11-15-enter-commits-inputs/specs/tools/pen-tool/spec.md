# Spec Delta

## ADDED Requirements

### Requirement: Pen Enter waits for an unfocused value input

The Pen tool's Enter path finish SHALL run only when no keyboard-editable value
input has focus. While a value input has focus, Enter SHALL belong to the input
(see `ui/value-input-enter`); Esc, Ctrl-click, and switching tools SHALL still
end the subpath regardless of focus.

#### Scenario: A focused value field keeps the path open [pen_path_enter_field_focus]

- **WHEN** a subpath is being drawn, a value input has focus, and Enter is
  pressed
- **THEN** the field commits and the subpath stays open; a later Enter with no
  value input focused ends it open
