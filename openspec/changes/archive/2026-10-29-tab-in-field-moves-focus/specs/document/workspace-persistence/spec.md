# Spec Delta

## ADDED Requirements

### Requirement: Tab in a text field moves focus

While an editable text widget in the frame has keyboard focus (a line edit, a
spin box or numeric field, a text edit, or an editable combo box), `Tab` and
`Shift+Tab` SHALL NOT show or hide the panels in any screen mode. The key SHALL
reach the focused widget instead: a single-line field SHALL commit its typed
value and move focus to the next field (`Tab`) or the previous one
(`Shift+Tab`). When no text widget has focus, `Tab` and `Shift+Tab` SHALL show
and hide the panels as before.

#### Scenario: Tab in a Properties field commits and moves focus [panels_tab_field_focus]

- **WHEN** a value is typed into a Properties panel spin box and `Tab` is pressed
- **THEN** the value is applied, focus moves to the next field, and the panels
  stay visible; `Shift+Tab` then moves focus back without hiding the panels

#### Scenario: Tab outside a text field still hides the panels [panels_tab_canvas_toggle]

- **WHEN** the canvas has focus and `Tab` is pressed
- **THEN** the panels are hidden, and pressing `Tab` again restores them
