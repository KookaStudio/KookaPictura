# Spec Delta

## ADDED Requirements

### Requirement: Crop Enter waits for an unfocused value input

The Crop tool's Enter commit SHALL run only when no keyboard-editable value
input has focus. While a value input has focus, Enter SHALL belong to the input
(see `ui/value-input-enter`), and a double-click inside the box or the options
bar's Apply SHALL still commit regardless of focus.

#### Scenario: A focused value field keeps the crop pending [crop_enter_field_focus]

- **WHEN** the Crop tool has a pending box, a tool-options value field has focus,
  and Enter is pressed
- **THEN** the field commits and the crop box stays pending; a later Enter with
  no value input focused crops the document
