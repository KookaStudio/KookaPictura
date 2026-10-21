# Spec Delta

## ADDED Requirements

### Requirement: Hidden layers are not movable or transformable

A hidden layer SHALL NOT be moved or transformed. While the active layer is
hidden, the Move tool's `Show Transform Controls` box SHALL be hidden, an
in-canvas Move drag SHALL leave the layer's rect and the history unchanged, and
`Free Transform`, `Skew`, `Distort`, `Perspective`, and `Warp` SHALL be disabled
and refused. A selection SHALL NOT lift a hidden layer's pixels, and an Alt-drag
SHALL NOT duplicate it. Toggling the layer's visibility SHALL remain available.

#### Scenario: A hidden layer shows no controls box [lhe_box]

- **WHEN** the active layer is hidden while the Move tool's controls box is shown
- **THEN** the box disappears and no transform session runs

#### Scenario: The Move tool does not move a hidden layer [lhe_move]

- **WHEN** a Move drag starts on a hidden layer
- **THEN** the layer's rect is unchanged, no move preview starts, and no history
  state is added

#### Scenario: Transform commands refuse a hidden layer [lhe_transform]

- **WHEN** `Free Transform`, a transform mode, or `Warp` is invoked on a hidden
  layer, with or without a selection
- **THEN** the command is disabled or refused, no transform session starts, and
  the document is unchanged
