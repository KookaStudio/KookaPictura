# Spec Delta

## ADDED Requirements

### Requirement: The field gate follows the newly focused widget

The Delete and Backspace shortcuts SHALL be disabled exactly while an editable
text field holds focus, judged by the widget that receives focus: moving focus
into a field SHALL disable them, and moving focus out of a field to a widget
that is not a field SHALL enable them again.

#### Scenario: Leaving a field gives Delete back to the workspace [dk_after_field]

- **WHEN** a text field has focus, focus then moves to a widget that is not a
  field, a pixel layer is selected, and Delete is pressed
- **THEN** the shortcuts were disabled while the field had focus and enabled
  after it lost focus, and the layer is deleted with one undo state
