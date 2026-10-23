# Spec Delta

## Purpose

Quick Selection: a stroke-accumulated selection that records one history state
and switches New to Add.

## ADDED Requirements

### Requirement: Quick Selection stroke

A Quick Selection press SHALL begin one stroke, and each pointer move SHALL
grow the selection from the current stroke. A New-mode stroke SHALL replace the
previous selection with its first region and SHALL grow the stroke's accumulated
selection with later regions. Releasing SHALL record exactly one "Selection"
history state for the whole stroke; a stroke that selects nothing SHALL record
none. Cancelling the stroke before release SHALL restore the selection captured
at press and record no state. After a New-mode stroke's first selection the
tool's combine mode SHALL switch from New to Add. Alt SHALL subtract while a
selection exists at press, following the shared selection-modifier rules.

#### Scenario: A drag across two regions selects their union [qs_union_one_state]

- **WHEN** a Quick Selection drag crosses two disjoint regions
- **THEN** both regions are selected and the history gains exactly one
  "Selection" state

#### Scenario: New switches to Add after the first selection [qs_new_to_add]

- **WHEN** the first Quick Selection stroke completes in New mode
- **THEN** the tool's combine mode reads Add, and the next stroke adds to the
  existing selection instead of replacing it

#### Scenario: Cancelling a stroke restores the pre-stroke selection [qs_cancel_restores]

- **WHEN** a Quick Selection drag is cancelled before release
- **THEN** the selection and its history are exactly as they were before the
  press
