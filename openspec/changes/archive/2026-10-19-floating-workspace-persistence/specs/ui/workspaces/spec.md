# Spec Delta

## ADDED Requirements

### Requirement: Floating panel state in a workspace arrangement

A workspace's remembered arrangement SHALL include which widget groups and whole
widget columns are floating, each overlay's position and size, and the column
that owns each floating group. Switching away from a workspace and back,
applying a workspace that carries float markers, and loading such a workspace
after a store reload SHALL restore those floats at their remembered positions.
A workspace arrangement without float markers SHALL open with every widget panel
docked. Applying a workspace SHALL continue to leave the Tools column's docked
or floating state and position unchanged.

#### Scenario: Switching away and back remembers floats [wsp_floats_remembered]

- **WHEN** a workspace has a floating group or whole column, another workspace
  is selected, and the first workspace is selected again
- **THEN** the float is restored floating at its remembered position and size

#### Scenario: A saved workspace restores its floats [wsp_floats_roundtrip]

- **WHEN** a user workspace created from a floating arrangement is reloaded from
  the workspace store
- **THEN** its floating groups and columns are restored floating with their
  remembered geometry

#### Scenario: A workspace without float markers docks everything [wsp_floats_legacy]

- **WHEN** a workspace arrangement carries no floating state
- **THEN** every widget panel opens docked, including panels that are floating in
  the current arrangement

#### Scenario: A workspace apply leaves the Tools column alone [wsp_floats_tools]

- **WHEN** a workspace is applied while the Tools column is docked or floating
- **THEN** the Tools column keeps its current docked or floating state and
  position
