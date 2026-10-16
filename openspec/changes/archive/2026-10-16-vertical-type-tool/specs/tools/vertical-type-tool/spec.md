## ADDED Requirements

### Requirement: Vertical Type tool

The Vertical Type tool SHALL open the same point-type session as the
Horizontal Type tool, but SHALL set each line as a column read top to bottom,
the first column centred on the click and later columns to its left, aligned
top, center, or bottom at the click. Its commit SHALL add one type layer whose
`TySh` orientation is vertical and SHALL record exactly one "Vertical Type"
state.

#### Scenario: A column of type

- **WHEN** the `tst_type_tools` vertical test clicks at (100, 10), types "TALL", and presses keypad Enter
- **THEN** one "Vertical Type" state is recorded and the layer starts at y 10, straddles x 100, and is more than three times taller than wide

#### Scenario: Columns run leftward

- **WHEN** `type_layer::tests::vertical_type_is_taller_than_wide_and_columns_run_leftward` places "AB\rCD" vertically
- **THEN** its right edge matches the one-column placement and its left edge lies further left
