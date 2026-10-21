# Spec Delta

## ADDED Requirements

### Requirement: Layer menu duplication follows the panel selection

`Layer ▸ Duplicate Layer…` SHALL duplicate every layer selected in the Layers
panel, addressing each layer by its path so a layer nested inside a group is
duplicated in place. The command SHALL be enabled only when the active document
has a non-empty panel selection; with no selection it SHALL be disabled and
SHALL change nothing. Each copy SHALL sit directly above its source in the same
container, and the whole operation SHALL be one undoable `"Duplicate Layer"`
step.

#### Scenario: Duplicate a nested layer from the Layer menu [lpd_nested]

- **WHEN** a layer inside a group is selected and `Layer ▸ Duplicate Layer…` is
  run
- **THEN** a copy named `"<name> copy"` appears directly above the source inside
  the same group and exactly one history state is added

#### Scenario: Duplicate a multi-selection from the Layer menu [lpd_multi]

- **WHEN** two layers are selected and `Layer ▸ Duplicate Layer…` is run
- **THEN** both layers gain a copy directly above themselves and exactly one
  history state is added

#### Scenario: Duplicate is disabled with no selection [lpd_empty]

- **WHEN** the Layers panel has no selected row
- **THEN** `Layer ▸ Duplicate Layer…` is disabled and running it leaves the rows
  and history unchanged
