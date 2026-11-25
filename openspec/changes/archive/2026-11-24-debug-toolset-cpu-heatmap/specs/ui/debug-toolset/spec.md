# Spec Delta

## ADDED Requirements

### Requirement: Per-CPU usage is a heat matrix

The Platform group's `CPUs` row SHALL show one square per logical CPU, coloured from
cold to hot by that CPU's usage, with no numbers. The squares SHALL form one row for
8 or fewer CPUs and two rows for more. Before the second sample the row SHALL read
`…`, and it SHALL be emptied when the toolset turns off.

#### Scenario: Colour runs from cold to hot [dbg_cpu_heat_colours]

- **WHEN** a heat matrix is given loads of 0 %, 50 % and 100 %
- **THEN** the 0 % square is blue, the 100 % square red, and the 50 % square differs
  from both

#### Scenario: One or two rows by CPU count [dbg_cpu_heat_rows]

- **WHEN** a heat matrix is given 4, 8, 9 and 16 loads
- **THEN** it has 1, 1, 2 and 2 rows, and every load has a square inside the widget

#### Scenario: The panel feeds the matrix from the sampler [dbg_cpu_heat_panel]

- **WHEN** the panel has taken one sample, then a second
- **THEN** the matrix is empty after the first and holds one load per CPU in
  `/proc/stat` after the second, and is empty again after the toolset is turned off
