# Spec Delta

## ADDED Requirements

### Requirement: The panel is titled Pictura Metrics

The panel's header SHALL read `Pictura Metrics`.

#### Scenario: The header shows the product title [dbg_title]

- **WHEN** the toolset is on
- **THEN** the panel's title label reads `Pictura Metrics`

### Requirement: Tables are laid out in two columns

The panel SHALL place Pictura Resource Usage and Version in a left column, and Pictura
Memory Budget, Pictura Activity and Platform in a right column to its right, so the
panel is wider than the sum of its parts is tall.

#### Scenario: Budget, activity and platform sit right of usage [dbg_two_columns]

- **WHEN** the panel is shown
- **THEN** the Memory Budget, Activity and Platform tables start at or beyond the right
  edge of the Resource Usage table, the Version table is in the left column, and the
  panel is shorter than the stacked tables would be
