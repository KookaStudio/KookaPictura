# Spec Delta

## MODIFIED Requirements

### Requirement: Quick Selection stroke

A Quick Selection press SHALL begin one stroke, and each pointer move SHALL grow
the selection from the current stroke. An Add-mode move wholly inside a region
the stroke has already selected SHALL leave the selection unchanged. A New-mode
stroke SHALL replace the previous selection with its first region and SHALL grow
the stroke's accumulated selection with later regions. Releasing SHALL record
exactly one "Selection" history state for the whole stroke; a stroke that selects
nothing SHALL record none. Cancelling the stroke before release SHALL restore the
selection captured at press and record no state. After a New-mode stroke's first
selection the tool's combine mode SHALL switch from New to Add. That mode SHALL
belong to Quick Selection alone: the switch SHALL NOT change another selection
tool's combine mode. Alt SHALL subtract while a selection exists at press,
following the shared selection-modifier rules.

#### Scenario: A drag across two regions selects their union [qs_union_one_state]

- **WHEN** a Quick Selection drag crosses two disjoint regions
- **THEN** both regions are selected and the history gains exactly one
  "Selection" state

#### Scenario: New switches to Add after the first selection [qs_new_to_add]

- **WHEN** the first Quick Selection stroke completes in New mode
- **THEN** the tool's combine mode reads Add, and the next stroke adds to the
  existing selection instead of replacing it

#### Scenario: A move inside the selected region changes nothing [qs_move_inside_noop]

- **WHEN** an Add-mode Quick Selection pointer move stays inside the region the
  current stroke has already selected
- **THEN** the selection and its history are unchanged

#### Scenario: Quick Selection does not change another tool's mode [qs_mode_isolation]

- **WHEN** a New-mode Quick Selection stroke selects a region and the user then
  switches to the Rectangular Marquee with New still selected
- **THEN** a new marquee drag replaces the existing selection instead of adding
  to it, and the marquee's combine mode still reads New

#### Scenario: Cancelling a stroke restores the pre-stroke selection [qs_cancel_restores]

- **WHEN** a Quick Selection drag is cancelled before release
- **THEN** the selection and its history are exactly as they were before the
  press
