## ADDED Requirements

### Requirement: Vertical Type Mask tool

The Vertical Type Mask tool SHALL behave as the Horizontal Type Mask tool with
the text set in the Vertical Type tool's columns, and its commit SHALL record
exactly one "Vertical Type Mask" state.

#### Scenario: Adding a column to the selection

- **WHEN** the `tst_type_tools` mask test Shift-clicks with the Vertical Type Mask tool, types "IO", and commits over an existing type selection
- **THEN** one "Vertical Type Mask" state is recorded and the selection grows
