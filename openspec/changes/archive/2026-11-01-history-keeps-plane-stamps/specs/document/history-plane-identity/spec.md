# Spec Delta

## ADDED Requirements

### Requirement: A capture preserves the identity of unchanged planes

A history capture or restore SHALL leave the stamp of an unchanged plane as it
was, and SHALL give a written plane a new stamp shared by the live document and
history's current state. Stamps SHALL stay unique per plane within a document.

#### Scenario: Unchanged planes keep their stamps [hpi_keeps]

- **WHEN** one channel of a layer is written and a state is captured
- **THEN** every other plane keeps its stamp and the written plane has a new nonzero one

#### Scenario: Undo keeps unchanged stamps [hpi_undo]

- **WHEN** an edit to one channel is undone
- **THEN** planes the edit did not write keep their stamps

#### Scenario: A duplicated layer's planes stay unique [hpi_unique]

- **WHEN** a layer is cloned into the stack and a state is captured
- **THEN** no two planes of the document carry the same nonzero stamp
- **AND** a further capture keeps all of them
