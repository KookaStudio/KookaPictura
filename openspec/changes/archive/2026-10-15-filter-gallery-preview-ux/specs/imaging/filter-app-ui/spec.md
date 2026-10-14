## ADDED Requirements

### Requirement: Parameter slider activates its value box

A filter parameter row that carries both a slider and a value box SHALL focus the
value box when the slider is clicked, so Up/Down (and typing) adjust the same
parameter without a separate click into the box.

#### Scenario: Clicking the slider focuses the box [fa_slider_focus]

- **WHEN** a parameter slider is clicked
- **THEN** its value box takes focus, so a following Up/Down changes the
  parameter
